// src/orca_ws.rs

use tokio_tungstenite::connect_async;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{info, error, warn, debug};
use tokio_tungstenite::tungstenite::http::Uri;
use crate::config::METEORA_PROGRAM_ID;
use crate::websocket::ws_parser;
use crate::config::CONFIG;
use std::time::Instant;
use flume::Receiver;
use crate::websocket::ws_parser::PoolCommitment;
use crate::websocket::ws_parser::process_account_data;
use crate::websocket::ws_data::
{WebSocketChannels, OrcaWebSocketChannels, DexType,
    NotificationResultFinalized, NotificationResultProcessed, 
    ProgramNotification, OrcaFinalizedResponse, ProcessedResponse};

// Переименовываем основную функцию
pub async fn start_meteora_websocket_finalized() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Meteora WebSocket finalized subscriptions");
        
    let url = CONFIG.helius_websocket_url.parse::<Uri>()?;
    debug!("Connecting to WebSocket URL");
    
    let (ws_stream, _) = connect_async(url).await?;
    info!("Successfully connected to WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();

    // Подписка на пулы Finalized
    let program_subscription_clmm = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "programSubscribe",
        "params": [
            METEORA_PROGRAM_ID,
            {
                "encoding": "base64+zstd",
                "commitment": "finalized"
            }
        ]
    });

    // Отправляем подписки
    debug!("Sending program subscription request");
    write.send(tokio_tungstenite::tungstenite::Message::Text(program_subscription_clmm.to_string())).await?;

    info!("Successfully sent all subscription requests");

    let (channels, receivers) = OrcaWebSocketChannels::new();
    
    // Запускаем обработчик
    tokio::spawn(handle_program_notifications(receivers.program_rx));

    // Обработка сообщений
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    // debug!("Received WebSocket message {}", text);
                    // let message_receive_time = Instant::now();
                    
                    match serde_json::from_str(&text) {
                        Ok(json) => {
                            match ws_parser::parse_ws_message::<OrcaFinalizedResponse>(json).await {
                                Ok(response) => {
                                    if ws_parser::is_subscription_success(&response) {
                                        info!("Successfully subscribed with id: {:?}", response.base.result);
                                        continue;
                                    }
                                    
                                    // let parse_duration = message_receive_time.elapsed();
                                    // debug!("Initial message parsing took: {:?}", parse_duration);
                                    
                                    match response.base.method.as_deref() {
                                        Some("programNotification") => {
                                            ws_parser::handle_program_update(
                                                response,
                                                &channels,
                                                DexType::Meteora
                                            ).await;
                                        },
                                        Some(method) => {
                                            warn!("Received unknown notification method: {}", method);
                                        },
                                        None => {
                                            debug!("Received message without method");
                                        }
                                    }
                                },
                                Err(e) => error!("Failed to parse WebSocket message: {}", e),
                            }
                        },
                        Err(e) => error!("Failed to parse JSON: {}", e),
                    }
                }
            },
            Err(e) => error!("Error receiving message: {}", e),
        }
    }

    warn!("WebSocket connection closed");
    Ok(())
}

// Обработка пулов Processed
pub async fn start_meteora_websocket_processed() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Meteora WebSocket processed subscriptions");
        
    let url = CONFIG.helius_websocket_url.parse::<Uri>()?;
    debug!("Connecting to WebSocket URL");
    
    let (ws_stream, _) = connect_async(url).await?;
    info!("Successfully connected to WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();

    // Удаляем подписки на аккаунты, оставляем только program и slot
    let program_subscription_clmm = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "programSubscribe",
        "params": [
            METEORA_PROGRAM_ID,
            {
                "encoding": "base64+zstd",
                "commitment": "processed"
            }
        ]
    });

    // Отправляем подписки
    debug!("Sending program subscription request");
    write.send(tokio_tungstenite::tungstenite::Message::Text(program_subscription_clmm.to_string())).await?;
    info!("Successfully sent all subscription requests");

    let (channels, receivers) = WebSocketChannels::new();
    
    // Запускаем обработчики (удаляем account handler)
    tokio::spawn(handle_program_notifications_processed(receivers.program_rx));

    // Обработка сообщений
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    // debug!("Received WebSocket message {}", text);
                    // let message_receive_time = Instant::now();
                    
                    match serde_json::from_str(&text) {
                        Ok(json) => {
                            match ws_parser::parse_ws_message::<ProcessedResponse>(json).await {
                                Ok(response) => {
                                    if ws_parser::is_subscription_success(&response) {
                                        info!("Successfully subscribed with id: {:?}", response.result);
                                        continue;
                                    }
                                    
                                    // let parse_duration = message_receive_time.elapsed();
                                    // debug!("Initial message parsing took: {:?}", parse_duration);
                                    
                                    match response.method.as_deref() {
                                        Some("programNotification") => {
                                            ws_parser::handle_program_update(
                                                response,
                                                &channels,
                                                DexType::Meteora
                                            ).await;
                                        },
                                        Some(method) => {
                                            warn!("Received unknown notification method: {}", method);
                                        },
                                        None => {
                                            debug!("Received message without method");
                                        }
                                    }
                                },
                                Err(e) => error!("Failed to parse WebSocket message: {}", e),
                            }
                        },
                        Err(e) => error!("Failed to parse JSON: {}", e),
                    }
                }
            },
            Err(e) => error!("Error receiving message: {}", e),
        }
    }

    warn!("WebSocket connection closed");
    Ok(())
}

// Обработчик для программы с использованием process_account_data finalized
async fn handle_program_notifications(rx: Receiver<OrcaFinalizedResponse>) {
    // info!("Starting program notifications handler");
    while let Ok(response) = rx.recv_async().await {
        if let Some(params) = response.base.params {
            match params.result {
                NotificationResultFinalized::Program { context, value } => {
                    if let Ok(program_notification) = serde_json::from_value::<ProgramNotification>(value) {
                        let pubkey = program_notification.pubkey.parse().unwrap_or_default();
                        let receive_time = Instant::now();
                        process_account_data(
                            context.slot,
                            pubkey,
                            &program_notification.account,
                            receive_time,
                            PoolCommitment::Finalized,
                            DexType::Meteora
                        ).await;
                    }
                    // info!("Program notification processed");
                },
                NotificationResultFinalized::Slot { .. } => {
                    // Игнорируем уведомления о слотах в этом обработчике
                    debug!("Received slot notification in finalized program handler");
                }
            }
        }
    }
}

// Обработчик для программы с использованием process_account_data processed
async fn handle_program_notifications_processed(rx: Receiver<ProcessedResponse>) {
    // info!("Starting program notifications handler processed");
    while let Ok(response) = rx.recv_async().await {
        if let Some(params) = response.params {
            match params.result {
                NotificationResultProcessed::Program { context, value } => {
                    if let Ok(program_notification) = serde_json::from_value::<ProgramNotification>(value) {
                        let pubkey = program_notification.pubkey.parse().unwrap_or_default();
                        let receive_time = Instant::now();
                        process_account_data(
                            context.slot,
                            pubkey,
                            &program_notification.account,
                            receive_time,
                            PoolCommitment::Processed,
                            DexType::Meteora
                        ).await;
                    }
                    // info!("Program notification processed processed");
                },
            }
        }
    }
}

// TODO: Форматы подтверждения подписки:

/*

### 1. `"commitment": "finalized"`
- Самый высокий уровень подтверждения
- Блок подтвержден суперБольшинством кластера
- Достиг максимального lockout периода
- Кластер признал этот блок финализированным
- **Использование**: 
  * Критические операции требующие 100% подтверждения
  * Финальные транзакции с деньгами
  * Когда важна безопасность, а не скорость

### 2. `"commitment": "confirmed"`
- Средний уровень подтверждения
- Блок получил голоса от суперБольшинства кластера
- Учитывает голоса из gossip и replay
- Не учитывает голоса потомков блока
- **Использование**:
  * Оптимальный баланс между скоростью и безопасностью
  * Для большинства DeFi операций
  * Когда нужна относительная уверенность

### 3. `"commitment": "processed"`
- Самый быстрый уровень подтверждения
- Самый последний блок узла
- Блок может быть пропущен кластером
- **Использование**:
  * Мониторинг в реальном времени
  * Операции не требующие подтверждений
  * Когда важна скорость, а не гарантии

### 4. Default (если не указан):
- По умолчанию используется `finalized`
- Максимальная безопасность
- Может быть медленнее других опций

### Практические рекомендации:

Для WebSocket подписок следует выбирать:

1. Для мониторинга пулов:
```json
{"commitment": "processed"} // Максимальная скорость обновлений
```

2. Для валидации транзакций:
```json
{"commitment": "confirmed"} // Баланс скорости и безопасности
```

3. Для критических операций:
```json
{"commitment": "finalized"} // Максимальная надежность
```

Таблица сравнения:
```
Level       Speed   Safety   Use Case
processed   Fast    Low      Real-time monitoring
confirmed   Medium  Medium   Most DeFi operations
finalized   Slow    High     Critical transactions
```

*/
