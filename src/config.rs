// src/config.rs

use dotenv::dotenv;
use std::env;
use lazy_static::lazy_static;

// Константы для HTTP клиента
#[allow(dead_code)]
pub const INITIALIZE_HTTP_CLIENT: bool = false;
#[allow(dead_code)]
pub const DEFAULT_QUOTE_API_URL: &str = "https://quote-api.jup.ag/v6";
#[allow(dead_code)]
pub const H2_INITIAL_WINDOW_SIZE: u32 = 1024 * 1024 * 2; // 2MB для локального соединения

// Константы для батчинга и rate limiting
#[allow(dead_code)]
pub const BATCH_SIZE: usize = 125;           // Размер батча для RPC запросов
#[allow(dead_code)]
pub const RATE_LIMIT: u32 = 450;             // Максимальное количество запросов
#[allow(dead_code)]
pub const RATE_LIMIT_REFRESH: u64 = 1;       // Время обновления rate limit в секундах

// Константы для параллельной обработки
#[allow(dead_code)]
pub const WORKER_THREADS: usize = 4;         // Количество воркеров
#[allow(dead_code)]
pub const CPU_CORES: usize = 4;              // Количество ядер системы

// Функции для доступа к конфигурации
pub fn get_config() -> &'static Config {
    &CONFIG
}

// Структура конфигурации
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Config {
    pub local_api_host: String,
    pub solana_rpc_url: String,
    pub wallet_private_key: String,
    pub jupiter_program_id: String,
    pub helius_api_key: String,
    pub helius_rpc_url: String,
    pub helius_enhanced_rpc_url: String,
    pub helius_websocket_url: String,
    pub helius_yellowstone_endpoint: String,
    pub helius_yellowstone_auth_token: String,
}

// Глобальная конфигурация
lazy_static! {
    pub static ref CONFIG: Config = {
        dotenv().ok();
        
        Config {
            local_api_host: env::var("LOCAL_API_HOST")
                .expect("LOCAL_API_HOST must be set"),
            solana_rpc_url: env::var("SOLANA_RPC_URL")
                .expect("SOLANA_RPC_URL must be set"),
            wallet_private_key: env::var("WALLET_PRIVATE_KEY")
                .expect("WALLET_PRIVATE_KEY must be set"),
            jupiter_program_id: env::var("JUPITER_PROGRAM_ID")
                .expect("JUPITER_PROGRAM_ID must be set"),
            helius_api_key: env::var("HELIUS_API_KEY")
                .expect("HELIUS_API_KEY must be set"),
            helius_rpc_url: env::var("HELIUS_RPC_URL")
                .expect("HELIUS_RPC_URL must be set"),
            helius_enhanced_rpc_url: env::var("HELIUS_ENCHANCED_RPC_URL")
                .expect("HELIUS_ENCHANCED_RPC_URL must be set"),
            helius_websocket_url: env::var("HELIUS_WEBSOCKET_URL")
                .expect("HELIUS_WEBSOCKET_URL must be set"),
            helius_yellowstone_endpoint: env::var("HELIUS_YELLOWSTONE_ENDPOINT")
                .expect("HELIUS_YELLOWSTONE_ENDPOINT must be set"),
            helius_yellowstone_auth_token: env::var("HELIUS_YELLOWSTONE_AUTH_TOKEN")
                .expect("HELIUS_YELLOWSTONE_AUTH_TOKEN must be set"),
        }
    };
}