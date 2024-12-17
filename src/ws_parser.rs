// src/ws_parser.rs

use log::{debug, error, warn};
use serde_json::Value;
use crate::data::GLOBAL_DATA;
use solana_program::pubkey::Pubkey;
use crate::decoder::{decode_base64_zstd, parse_whirlpool_data};
use crate::ws_orca::{WebSocketResponseFinalized, WebSocketResponseProcessed, DataNotification, ProgramNotification, SlotInfo, NotificationResultFinalized, NotificationResultProcessed};
use crate::data::OrcaPoolStateBase;
use crate::data::FinalizedOrcaPoolState;
use crate::data::ProcessedOrcaPoolState;
use crate::data::unix_timestamp;
use std::time::Instant;
use flume::{Sender, Receiver};

// Добавляем enum для типов commitment
#[derive(Debug, Clone, Copy)]
pub enum PoolCommitment {
    Processed,
    Finalized,
}

// Каналы для обработки данных пулов Finalized
pub struct WebSocketChannels {
    pub program_tx: Sender<WebSocketResponseFinalized>,
    pub slot_tx: Sender<SlotInfo>,
}

// Каналы для обработки данных пулов Processed
pub struct ProcessedWebSocketChannels {
    pub program_tx: Sender<WebSocketResponseProcessed>,
}

// Каналы для получения данных пулов Processed
impl ProcessedWebSocketChannels {
    pub fn new() -> (Self, ProcessedWebSocketReceivers) {
        let (program_tx, program_rx) = flume::unbounded();
        (
            Self { program_tx },
            ProcessedWebSocketReceivers { program_rx }
        )
    }
}

// Создание каналов для обработки данных пулов Finalized
impl WebSocketChannels {
    pub fn new() -> (Self, WebSocketReceivers) {
        let (program_tx, program_rx) = flume::unbounded();
        let (slot_tx, slot_rx) = flume::unbounded();
        
        (
            Self { program_tx, slot_tx },
            WebSocketReceivers { program_rx, slot_rx }
        )
    }
}

// Канал для получения данных пулов Processed
pub struct ProcessedWebSocketReceivers {
    pub program_rx: Receiver<WebSocketResponseProcessed>,
}

// Каналы для получения данных пулов Finalized
pub struct WebSocketReceivers {
    pub program_rx: Receiver<WebSocketResponseFinalized>,
    pub slot_rx: Receiver<SlotInfo>,
}

// Парсинг сообщений из WebSocket для Finalized
pub async fn parse_ws_message(value: Value) -> Result<WebSocketResponseFinalized, serde_json::Error> {
    // debug!("Parsing WebSocket message: {}", value);
    let response = serde_json::from_value(value)?;
    // debug!("Successfully parsed WebSocket message");
    Ok(response)
}

// Парсинг сообщений из WebSocket для Processed
pub async fn parse_ws_message_processed(value: Value) -> Result<WebSocketResponseProcessed, serde_json::Error> {
    // debug!("Parsing WebSocket message: {}", value);
    let response = serde_json::from_value(value)?;
    // debug!("Successfully parsed WebSocket message");
    Ok(response)
}

// Проверка, является ли ответ успешной подпиской для Finalized
pub fn is_subscription_success(response: &WebSocketResponseFinalized) -> bool {
    let is_success = response.result.is_some() && response.id.is_some() && response.method.is_none();
    if is_success {
        debug!("Received successful subscription confirmation with id: {:?}", response.result);
    }
    is_success
}

// Проверка, является ли ответ успешной подпиской для Processed
pub fn is_subscription_success_processed(response: &WebSocketResponseProcessed) -> bool {
    let is_success = response.result.is_some() && response.id.is_some() && response.method.is_none();
    if is_success {
        debug!("Received successful subscription confirmation with id: {:?}", response.result);
    }
    is_success
}

// Обработка обновления аккаунта для Orca finalized
pub async fn handle_orca_program_update(response: WebSocketResponseFinalized, channels: &WebSocketChannels) {
    let ws_receive_time = Instant::now();
    
    // Обработка слотов
    if let Some(slot_info) = response.slot {
        // Отправляем в канал для асинхронной обработки
        let _ = channels.slot_tx.send_async(slot_info).await;
        return;
    }
    
    if let Some(params) = response.params.clone() {
        match (response.method.as_deref(), params.result) {
            // Обработка program notification
            (Some("programNotification"), NotificationResultFinalized::Program { context, value }) => {
                // Отправляем в канал для асинхронной обработки
                let _ = channels.program_tx.send_async(response.clone()).await;
                
                if let Ok(program_notification) = serde_json::from_value::<ProgramNotification>(value) {
                    let pubkey = program_notification.pubkey.parse().unwrap_or_default();
                    process_orca_account_data(
                        context.slot,
                        pubkey,
                        &program_notification.account,
                        ws_receive_time,
                        PoolCommitment::Finalized
                    ).await;
                }
            },
            
            // Обработка slot notification
            (Some("slotNotification"), NotificationResultFinalized::Slot { slot, parent, root }) => {
                let slot_info = SlotInfo {
                    slot,
                    parent,
                    root,
                };
                // Отправляем в канал для асинхронной обработки
                let _ = channels.slot_tx.send_async(slot_info).await;
            },
            
            // Неизвестный тип уведомления
            (Some(method), _) => {
                warn!("Unknown notification method: {}", method);
            },
            
            // Отсутствует метод
            (None, _) => {
                debug!("Received message without method");
            }
        }
    }
}

// Обработка обновления аккаунта для Orca processed
pub async fn handle_orca_program_update_processed(response: WebSocketResponseProcessed, channels: &ProcessedWebSocketChannels) {
    let ws_receive_time = Instant::now();
    
    if let Some(params) = response.params.clone() {
        match (response.method.as_deref(), params.result) {
            // Обработка program notification
            (Some("programNotification"), NotificationResultProcessed::Program { context, value }) => {
                // Отправляем в канал для асинхронной обработки
                let _ = channels.program_tx.send_async(response.clone()).await;
                
                if let Ok(program_notification) = serde_json::from_value::<ProgramNotification>(value) {
                    let pubkey = program_notification.pubkey.parse().unwrap_or_default();
                    process_orca_account_data(
                        context.slot,
                        pubkey,
                        &program_notification.account,
                        ws_receive_time,
                        PoolCommitment::Processed
                    ).await;
                }
            },
            // Неизвестный тип уведомления
            (Some(method), _) => {
                warn!("Unknown notification method: {}", method);
            },
            
            // Отсутствует метод
            (None, _) => {
                debug!("Received message without method");
            }
        }
    }
}

// Общая логика обработки данных аккаунта для Orca
pub async fn process_orca_account_data(
    slot: u64, 
    pubkey: Pubkey,
    account: &DataNotification,
    receive_time: Instant,
    commitment: PoolCommitment
) {
    let processing_start = Instant::now();
    // debug!("Processing account data for pool {} at slot {}", pubkey, slot);

    // Проверяем существование пула в глобальных данных
    if !GLOBAL_DATA.orca_pools.iter().any(|entry| 
        entry.value().iter().any(|pool| pool.pool_address == pubkey)
    ) {
        // warn!("Pool {} not found in GLOBAL_DATA", pubkey);
        return;
    }

    // Проверяем актуальность данных
    if !GLOBAL_DATA.validate_slot_consistency(slot) {
        // TODO: Добавить реализацию и логику для регулирования
        warn!("Skipping outdated update for pool {} at slot {}", pubkey, slot);
        // return;
    }

    if account.data.1 == "base64+zstd".to_string() {
        match decode_base64_zstd(&account.data.0) {
            Ok(decompressed) => {
                let decode_time = processing_start.elapsed();
                // debug!("Data decoded in {:?}", decode_time);

                match parse_whirlpool_data(&decompressed) {
                    Ok(pool_data) => {
                        let parse_time = processing_start.elapsed();
                        // debug!("Data parsed in {:?}", parse_time);

                        match commitment {
                            PoolCommitment::Finalized => {
                                if let Some(mut state) = GLOBAL_DATA.finalized_pool_states.get_mut(&pubkey) {
                                    // Обновляем существующее состояние
                                    state.update(&pool_data, slot);
                                } else {
                                    // Создаем base только для нового пула
                                    let base = OrcaPoolStateBase::from_whirlpool(pubkey, &pool_data);
                                    GLOBAL_DATA.finalized_pool_states.insert(pubkey, FinalizedOrcaPoolState {
                                        base,
                                        finalized_slot: slot,
                                        last_update_time: unix_timestamp(),
                                    });
                                }
                            },
                            PoolCommitment::Processed => {
                                if let Some(mut state) = GLOBAL_DATA.processed_pool_states.get_mut(&pubkey) {
                                    // Обновляем существующее состояние
                                    state.update(&pool_data, slot);
                                } else {
                                    // Создаем base только для нового пула
                                    let base = OrcaPoolStateBase::from_whirlpool(pubkey, &pool_data);
                                    GLOBAL_DATA.processed_pool_states.insert(pubkey, ProcessedOrcaPoolState {
                                        base,
                                        processed_slot: slot,
                                        last_update_time: unix_timestamp(),
                                    });
                                }
                            }
                        }
                        
                        let total_duration = receive_time.elapsed();
                        debug!("Pool {} updated. Timings: total={:?}, decode={:?}, parse={:?}, commitment={:?}", 
                               pubkey, total_duration, decode_time, parse_time, commitment);
                    },
                    Err(e) => error!("Failed to parse pool {}: {}", pubkey, e)
                }
            },
            Err(e) => error!("Failed to decode pool {}: {}", pubkey, e)
        }
    }
}