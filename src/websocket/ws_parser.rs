// src/ws_parser.rs
#[allow(unused_imports)]
use log::{debug, error, warn, info};
use serde_json::Value;
use crate::data::GLOBAL_DATA;
use solana_program::pubkey::Pubkey;
use crate::decoder::{WhirlpoolData, RaydiumData, MeteoraData};
use crate::decoder::{decode_base64_zstd, parse_whirlpool_data, parse_raydium_data, parse_meteora_data};
use crate::data::{unix_timestamp, PoolState, PoolStateBase, FinalizedPoolState, ProcessedPoolState};
use std::time::Instant;
use crate::websocket::ws_data::{
    WebSocketHandler, HasSubscriptionFields,
    HasNotificationFields, DexType,
    SlotInfo, NotificationResultFinalized,
    NotificationResultProcessed, NotificationResult,
    ProgramNotification, DataNotification
};

// Добавляем enum для типов commitment
#[derive(Debug, Clone, Copy)]
pub enum PoolCommitment {
    Processed,
    Finalized,
}

// Добавляем общий тип для всех пулов
#[derive(Debug)]
pub enum PoolData {
    Whirlpool(WhirlpoolData),
    Raydium(RaydiumData),
    Meteora(MeteoraData),
}

// Обобщенная функция парсинга для всех типов ответов
pub async fn parse_ws_message<T: serde::de::DeserializeOwned>(
    value: Value
) -> Result<T, serde_json::Error> {
    // debug!("Parsing WebSocket message: {}", value);
    serde_json::from_value(value)
}

// Обобщенная функция проверки подписки
pub fn is_subscription_success<T>(response: &T) -> bool 
where 
    T: HasSubscriptionFields,
{
    response.has_valid_subscription()
}

// Обобщенная функция обработки обновлений
pub async fn handle_program_update<T: HasNotificationFields + Clone>(
    response: T, 
    channels: &impl WebSocketHandler<T>,
    dex: DexType
) {
    // info!("Handling program update");
    let ws_receive_time = Instant::now();

    if let Some(params) = response.get_params() {
        if let Some(program_tx) = channels.get_program_tx() {
            let _ = program_tx.send_async(response.clone()).await;
        }

        match params.result {
            NotificationResult::Finalized(result) => match result {
                NotificationResultFinalized::Program { context, value } => {
                    if let Ok(program_notification) = serde_json::from_value::<ProgramNotification>(value) {
                        let pubkey = program_notification.pubkey.parse().unwrap_or_default();
                        process_account_data(
                            context.slot,
                            pubkey,
                            &program_notification.account,
                            ws_receive_time,
                            PoolCommitment::Finalized,
                            dex
                        ).await;
                    }
                    // info!("Program notification finalized received at slot {}", context.slot);
                },
                NotificationResultFinalized::Slot { slot, parent, root } => {
                    let slot_info = SlotInfo { slot, parent, root };
                    if let Some(slot_tx) = channels.get_slot_tx() {
                        let _ = slot_tx.send_async(slot_info).await;
                    }
                    info!("Slot notification received at slot {}", slot);
                }
            },
            NotificationResult::Processed(result) => match result {
                NotificationResultProcessed::Program { context, value } => {
                    if let Ok(program_notification) = serde_json::from_value::<ProgramNotification>(value) {
                        let pubkey = program_notification.pubkey.parse().unwrap_or_default();
                        process_account_data(
                            context.slot,
                            pubkey,
                            &program_notification.account,
                            ws_receive_time,
                            PoolCommitment::Processed,
                            dex
                        ).await;
                    }
                    // info!("Program notification processed received at slot {}", context.slot);
                }
            }
        }
    }
}


// Общая логика обработки данных аккаунта для
#[allow(unused_variables)]
pub async fn process_account_data(
    slot: u64, 
    pubkey: Pubkey,
    account: &DataNotification,
    receive_time: Instant,
    commitment: PoolCommitment,
    dex: DexType,
) {
    // debug!("Processing account data for pool {} at slot {}", pubkey, slot);
    let processing_start = Instant::now();
    // let receive_time = receive_time.elapsed();
    // debug!("Receive time to process_account_data: {:?}", receive_time);

    // Проверяем существование пула
    if !GLOBAL_DATA.pool_exists(dex, &pubkey) {
        // warn!("Pool {} not found in GLOBAL_DATA", pubkey);
        return;
    }
    // debug!("Pool {} found in GLOBAL_DATA", pubkey);

    // Проверяем актуальность данных
    if !GLOBAL_DATA.validate_slot_consistency(slot) {
        // TODO: Добавить реализацию и логику для регулирования
        // warn!("Skipping outdated update for pool {} at slot {}", pubkey, slot);
        // return;
    }

    if account.data.1 == "base64+zstd".to_string() {
        match decode_base64_zstd(&account.data.0) {
            Ok(decompressed) => {
                // info!("Data decoded {:?}", decompressed);
                let decode_time = processing_start.elapsed();
                // debug!("Data decoded in {:?}", decode_time);

                let parse_result: Result<PoolData, Box<dyn std::error::Error + Send + Sync>> = match dex {
                    DexType::Orca => parse_whirlpool_data(&decompressed).map(PoolData::Whirlpool),
                    DexType::Raydium => parse_raydium_data(&decompressed).map(PoolData::Raydium),
                    DexType::Meteora => parse_meteora_data(&decompressed).map(PoolData::Meteora),
                };

                match parse_result {
                    Ok(pool_data) => {
                        // info!("Pool data parsed {:?}", pool_data);
                        let parse_time = processing_start.elapsed();
                        // debug!("Data parsed in {:?}", parse_time);

                        match commitment {
                            PoolCommitment::Finalized => {
                                // info!("Updating finalized pool state for FinalizedPoolState");
                                if let Some(states) = GLOBAL_DATA.finalized_pool_states.get_mut(&dex) {
                                    // debug!("States {:?}", states);
                                    if let Some(mut state) = states.get_mut(&pubkey) {
                                        // debug!("State {:?}", state);
                                        state.update(&pool_data, slot);
                                        // info!("Pool PoolCommitment {:?} updated in {:?} for dex {:?}", commitment, processing_start.elapsed(), dex);
                                    } else {
                                        let base = PoolStateBase::from_pool_data(pubkey, &pool_data);
                                        // debug!("Base {:?}", base);
                                        states.insert(pubkey, FinalizedPoolState {
                                            base,
                                            finalized_slot: slot,
                                            last_update_time: unix_timestamp(),
                                        });
                                        // info!("Pool PoolCommitment {:?} updated in {:?} for dex {:?}", commitment, processing_start.elapsed(), dex);
                                    }
                                }
                            },
                            PoolCommitment::Processed => {
                                // info!("Updating processed pool state for ProcessedPoolState");
                                if let Some(states) = GLOBAL_DATA.processed_pool_states.get_mut(&dex) {
                                    // debug!("States {:?}", states);
                                    if let Some(mut state) = states.get_mut(&pubkey) {
                                        // debug!("State {:?}", state);
                                        state.update(&pool_data, slot);
                                        // info!("Pool PoolCommitment {:?} updated in {:?} for dex {:?}", commitment, processing_start.elapsed(), dex);
                                    } else {
                                        let base = PoolStateBase::from_pool_data(pubkey, &pool_data);
                                        // debug!("Base {:?}", base);
                                        states.insert(pubkey, ProcessedPoolState {
                                            base,
                                            processed_slot: slot,
                                            last_update_time: unix_timestamp(),
                                        });
                                        // info!("Pool PoolCommitment {:?} updated in {:?} for dex {:?}", commitment, processing_start.elapsed(), dex);
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => error!("Failed to parse pool {}: {}", pubkey, e)
                }
            },
            Err(e) => error!("Failed to decode pool {}: {}", pubkey, e)
        }
    }
}