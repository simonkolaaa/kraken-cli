pub mod state;
pub mod strategy;
pub mod news;
pub mod llm;
pub mod dashboard;
pub mod telegram;
pub mod screener;
pub mod indicators;

use std::time::{Duration, Instant};
use tokio::time;
use tracing::{error, info, warn};

use crate::client::SpotClient;
use crate::paper::OrderSide;
use state::BotState;
use strategy::{MarketContext, LlmSentimentStrategy, Signal, TradingStrategy};
use llm::OpenRouterClient;
use news::NewsFetcher;
use telegram::TelegramNotifier;

const MAX_CONSECUTIVE_ERRORS: u32 = 3;

pub async fn run_bot_loop(
    client: &SpotClient,
    watchlist: Vec<String>,
    interval_minutes: u64,
) -> crate::errors::Result<()> {
    info!(
        "Starting autonomous sentiment bot. Watchlist: {:?}, interval: {}m",
        watchlist, interval_minutes
    );

    let state = BotState::new()?;
    let dashboard_state = state.paper_state.clone();
    
    // Spawn dashboard
    tokio::spawn(async move {
        dashboard::start_dashboard(dashboard_state).await;
    });

    let telegram = TelegramNotifier::new();

    let start_msg = format!("🤖 <b>Bot avviato con successo.</b>\nWatchlist: {:?}\nSaldo iniziale: {:.2} USD", watchlist, state.get_balance("USD").await);
    if let Some(tg) = &telegram {
        tg.send_message(&start_msg).await;
    }

    let mut consecutive_errors = 0;

    let llm_client = OpenRouterClient::new()?;
    let news_fetcher = NewsFetcher::new()?;
    let mut strategy = LlmSentimentStrategy::new(llm_client, 55);

    let mut ticker = time::interval(Duration::from_secs(interval_minutes * 60));

    loop {
        ticker.tick().await;

        let current_base_assets = match screener::get_global_usd_assets(50).await {
            Ok(assets) => assets,
            Err(e) => {
                error!("Screener API failed: {}. Retrying next cycle.", e);
                continue;
            }
        };

        info!("--- Bot Loop Iteration ---");

        // 1. Fetch News
        let general_news = match news_fetcher.fetch_news_for_asset("MARKET").await {
            Ok(n) => {
                consecutive_errors = 0;
                n
            }
            Err(e) => {
                warn!("Failed to fetch news: {}. Will use fallback.", e);
                consecutive_errors += 1;
                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    let kill_msg = "🚨 <b>ATTENZIONE: Kill Switch Attivato!</b> Bot fermato a causa di errori continui sulle news.";
                    if let Some(tg) = &telegram { tg.send_message(kill_msg).await; }
                    error!("FATAL: Reached {} consecutive errors. Kill Switch activated.", MAX_CONSECUTIVE_ERRORS);
                    break;
                }
                vec!["Market shows general volatility.".to_string()]
            }
        };

        // 2. Iterate over dynamic watchlist
        let mut evaluated_count = 0;
        for base_asset in &current_base_assets {
            if evaluated_count >= 10 {
                info!("Reached maximum evaluated candidates for this cycle (10).");
                break;
            }

            let pair = format!("{}USD", base_asset);
            info!("Evaluating technicals for asset: {} (Pair: {})", base_asset, pair);
            
            let params = vec![("pair", pair.as_str()), ("interval", "15")];
            let ohlc_data = match client.public_get("OHLC", &params, false).await {
                Ok(data) => {
                    consecutive_errors = 0;
                    data
                }
                Err(e) => {
                    error!("Failed to fetch OHLC data for {}: {}", pair, e);
                    consecutive_errors += 1;
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        let kill_msg = "🚨 <b>ATTENZIONE: Kill Switch Attivato!</b> Bot fermato a causa di errori continui su OHLC.";
                        if let Some(tg) = &telegram { tg.send_message(kill_msg).await; }
                        error!("FATAL: Reached {} consecutive errors. Kill Switch activated.", MAX_CONSECUTIVE_ERRORS);
                        return Ok(());
                    }
                    continue;
                }
            };

            let closes = extract_closes(&ohlc_data, &pair);
            let latest_close = closes.last().copied().unwrap_or(0.0);
            if latest_close == 0.0 {
                warn!("Could not extract latest price for {}. Skipping.", pair);
                continue;
            }

            let mut current_rsi = None;
            let mut current_sma = None;

            if closes.len() >= 50 {
                let latest_50_closes = &closes[closes.len() - 50..];
                current_rsi = indicators::calculate_rsi(latest_50_closes, 14);
                current_sma = indicators::calculate_sma(latest_50_closes, 20);
                state.update_indicators(current_rsi, current_sma).await;
            }

            // Validation Logic based on RSI before AI
            if let Some(rsi) = current_rsi {
                if rsi > 70.0 || rsi < 30.0 {
                    info!("RSI for {} is {:.2} (extreme). Skipping AI analysis.", pair, rsi);
                    continue;
                }
            } else {
                warn!("Could not calculate RSI for {}. Skipping.", pair);
                continue;
            }

            // Strong Candidate found!
            evaluated_count += 1;
            let rsi_val = current_rsi.unwrap();
            let msg = format!("🎯 <b>Nuovo Candidato Forte:</b> {} (RSI: {:.2}).\n<i>Analizzo le News e valuto con LLM...</i>", pair, rsi_val);
            if let Some(tg) = &telegram {
                tg.send_message(&msg).await;
            }

            let usd_balance = state.get_balance("USD").await;
            let asset_balance = state.get_balance(base_asset).await;

            let pair_context = MarketContext {
                ohlc_data: Some(&ohlc_data),
                news: &general_news,
                usd_balance,
                asset_balance,
            };

            let signal = strategy.evaluate(&pair_context, &pair).await;

            match signal {
                Signal::Buy => {
                    let balance = state.get_balance("USD").await;
                    if balance > 0.0 {
                        // Allocate 20% of quote asset for multi-asset strategy
                        let amount_to_spend = balance * 0.2; 
                        let volume = amount_to_spend / latest_close;
                        
                        if let Err(e) = state.execute_trade(OrderSide::Buy, &pair, volume, latest_close).await {
                            warn!("Could not execute BUY: {}", e);
                        } else {
                            if let Some(tg) = &telegram {
                                tg.send_message(&format!("✅ <b>Eseguito BUY</b> di {:.4} {} a {:.2}$!", volume, pair, latest_close)).await;
                            }
                        }
                    } else {
                        warn!("Insufficient USD balance to BUY {}", pair);
                    }
                }
                Signal::Sell => {
                    let volume = state.get_balance(base_asset).await;
                    if volume > 0.0 {
                        if let Err(e) = state.execute_trade(OrderSide::Sell, &pair, volume, latest_close).await {
                            warn!("Could not execute SELL: {}", e);
                        } else {
                            if let Some(tg) = &telegram {
                                tg.send_message(&format!("✅ <b>Eseguito SELL</b> di {:.4} {} a {:.2}$!", volume, pair, latest_close)).await;
                            }
                        }
                    } else {
                        warn!("Insufficient {} balance to SELL {}", base_asset, pair);
                    }
                }
                Signal::Hold => {
                    info!("Signal: HOLD for {}. No action taken.", pair);
                }
            }

            state.print_portfolio_summary(latest_close, &pair).await;
        }
    }

    Ok(())
}

fn extract_closes(ohlc_data: &serde_json::Value, pair: &str) -> Vec<f64> {
    let mut closes = Vec::new();
    let candles = ohlc_data.get(pair).or_else(|| {
        ohlc_data
            .as_object()
            .and_then(|obj| obj.values().find(|v| v.is_array()))
    });

    if let Some(serde_json::Value::Array(arr)) = candles {
        for candle in arr {
            if let serde_json::Value::Array(c) = candle {
                if let Some(close_str) = c.get(4).and_then(|v| v.as_str()) {
                    if let Ok(val) = close_str.parse::<f64>() {
                        closes.push(val);
                    }
                } else if let Some(close_num) = c.get(4).and_then(|v| v.as_f64()) {
                    closes.push(close_num);
                }
            }
        }
    }
    closes
}

fn get_quote_asset(pair: &str) -> &str {
    if pair.contains('/') {
        pair.split('/').last().unwrap_or("USD")
    } else {
        let len = pair.len();
        if len > 3 {
            &pair[len - 3..]
        } else {
            "USD"
        }
    }
}

fn get_base_asset(pair: &str) -> &str {
    if pair.contains('/') {
        pair.split('/').next().unwrap_or("BTC")
    } else {
        let len = pair.len();
        if len > 3 {
            &pair[..len - 3]
        } else {
            "BTC"
        }
    }
}
