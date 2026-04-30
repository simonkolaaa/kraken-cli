use crate::bot::llm::OpenRouterClient;
use serde_json::Value;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub struct MarketContext<'a> {
    pub ohlc_data: Option<&'a Value>,
    pub news: &'a [String],
    pub usd_balance: f64,
    pub asset_balance: f64,
}

pub trait TradingStrategy {
    #[allow(async_fn_in_trait)]
    async fn evaluate(&mut self, context: &MarketContext<'_>, pair: &str) -> Signal;
}

pub struct LlmSentimentStrategy {
    llm_client: OpenRouterClient,
    confidence_threshold: u8,
}

impl LlmSentimentStrategy {
    pub fn new(llm_client: OpenRouterClient, confidence_threshold: u8) -> Self {
        Self {
            llm_client,
            confidence_threshold,
        }
    }
}

impl TradingStrategy for LlmSentimentStrategy {
    async fn evaluate(&mut self, context: &MarketContext<'_>, pair: &str) -> Signal {
        let fallback_news = vec!["Nessuna notizia recente. Assenza di notizie negative, valuta positivamente l'assenza di panico e i segnali tecnici.".to_string()];
        let news_to_use = if context.news.is_empty() {
            warn!("No news available for LLM evaluation on {}. Using fallback positive context.", pair);
            &fallback_news
        } else {
            context.news
        };

        match self.llm_client.analyze_sentiment(pair, news_to_use, context.usd_balance, context.asset_balance).await {
            Ok(decision) => {
                info!("LLM Decision for {}: {} (Confidence: {}%)", pair, decision.decision, decision.confidence);
                if decision.confidence < self.confidence_threshold {
                    info!("Confidence {} is below threshold {}. Forcing HOLD.", decision.confidence, self.confidence_threshold);
                    return Signal::Hold;
                }
                decision.to_signal()
            }
            Err(e) => {
                warn!("Failed to get LLM decision: {}. Defaulting to HOLD.", e);
                Signal::Hold
            }
        }
    }
}

pub struct SmaCrossover {
    short_window: usize,
    long_window: usize,
    last_signal: Signal,
}

impl SmaCrossover {
    pub fn new(short_window: usize, long_window: usize) -> Self {
        Self {
            short_window,
            long_window,
            last_signal: Signal::Hold,
        }
    }

    fn calculate_sma(closes: &[f64], window: usize) -> Option<f64> {
        if closes.len() < window || window == 0 {
            return None;
        }
        let sum: f64 = closes.iter().rev().take(window).sum();
        Some(sum / window as f64)
    }
}

impl TradingStrategy for SmaCrossover {
    async fn evaluate(&mut self, context: &MarketContext<'_>, pair: &str) -> Signal {
        let ohlc_data = match context.ohlc_data {
            Some(data) => data,
            None => return Signal::Hold,
        };

        let candles = ohlc_data.get(pair).or_else(|| {
            ohlc_data
                .as_object()
                .and_then(|obj| obj.values().find(|v| v.is_array()))
        });

        let mut closes = Vec::new();
        if let Some(Value::Array(arr)) = candles {
            for candle in arr {
                if let Value::Array(c) = candle {
                    if let Some(close_str) = c.get(4).and_then(|v| v.as_str()) {
                        if let Ok(close_val) = close_str.parse::<f64>() {
                            closes.push(close_val);
                        }
                    } else if let Some(close_num) = c.get(4).and_then(|v| v.as_f64()) {
                        closes.push(close_num);
                    }
                }
            }
        }

        debug!("Extracted {} close prices for {}", closes.len(), pair);

        let short_sma = Self::calculate_sma(&closes, self.short_window);
        let long_sma = Self::calculate_sma(&closes, self.long_window);

        if let (Some(short), Some(long)) = (short_sma, long_sma) {
            let current_signal = if short > long {
                Signal::Buy
            } else if short < long {
                Signal::Sell
            } else {
                Signal::Hold
            };

            let signal_to_return = if current_signal != self.last_signal && current_signal != Signal::Hold {
                info!("SMA Crossover detected: {:?} -> {:?}", self.last_signal, current_signal);
                current_signal
            } else {
                Signal::Hold
            };

            if current_signal != Signal::Hold {
                self.last_signal = current_signal;
            }

            signal_to_return
        } else {
            Signal::Hold
        }
    }
}
