// src/raydium_ws.rs

use tokio;
use tokio_tungstenite::{connect_async, WebSocketStream};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tracing::{info, error};
use tokio_tungstenite::tungstenite::http::Uri;
use crate::config::RAYDIUM_PROGRAM_ID;

pub async fn start_websocket_subscription() {
    let url = "wss://mainnet.helius-rpc.com/?api-key=7e92d5de-6cb0-4b4d-99cc-9174162e1d5f".parse::<Uri>().unwrap();
    
    match connect_async(url).await {
        Ok((ws_stream, _)) => {
            let (mut write, mut read) = ws_stream.split();
            
            // Правильный формат подписки для Helius
            let subscription = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "programSubscribe",
                "params": [
                    RAYDIUM_CLMM_PROGRAM_ID,  // Raydium CLMM Program ID
                    {
                        "encoding": "base64+zstd",
                        "commitment": "confirmed"
                    }
                ]
            });

            // Отправляем запрос на подписку
            if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(subscription.to_string())).await {
                error!("Failed to send subscription request: {}", e);
                return;
            }

            info!("Subscription request sent, waiting for messages...");

            // Обрабатываем входящие сообщения
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(msg) => {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                info!("Received program update: {}", json);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to connect: {}", e);
        }
    }
}