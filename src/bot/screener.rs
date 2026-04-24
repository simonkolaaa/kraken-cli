use reqwest::Client;
use std::error::Error;
use tracing::{error, info};

pub async fn get_hot_assets(limit: usize) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    info!("Fetching hot assets from CryptoCompare screener...");
    let url = "https://min-api.cryptocompare.com/data/top/totalvolfull?limit=20&tsym=USD";
    
    let client = Client::new();
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        error!("CryptoCompare screener API returned status: {}", resp.status());
        return Err(format!("API error: {}", resp.status()).into());
    }

    let json: serde_json::Value = resp.json().await?;
    let mut hot_assets = Vec::new();

    let stablecoins = vec!["USDT", "USDC", "DAI", "FDUSD", "TUSD", "USDD"];

    if let Some(data) = json.get("Data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(coin_info) = item.get("CoinInfo") {
                if let Some(name) = coin_info.get("Name").and_then(|n| n.as_str()) {
                    let name_upper = name.to_uppercase();
                    if !stablecoins.contains(&name_upper.as_str()) {
                        hot_assets.push(name_upper);
                        if hot_assets.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
    }

    info!("Market Screener selected hot assets: {:?}", hot_assets);
    Ok(hot_assets)
}
