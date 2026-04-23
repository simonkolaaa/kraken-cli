use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};
use crate::errors::{KrakenError, Result};
use crate::bot::strategy::Signal;

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

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

pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| KrakenError::Validation(format!("Failed to build Gemini client: {}", e)))?;
            
        Ok(Self { client, api_key })
    }

    pub async fn analyze_sentiment(&self, asset: &str, news: &[String]) -> Result<LlmDecision> {
        if self.api_key.is_empty() {
            info!("GEMINI_API_KEY not set. Using mock LLM response.");
            return Ok(LlmDecision {
                decision: "HOLD".to_string(),
                confidence: 50,
            });
        }

        let news_text = news.join("\n- ");
        let prompt = format!(
            "Sei un analista finanziario. Leggi queste news recenti:\n- {}\nC'è un sentiment rialzista o ribassista per l'asset {}? Rispondi ESCLUSIVAMENTE con un JSON: {{ \"decision\": \"BUY\" | \"SELL\" | \"HOLD\", \"confidence\": 1-100 }}.",
            news_text, asset
        );

        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={}", self.api_key);
        
        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };

        match self.client.post(&url).json(&request_body).send().await {
            Ok(resp) => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                
                if let Some(text) = json
                    .get("candidates")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("content"))
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.get(0))
                    .and_then(|p| p.get("text"))
                    .and_then(|t| t.as_str())
                {
                    // Clean up markdown formatting if LLM includes it
                    let clean_text = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                    
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
