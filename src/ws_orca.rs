// src/orca_ws.rs

use tokio_tungstenite::connect_async;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{info, error, warn, debug};
use tokio_tungstenite::tungstenite::http::Uri;
use crate::config::ORCA_PROGRAM_ID;
use crate::ws_parser;
use crate::data::GLOBAL_DATA;
use crate::config::CONFIG;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebSocketResponse {
    pub method: Option<String>,
    pub params: Option<NotificationParams>,
    pub result: Option<u64>,
    pub id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct NotificationParams {
    pub result: NotificationResult,
}

#[derive(Debug, Deserialize)]
pub struct NotificationResult {
    pub context: Context,
    pub value: AccountInfo,
}

#[derive(Debug, Deserialize)]
pub struct Context {
    pub slot: u64,
}

#[derive(Debug, Deserialize)]
pub struct AccountInfo {
    pub pubkey: String,
    pub account: Account,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub data: (String, String), // (data, encoding)
}

// Основная функция подписки
pub async fn start_orca_websocket() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Orca WebSocket subscriptions");
        
    let url = CONFIG.helius_websocket_url.parse::<Uri>()?;
    debug!("Connecting to WebSocket URL");
    
    let (ws_stream, _) = connect_async(url).await?;
    info!("Successfully connected to WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();

    // Подписка на аккаунты пулов
    let account_subscription = {
        let mut pool_addresses = Vec::new();
        for entry in GLOBAL_DATA.orca_pools.iter() {
            for pool in entry.value() {
                pool_addresses.push(pool.pool_address.to_string());
                debug!("Adding pool address to subscription: {}", pool.pool_address);
            }
        }
        
        // Изменяем формат - передаем каждый адрес отдельной подпиской
        pool_addresses.iter().enumerate().map(|(index, address)| {
            json!({
                "jsonrpc": "2.0",
                "id": index + 1, // Уникальный id для каждой подписки
                "method": "accountSubscribe",
                "params": [
                    address,  // Передаем один адрес вместо массива
                    {
                        "encoding": "base64+zstd",
                        "commitment": "confirmed"
                    }
                ]
            })
        }).collect::<Vec<_>>()
    };

    // Подписка на программу
    let program_subscription = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "programSubscribe",
        "params": [
            ORCA_PROGRAM_ID,
            {
                "encoding": "base64+zstd",
                "commitment": "confirmed"
            }
        ]
    });

    // Отправляем подписки на аккаунты
    for subscription in account_subscription {
        debug!("Sending account subscription request for address");
        write.send(tokio_tungstenite::tungstenite::Message::Text(subscription.to_string())).await?;
    }

    // Отправляем подписку на программу
    debug!("Sending program subscription request");
    write.send(tokio_tungstenite::tungstenite::Message::Text(program_subscription.to_string())).await?;

    info!("Successfully sent all subscription requests");

    // Обработка сообщений
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    debug!("Received WebSocket message");
                    
                    match serde_json::from_str(&text) {
                        Ok(json) => {
                            match ws_parser::parse_ws_message(json).await {
                                Ok(response) => {
                                    if ws_parser::is_subscription_success(&response) {
                                        info!("Successfully subscribed with id: {:?}", response.result);
                                        continue;
                                    }
                                    
                                    match response.method.as_deref() {
                                        Some("accountNotification") => {
                                            debug!("Processing account notification");
                                            ws_parser::handle_orca_account_update(response).await;
                                        },
                                        Some("programNotification") => {
                                            debug!("Processing program notification");
                                            ws_parser::handle_orca_account_update(response).await;
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
