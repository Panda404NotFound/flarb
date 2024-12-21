// src/orca_ws.rs

use tokio_tungstenite::connect_async;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{info, error, warn, debug};
use tokio_tungstenite::tungstenite::http::Uri;
use crate::config::ORCA_PROGRAM_ID;
use crate::websocket::ws_parser;
use crate::data::GLOBAL_DATA;
use crate::config::CONFIG;
use std::time::Instant;
use flume::Receiver;
use crate::websocket::ws_parser::PoolCommitment;
use crate::websocket::ws_parser::process_account_data;
use crate::websocket::ws_data::
{WebSocketChannels, OrcaWebSocketChannels, DexType,
    NotificationResultFinalized, NotificationResultProcessed, 
    ProgramNotification, SlotInfo, OrcaFinalizedResponse, ProcessedResponse};

// Переименовываем основную функцию
pub async fn start_orca_websocket_finalized() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Orca WebSocket finalized subscriptions");
        
    let url = CONFIG.helius_websocket_url.parse::<Uri>()?;
    debug!("Connecting to WebSocket URL");
    
    let (ws_stream, _) = connect_async(url).await?;
    info!("Successfully connected to WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();

    // Удаляем подписки на аккаунты, оставляем только program и slot
    let program_subscription = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "programSubscribe",
        "params": [
            ORCA_PROGRAM_ID,
            {
                "encoding": "base64+zstd",
                "commitment": "finalized"
            }
        ]
    });

    let slot_subscription = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "slotSubscribe",
        "params": []
    });

    // Отправляем подписки
    debug!("Sending program subscription request");
    write.send(tokio_tungstenite::tungstenite::Message::Text(program_subscription.to_string())).await?;

    debug!("Sending slot subscription request");
    write.send(tokio_tungstenite::tungstenite::Message::Text(slot_subscription.to_string())).await?;

    info!("Successfully sent all subscription requests");

    let (channels, receivers) = OrcaWebSocketChannels::new();
    
    // Запускаем обработчики (удаляем account handler)
    tokio::spawn(handle_program_notifications(receivers.program_rx));
    tokio::spawn(handle_slot_notifications(receivers.slot_rx));

    // Обработка сообщений
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    // debug!("Received WebSocket message");
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
                                                DexType::Orca
                                            ).await;
                                        },
                                        Some("slotNotification") => {
                                            if let Some(params) = response.base.params {
                                                if let NotificationResultFinalized::Slot { slot, parent, root } = params.result {
                                                    let slot_info = SlotInfo {
                                                        slot,
                                                        parent,
                                                        root,
                                                    };
                                                    // Обновляем состояние сети
                                                    GLOBAL_DATA.update_network_state(slot_info.clone());
                                                    // Отправляем в канал для асинхронной обработки
                                                    let _ = channels.slot_tx.send_async(slot_info).await;
                                                }
                                            }
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
pub async fn start_orca_websocket_processed() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Orca WebSocket finalized subscriptions");
        
    let url = CONFIG.helius_websocket_url.parse::<Uri>()?;
    debug!("Connecting to WebSocket URL");
    
    let (ws_stream, _) = connect_async(url).await?;
    info!("Successfully connected to WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();

    // Удаляем подписки на аккаунты, оставляем только program и slot
    let program_subscription = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "programSubscribe",
        "params": [
            ORCA_PROGRAM_ID,
            {
                "encoding": "base64+zstd",
                "commitment": "processed"
            }
        ]
    });

    // Отправляем подписки
    debug!("Sending program subscription request");
    write.send(tokio_tungstenite::tungstenite::Message::Text(program_subscription.to_string())).await?;
    info!("Successfully sent all subscription requests");

    let (channels, receivers) = WebSocketChannels::new();
    
    // Запускаем обработчики (удаляем account handler)
    tokio::spawn(handle_program_notifications_processed(receivers.program_rx));

    // Обработка сообщений
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    // debug!("Received WebSocket message");
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
                                                DexType::Orca
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
                            DexType::Orca
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
                            DexType::Orca
                        ).await;
                    }
                    // info!("Program notification processed processed");
                },
            }
        }
    }
}

// Добавляем обработчик слотов
async fn handle_slot_notifications(rx: Receiver<SlotInfo>) {
    // info!("Starting slot notifications handler");
    while let Ok(slot_info) = rx.recv_async().await {
        // Валидация и логирование
        if slot_info.slot < slot_info.parent {
            warn!("Invalid slot sequence detected: slot {} is less than parent {}", 
                  slot_info.slot, slot_info.parent);
        }

        if slot_info.parent - slot_info.root > 150 {
            warn!("Large gap between parent and root slots: parent={}, root={}", 
                  slot_info.parent, slot_info.root);
        }

        GLOBAL_DATA.update_network_state(slot_info);
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
