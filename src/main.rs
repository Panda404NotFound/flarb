// main.rs
mod macros;
mod config;
mod jupiter;
mod test_param;

use log::info;
use jupiter::quote::test_valid_pools;
use crate::config::INITIALIZE_HTTP_CLIENT;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Инициализируем логгер
    env_logger::init();

    // Инициализируем конфигурацию
    let config = config::get_config();
    info!("Инициализация конфига выполнена");
    
    // Устанавливаем URL для Jupiter API только если INITIALIZE = true
    if INITIALIZE_HTTP_CLIENT {
        std::env::set_var("QUOTE_API_URL", &config.local_api_host);
        info!("Подменили Jup-ag клиента на свой локальный");
        
        // Инициализируем HTTP клиент
        config::initialize_http_client();
        info!("HTTP/2 клиент инициализирован");
    }
    
    info!("Тестирование пулов Jupiter...");
    test_valid_pools().await?;
    
    Ok(())
}