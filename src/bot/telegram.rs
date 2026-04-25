use reqwest::Client;
use serde_json::json;
use tracing::{error, info};

#[derive(Clone)]
pub struct TelegramNotifier {
    client: Client,
    token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new() -> Option<Self> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID").ok()?;

        if token.is_empty() || chat_id.is_empty() {
            return None;
        }

        Some(Self {
            client: Client::new(),
            token,
            chat_id,
        })
    }

    pub async fn send_message(&self, message: &str) {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        
        let payload = json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "HTML"
        });

        match self.client.post(&url).json(&payload).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    error!("Failed to send Telegram message. Status: {}", resp.status());
                } else {
                    info!("Telegram notification sent: {}", message);
                }
            }
            Err(e) => {
                error!("Error sending Telegram message: {}", e);
            }
        }
    }
    pub async fn get_updates(&self, offset: u64) -> Result<(u64, Vec<serde_json::Value>), String> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=30", self.token, offset);
        
        let resp = self.client.get(&url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("Status {}", resp.status()));
        }
        
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut new_offset = offset;
        let mut updates = Vec::new();

        if let Some(result) = json.get("result").and_then(|r| r.as_array()) {
            for update in result {
                if let Some(update_id) = update.get("update_id").and_then(|u| u.as_u64()) {
                    new_offset = new_offset.max(update_id + 1);
                    updates.push(update.clone());
                }
            }
        }

        Ok((new_offset, updates))
    }
}
