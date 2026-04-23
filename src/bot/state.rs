use std::collections::HashMap;
use tracing::{error, info};

use crate::errors::Result;
use crate::paper::{self, OrderSide, PaperState};

pub(crate) struct BotState {
    pub(crate) paper_state: PaperState,
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

        Ok(Self { paper_state })
    }

    pub(crate) fn get_balance(&self, asset: &str) -> f64 {
        self.paper_state.available_balance(asset)
    }

    pub(crate) fn execute_trade(
        &mut self,
        side: OrderSide,
        pair: &str,
        volume: f64,
        price: f64,
    ) -> Result<()> {
        info!("Executing paper trade: {:?} {} {} @ {}", side, volume, pair, price);

        match self.paper_state.place_market_order(side, pair, volume, price, price) {
            Ok(trade) => {
                info!("Trade filled successfully: {:?}", trade);
                paper::save_state(&self.paper_state)?;
            }
            Err(e) => {
                error!("Failed to execute trade: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub(crate) fn print_portfolio_summary(&self, current_price: f64, pair: &str) {
        let mut prices = HashMap::new();
        let pair_no_slash = pair.replace("/", "");
        prices.insert(pair_no_slash, (current_price, current_price));
        
        let (total_value, _) = self.paper_state.compute_portfolio_value(&prices);
        info!("Current Portfolio Value: {:.2} USD", total_value);
    }
}
