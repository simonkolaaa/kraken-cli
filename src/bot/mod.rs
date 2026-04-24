pub mod state;
pub mod strategy;
pub mod news;
pub mod llm;
pub mod dashboard;
pub mod telegram;

use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use crate::client::SpotClient;
use crate::paper::OrderSide;
use state::BotState;
use strategy::{MarketContext, LlmSentimentStrategy, Signal, TradingStrategy};
use llm::GeminiClient;
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

    let llm_client = GeminiClient::new()?;
    let news_fetcher = NewsFetcher::new()?;
    let mut strategy = LlmSentimentStrategy::new(llm_client, 60);

    let mut ticker = time::interval(Duration::from_secs(interval_minutes * 60));

    loop {
        ticker.tick().await;

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

        // 2. Iterate over watchlist
        for pair in &watchlist {
            info!("Evaluating asset: {}", pair);
            
            let params = vec![("pair", pair.as_str()), ("interval", "1")];
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

            let latest_close = extract_latest_close(&ohlc_data, pair).unwrap_or(0.0);
            if latest_close == 0.0 {
                warn!("Could not extract latest price for {}. Skipping.", pair);
                continue;
            }

            let quote_asset = get_quote_asset(pair);
            let base_asset = get_base_asset(pair);
            let usd_balance = state.get_balance(quote_asset).await;
            let asset_balance = state.get_balance(base_asset).await;

            let pair_context = MarketContext {
                ohlc_data: Some(&ohlc_data),
                news: &general_news,
                usd_balance,
                asset_balance,
            };

            let signal = strategy.evaluate(&pair_context, pair).await;

            match signal {
                Signal::Buy => {
                    let balance = state.get_balance(quote_asset).await;
                    if balance > 0.0 {
                        // Allocate 20% of quote asset for multi-asset strategy
                        let amount_to_spend = balance * 0.2; 
                        let volume = amount_to_spend / latest_close;
                        
                        if let Err(e) = state.execute_trade(OrderSide::Buy, pair, volume, latest_close).await {
                            warn!("Could not execute BUY: {}", e);
                        } else {
                            if let Some(tg) = &telegram {
                                tg.send_message(&format!("✅ <b>Eseguito BUY</b> di {:.4} {} a {:.2}$!", volume, pair, latest_close)).await;
                            }
                        }
                    } else {
                        warn!("Insufficient {} balance to BUY {}", quote_asset, pair);
                    }
                }
                Signal::Sell => {
                    let volume = state.get_balance(base_asset).await;
                    if volume > 0.0 {
                        if let Err(e) = state.execute_trade(OrderSide::Sell, pair, volume, latest_close).await {
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

            state.print_portfolio_summary(latest_close, pair).await;
        }
    }

    Ok(())
}

fn extract_latest_close(ohlc_data: &serde_json::Value, pair: &str) -> Option<f64> {
    let candles = ohlc_data.get(pair).or_else(|| {
        ohlc_data
            .as_object()
            .and_then(|obj| obj.values().find(|v| v.is_array()))
    });

    if let Some(serde_json::Value::Array(arr)) = candles {
        if let Some(serde_json::Value::Array(last_candle)) = arr.last() {
            if let Some(close_str) = last_candle.get(4).and_then(|v| v.as_str()) {
                return close_str.parse::<f64>().ok();
            } else if let Some(close_num) = last_candle.get(4).and_then(|v| v.as_f64()) {
                return Some(close_num);
            }
        }
    }
    None
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
