// src/main.rs

mod macros;
mod config;
mod data;
mod fetch_address;
mod decoder;
mod websocket;
mod shred;
mod graph;
mod math;
mod router;

#[allow(unused_imports)]
use log::{info, error};
use crate::data::GLOBAL_DATA;
use crate::websocket::ws_data::DexType;

use crate::websocket::ws_orca::{start_orca_websocket_finalized, start_orca_websocket_processed};
use crate::websocket::ws_raydium::{start_raydium_websocket_finalized, start_raydium_websocket_processed};
use crate::websocket::ws_meteora::{start_meteora_websocket_finalized, start_meteora_websocket_processed};

use crate::config::{INITIALIZE_HTTP_CLIENT, get_config, DEFAULT_QUOTE_API_URL};
#[allow(unused_imports)]
use crate::shred::test_udp_connection;
use crate::fetch_address::start_fetching;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Инициализация только подсистемы логирования
    init_logging!();

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
        // Используем дефолтный URL если не инициализируем свой HTTP клиент
        std::env::set_var("QUOTE_API_URL", DEFAULT_QUOTE_API_URL);
        info!("Используем стандартный Jupiter API URL");
    }
/*
    // Запуск Shred Stream
    info!("Запуск Shred Stream...");
    tokio::spawn(async {
        let _ = shred::start_shred_stream().await;
    });
    info!("Shred Stream запущен");

    tokio::spawn(async {
        if let Err(e) = test_udp_connection().await {
            error!("Ошибка тестирования UDP: {}", e);
        }
    });
*/

    // Инициализация структуры DEX для пулов
    GLOBAL_DATA.initialize_pool_states(DexType::Orca);
    GLOBAL_DATA.initialize_pool_states(DexType::Raydium);
    GLOBAL_DATA.initialize_pool_states(DexType::Meteora);

    // Проверяем и загружаем файлы пулов
    config::check_pools().await?;
    info!("Проверка пулов завершена");
    
    // TODO: Запуск парсера интересующих DEX 
    info!("Запуск парсера адресов DEX...");
    start_fetching().await?;
    info!("Парсинг адресов DEX завершен");

    info!("Запуск построения графа и поиска цепочек...");
    graph::build_and_find_chains();
    info!("Построение графа и поиск цепочек завершены");
/*
    // TODO: Запуск RPC вызова для получения актуальных данных

    // TODO: Запуск построения графов 

    // TODO: Запуск поиска оптимальных путей

    // TODO: Запуск калькулятора
    
    // Запускаем тестирование Orca Whirlpool
    // info!("Запуск тестирования Orca Whirlpool...");
    // quote::test_valid_pools().await?;
*/ 

    // Запуск подписки на пулах Finalized в отдельной задаче
    tokio::spawn(async {
        let _ = start_orca_websocket_finalized().await;
    });

    // Запуск подписки на пулах Processed в отдельной задаче
    tokio::spawn(async {
        let _ = start_orca_websocket_processed().await;
    });

    // Запуск подписки на пулах Raydium CLMM и Raydium V4 finalized
    tokio::spawn(async {
        let _ = start_raydium_websocket_finalized().await;
    });
    
    // Запуск подписки на пулах Raydium CLMM и Raydium V4 Processed
    tokio::spawn(async {
        let _ = start_raydium_websocket_processed().await;
    });

    // Запуск подписки на пулах Meteora finalized
    tokio::spawn(async {
        let _ = start_meteora_websocket_finalized().await;
    });
        
    // Запуск подписки на пулах Meteora Processed
    tokio::spawn(async {
        let _ = start_meteora_websocket_processed().await;
    });

    // Держим главный поток активным
    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}