use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::{error, info};
use crate::errors::{KrakenError, Result};
use crate::bot::strategy::Signal;

#[derive(Deserialize, Debug)]
pub struct LlmDecision {
    pub decision: String,
    pub confidence: u8,
}

impl LlmDecision {
    pub fn to_signal(&self) -> Signal {
        match self.decision.as_str() {
            "BUY" => Signal::Buy,
            "SELL" => Signal::Sell,
            _ => Signal::Hold,
        }
    }
}

pub struct OpenRouterClient {
    client: Client,
    api_key: String,
}

impl OpenRouterClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| KrakenError::Validation(format!("Failed to build OpenRouter client: {}", e)))?;
            
        Ok(Self { client, api_key })
    }

    pub async fn analyze_sentiment(&self, asset: &str, news: &[String], usd_balance: f64, asset_balance: f64) -> Result<LlmDecision> {
        if self.api_key.is_empty() {
            info!("OPENROUTER_API_KEY not set. Using mock LLM response.");
            return Ok(LlmDecision {
                decision: "HOLD".to_string(),
                confidence: 50,
            });
        }

        let news_text = news.join(" | ");
        let prompt = format!(
            "Sei un trader aggressivo senza esagerare. Sii propenso al rischio senza esagerare. Se c'è positività tecnica o di news, favorisci un segnale di BUY.\nSei un analista finanziario. Tu hai {:.2} USD e {:.4} {}. Leggi queste news recenti:\n{}\nC'è panico o euforia? Se hai zero {} e c'è euforia (o assenza di notizie negative): BUY. Se hai molti {} e c'è panico: SELL. Altrimenti HOLD.\nRispondi ESCLUSIVAMENTE in JSON con 'decision' (BUY, SELL, HOLD) e 'confidence' (1-100).",
            usd_balance, asset_balance, asset, news_text, asset, asset
        );

        let url = "https://openrouter.ai/api/v1/chat/completions";
        
        let payload = json!({
            "model": "meta-llama/llama-3-8b-instruct:free",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.1
        });

        match self.client.post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "http://localhost")
            .header("X-Title", "KrakenBot")
            .json(&payload)
            .send()
            .await 
        {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    error!("OpenRouter API error: {} - {}", status, err_text);
                    return Err(KrakenError::Validation(format!("OpenRouter API returned {}", status)));
                }

                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                
                if let Some(content) = json
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    let clean_text = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                    
                    if let Ok(decision) = serde_json::from_str::<LlmDecision>(clean_text) {
                        return Ok(decision);
                    } else {
                        error!("Failed to parse JSON from LLM: {}", clean_text);
                    }
                }
                
                Ok(LlmDecision { decision: "HOLD".to_string(), confidence: 0 })
            }
            Err(e) => {
                error!("LLM API timeout/error: {}", e);
                Err(KrakenError::Validation(format!("LLM API error: {}", e)))
            }
        }
    }
}
