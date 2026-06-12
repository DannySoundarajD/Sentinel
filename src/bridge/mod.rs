// Sentinel Bridge: Telegram bot integration

pub struct TelegramBridge;

impl TelegramBridge {
    pub async fn start(token: &str, api_base: &str) -> anyhow::Result<()> {
        println!("Telegram bridge starting with token: {}, api: {}", token, api_base);
        Ok(())
    }
}
