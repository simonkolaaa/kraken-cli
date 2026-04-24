use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::errors::Result;
use crate::paper::{self, OrderSide, PaperState};

#[derive(Clone)]
pub(crate) struct BotState {
    pub(crate) paper_state: Arc<RwLock<PaperState>>,
}

impl BotState {
    pub fn new() -> Result<Self> {
        let paper_state = match paper::load_state() {
            Ok(state) => state,
            Err(_) => {
                info!("Paper state not found. Initializing new paper account with 10000 USD...");
                let mut new_state = PaperState::new(10000.0, "USD");
                paper::save_state(&new_state)?;
                new_state
            }
        };

        Ok(Self {
            paper_state: Arc::new(RwLock::new(paper_state)),
        })
    }

    pub(crate) async fn get_balance(&self, asset: &str) -> f64 {
        let state = self.paper_state.read().await;
        state.available_balance(asset)
    }

    pub(crate) async fn execute_trade(
        &self,
        side: OrderSide,
        pair: &str,
        volume: f64,
        price: f64,
    ) -> Result<()> {
        info!("Executing paper trade: {:?} {} {} @ {}", side, volume, pair, price);

        let mut state = self.paper_state.write().await;
        match state.place_market_order(side, pair, volume, price, price) {
            Ok(trade) => {
                info!("Trade filled successfully: {:?}", trade);
                paper::save_state(&*state)?;
            }
            Err(e) => {
                error!("Failed to execute trade: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub(crate) async fn print_portfolio_summary(&self, current_price: f64, pair: &str) {
        let state = self.paper_state.read().await;
        let mut prices = HashMap::new();
        let pair_no_slash = pair.replace("/", "");
        prices.insert(pair_no_slash, (current_price, current_price));
        
        let (total_value, _) = state.compute_portfolio_value(&prices);
        info!("Current Portfolio Value: {:.2} USD", total_value);
    }
}
