use reqwest::Client;
use std::error::Error;
use tracing::{error, info};

pub async fn get_global_usd_assets(limit: usize) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    info!("Fetching global market ticker from Kraken...");
    let url = "https://api.kraken.com/0/public/Ticker";
    
    let client = Client::new();
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        error!("Kraken Ticker API returned status: {}", resp.status());
        return Err(format!("API error: {}", resp.status()).into());
    }

    let json: serde_json::Value = resp.json().await?;
    
    if let Some(errs) = json.get("error").and_then(|e| e.as_array()) {
        if !errs.is_empty() {
            let msg = format!("Kraken API returned errors: {:?}", errs);
            error!("{}", msg);
            return Err(msg.into());
        }
    }

    let stablecoins = vec!["USDT", "USDC", "DAI", "FDUSD", "TUSD", "USDD"];
    let mut candidates: Vec<(String, f64)> = Vec::new();

    if let Some(result) = json.get("result").and_then(|r| r.as_object()) {
        for (pair, data) in result {
            if !pair.ends_with("USD") {
                continue;
            }
            
            // Extract base asset by removing 'USD' or 'ZUSD'
            let mut base = pair.clone();
            if base.ends_with("ZUSD") {
                base = base.replace("ZUSD", "");
            } else if base.ends_with("USD") {
                base = base.replace("USD", "");
            }
            // Remove Kraken 'X' or 'Z' prefix if present for crypto
            if base.starts_with('X') && base.len() > 3 {
                base = base[1..].to_string();
            }

            if stablecoins.contains(&base.as_str()) {
                continue;
            }

            // Extract 24h volume: data.v[1]
            if let Some(v_arr) = data.get("v").and_then(|v| v.as_array()) {
                if v_arr.len() > 1 {
                    if let Some(vol_str) = v_arr[1].as_str() {
                        if let Ok(vol) = vol_str.parse::<f64>() {
                            candidates.push((base, vol));
                        }
                    }
                }
            }
        }
    }

    // Sort by volume descending
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top_assets: Vec<String> = candidates.into_iter().take(limit).map(|(base, _)| base).collect();
    
    info!("Market Screener selected top {} USD assets by volume.", top_assets.len());
    Ok(top_assets)
}
