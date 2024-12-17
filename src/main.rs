// src/main.rs

mod macros;
mod config;
mod params;
mod ws_orca;
mod ws_parser;
mod data;
mod fetch_address;
mod decoder;

use log::info;
use crate::ws_orca::{start_orca_websocket_finalized, start_orca_websocket_processed};
use crate::config::{INITIALIZE_HTTP_CLIENT, get_config, DEFAULT_QUOTE_API_URL};
use crate::fetch_address::start_fetching;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Initialize only tracing subscriber
    tracing_subscriber::fmt::init();

    // Инициализируем конфигурацию
    let config = get_config();
    info!("Инициализация конфига выполнена");

    // Устанавливаем URL для Jupiter API
    if INITIALIZE_HTTP_CLIENT {
        std::env::set_var("QUOTE_API_URL", &config.local_api_host);
        info!("Подменили Jup-ag клиента на свой локальный");
        
        // Инициализируем HTTP клиент
        config::initialize_http_client();
        info!("HTTP/2 клиент инициализирован");
    } else {
        // Используем дефолтный URL ��сли не инициализируем свой HTTP клиент
        std::env::set_var("QUOTE_API_URL", DEFAULT_QUOTE_API_URL);
        info!("Используем стандартный Jupiter API URL");
    }


    // Проверяем и загружаем файлы пулов
    config::check_pools().await?;
    info!("Проверка пулов завершена");
    
    // TODO: Запуск парсера интересующих DEX 
    info!("Запуск парсера адресов DEX...");
    start_fetching().await?;
    info!("Парсинг адресов DEX завершен");

    // TODO: Запуск RPC вызова для получения актуальных данных

    // TODO: Запуск построения графов 

    // TODO: Запуск поиска оптимальных путей

    // TODO: Запуск калькулятора
    
    // Запускаем тестирование Orca Whirlpool
    // info!("Запуск тестирования Orca Whirlpool...");
    // quote::test_valid_pools().await?;

    // Запуск подписки на пулах Finalized в отдельной задаче
    tokio::spawn(async {
        let _ = start_orca_websocket_finalized().await;
    });

    // Запуск подписки на пулах Processed в отдельной задаче
    tokio::spawn(async {
        let _ = start_orca_websocket_processed().await;
    });
    
    // Держим главный поток активным
    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}