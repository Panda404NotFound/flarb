// src/config.rs

use dotenv::dotenv;
use std::env;
use lazy_static::lazy_static;
use reqwest::{Client, header};
use std::sync::OnceLock;
use std::time::Duration;

// Статический HTTP клиент
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

// Глобальные константы для клиента jup-ag
pub const INITIALIZE_HTTP_CLIENT: bool = false;
pub const DEFAULT_QUOTE_API_URL: &str = "https://quote-api.jup.ag/v6";

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

pub fn initialize_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        
        // HTTP/2 specific headers
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );
        
        Client::builder()
            .default_headers(headers)
            // Отключаем таймаут простоя - соединение будет поддерживаться постоянно
            .pool_idle_timeout(None)
            // Держим только одно соединение на хост для эффективного переиспользования
            .pool_max_idle_per_host(1)
            // Принудительно используем HTTP/2
            .http2_prior_knowledge()
            // Настройки TCP для стабильного соединения
            .tcp_keepalive(Some(Duration::from_secs(300)))
            .tcp_nodelay(true)
            // Увеличиваем таймаут запроса до разумного значения
            .timeout(Duration::from_secs(30))
            // Настройки HTTP/2
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .http2_adaptive_window(true)
            .build()
            .expect("Failed to create HTTP client")
    })
}

// Функции для доступа к конфигурации
#[allow(dead_code)]
pub fn get_config() -> &'static Config {
    &CONFIG
}

#[allow(dead_code)]
pub fn get_http_client() -> &'static Client {
    initialize_http_client()
}