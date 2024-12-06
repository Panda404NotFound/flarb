// src/config.rs

use reqwest::{Client, header};
use std::sync::OnceLock;
use std::time::Duration;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

pub fn initialize_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        
        // Добавляем важные заголовки для оптимизации
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        
        // Создаем клиент с оптимизированными настройками
        Client::builder()
            .default_headers(headers)
            .pool_idle_timeout(Some(Duration::from_secs(300)))
            .pool_max_idle_per_host(100) // Увеличиваем пул соединений
            .http2_prior_knowledge() // Принудительно используем HTTP/2
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .tcp_nodelay(true) // Отключаем алгоритм Нагла для снижения латентности
            .connection_verbose(true)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client")
    })
}