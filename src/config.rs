// src/config.rs

use dotenv::dotenv;
use std::env;
use lazy_static::lazy_static;
use reqwest::{Client, header};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::fs;
use log::{info, warn};
use std::path::Path;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};

// Статический HTTP клиент
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

// Константы для HTTP клиента
#[allow(dead_code)]
pub const INITIALIZE_HTTP_CLIENT: bool = false;
#[allow(dead_code)]
pub const DEFAULT_QUOTE_API_URL: &str = "https://quote-api.jup.ag/v6";

// Константы для фильтрации пулов
pub const MIN_TVL: f64 = 100000.0;             // Минимальный TVL для пула
pub const INITIAL_BALANCE: u64 = 100_000_000_000; // Константа начального баланса 100 SOL в лампортах
pub const INITIAL_TOKENS: [&str; 8] = ["SOL", "USDC", "USDT", "JUP", "ETH", "WETH", "JTO", "PYTH"];  // Начальные токены для торговли
pub const MAX_CHAIN_LENGTH: usize = 5;       // Максимальная длина цепочки включительно
pub const MIN_CHAIN_LENGTH: usize = 3;       // Минимальная длина цепочки включительно
pub const START_END_TOKEN_FOR_CHAINS: [&str; 1] = ["SOL"]; // Начальный и конечный токен для построения цепочек

// Добавим константы для URL скачивания пулов   
pub const METEORA_POOLS_URL: &str = "https://dlmm-api.meteora.ag/pair/all";
pub const ORCA_POOLS_URL: &str = "https://api.mainnet.orca.so/v1/whirlpool/list";
pub const RAYDIUM_POOLS_URL: &str = "https://api.raydium.io/v2/ammV3/ammPools";
pub const TOKENS_URL: &str = "https://tokens.jup.ag/tokens?tags=verified,community";

// Program IDs
pub const ORCA_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const METEORA_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

// Добавим структуру для описания пула
struct PoolFile {
    name: &'static str,
    url: &'static str,
}

// Функции для доступа к конфигурации
pub fn get_config() -> &'static Config {
    &CONFIG
}

#[allow(dead_code)]
// Функция для доступа к HTTP клиенту
pub fn get_http_client() -> &'static Client {
    initialize_http_client()
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
    pub dest_ip_ports: String,
    pub jito_udp_port: String,
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
            dest_ip_ports: env::var("DEST_IP_PORTS")
                .expect("DEST_IP_PORTS must be set"),
            jito_udp_port: env::var("JITO_UDP_PORT")
                .expect("JITO_UDP_PORT must be set"),
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
        // headers.insert(
        //     header::CONNECTION,
        //     header::HeaderValue::from_static("keep-alive"),
        // );
        
        Client::builder()
            .default_headers(headers)
            // При желании можно оставить pool_idle_timeout(None), но обычно лучше позволить соединениям закрываться
            // .pool_idle_timeout(None)
            .pool_max_idle_per_host(1)
            // Убираем http2_prior_knowledge(), позволяя автоматический ALPN (HTTP/1.1 или HTTP/2)
            // .http2_prior_knowledge()
            // Также можно убрать или оставить настройки HTTP/2, 
            // но для устранения конфликтов стоит убрать явно:
            // .http2_keep_alive_interval(Duration::from_secs(30))
            // .http2_keep_alive_timeout(Duration::from_secs(10))
            // .http2_adaptive_window(true)
            // Таймаут запроса
            .timeout(Duration::from_secs(30))
            // Настройки TCP
            .tcp_keepalive(Some(Duration::from_secs(300)))
            .tcp_nodelay(true)
            // Увеличиваем таймаут запроса до разумного значения
            // .timeout(Duration::from_secs(30))
            // Настройки HTTP/2
            // .http2_keep_alive_interval(Duration::from_secs(30))
            // .http2_keep_alive_timeout(Duration::from_secs(10))
            // .http2_adaptive_window(true)
            .build()
            .expect("Failed to create HTTP client")
    })
}

// Загрузка файла пула
async fn download_pool(name: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Загрузка {} через reqwest", name);
    
    let path = format!("pools/{}", name);

    // Делаем GET-запрос через общий HTTP клиент
    let client = get_http_client();
    let response = client
        .get(url)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!("Ошибка при скачивании {}: код HTTP {}", name, status).into());
    }

    // Считываем ответ в строку
    let body = response.text().await?;
    
    // Парсим полученную строку, проверяем JSON
    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    
    // Форматируем и пишем в целевой файл
    let formatted = serde_json::to_string_pretty(&parsed)?;
    fs::write(&path, formatted).await?;

    info!("Файл {} успешно загружен и сохранен", name);
    Ok(())
}

// Проверка и загрузка файлов пулов
pub async fn check_pools() -> Result<(), Box<dyn std::error::Error>> {
    info!("Начинаем проверку файлов пулов");
    
    let pools = vec![
        PoolFile { name: "meteora_pools.json", url: METEORA_POOLS_URL },
        PoolFile { name: "orca_pools.json", url: ORCA_POOLS_URL },
        PoolFile { name: "raydium_pools.json", url: RAYDIUM_POOLS_URL },
        PoolFile { name: "tokens.json", url: TOKENS_URL },
    ];

    // Создаем директорию pools, если она не существует
    if !Path::new("pools").exists() {
        fs::create_dir("pools").await?;
        info!("Создана директория pools/");
    }

    let mut failed_downloads = Vec::new();
    let mut downloads = Vec::new();

    for pool in pools {
        let path = format!("pools/{}", pool.name);
        
        // Проверяем существование и валидность файла
        if let Ok(content) = fs::read_to_string(&path).await {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json) => {
                    if json.is_object() || json.is_array() {
                        if json.is_object() && json.as_object().unwrap().is_empty() {
                            info!("Файл {} пуст, требуется загрузка", pool.name);
                            downloads.push(pool);
                        } else {
                            info!("Файл {} существует и валиден", pool.name);
                        }
                    } else {
                        warn!("Файл {} имеет неверный формат, будет загружен заново", pool.name);
                        downloads.push(pool);
                    }
                }
                Err(e) => {
                    warn!("Ошибка парсинга {}: {}, файл будет загружен заново", pool.name, e);
                    downloads.push(pool);
                }
            }
        } else {
            info!("Файл {} не найден, начинаем загрузку", pool.name);
            downloads.push(pool);
        }
    }

    // Первая попытка загрузки
    if !downloads.is_empty() {
        info!("Начинаем параллельную загрузку {} файлов", downloads.len());
        
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} Загрузка... {elapsed_precise}")
                .unwrap()
        );
        
        pb.enable_steady_tick(std::time::Duration::from_millis(120));

        // Выполняем все загрузки параллельно и собираем результаты
        let results = join_all(downloads.iter().map(|pool| download_pool(pool.name, pool.url))).await;
        
        // Проверяем результаты и собираем неудачные загрузки
        for (i, result) in results.iter().enumerate() {
            if let Err(e) = result {
                warn!("Ошибка при загрузке файла {}: {}", downloads[i].name, e);
                failed_downloads.push(PoolFile {
                    name: downloads[i].name,
                    url: downloads[i].url
                });
            }
        }
        
        pb.finish_with_message("Первая попытка загрузки завершена");
    }

    // Повторная попытка для неудачных загрузок
    if !failed_downloads.is_empty() {
        info!("Повторная попытка загрузки для {} файлов", failed_downloads.len());
        
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} Повторная загрузка... {elapsed_precise}")
                .unwrap()
        );
        
        pb.enable_steady_tick(std::time::Duration::from_millis(120));

        // Повторная попытка загрузки
        let retry_results = join_all(
            failed_downloads.iter().map(|pool| download_pool(pool.name, pool.url))
        ).await;
        
        // Проверяем результаты повторной попытки
        for (i, result) in retry_results.iter().enumerate() {
            if let Err(e) = result {
                // При повторной ошибке останавливаем выполнение
                return Err(format!(
                    "Критическая ошибка при повторной загрузке файла {}: {}",
                    failed_downloads[i].name, e
                ).into());
            }
        }
        
        pb.finish_with_message("Повторная загрузка завершена успешно");
    }

    info!("Все файлы успешно загружены");
    Ok(())
}