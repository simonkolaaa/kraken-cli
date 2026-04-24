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
}
