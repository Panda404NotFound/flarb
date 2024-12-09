// src/main.rs

mod macros;
mod config;
mod jupiter;
mod params;
mod markets;

use log::info;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Инициализируем логгер
    env_logger::init();

    // Инициализируем конфигурацию
    config::get_config();
    info!("Инициализация конфига выполнена");
    
    // Запускаем тестирование Orca Whirlpool
    info!("Запуск тестирования Orca Whirlpool...");
    markets::orca_test::run_whirlpool_test().await?;
    
    Ok(())
}