// src/ws_parser.rs

use log::{debug, error, info, warn};
use serde_json::Value;
use crate::data::GLOBAL_DATA;
use crate::decoder::{decode_base64_zstd, parse_whirlpool_data};
use crate::ws_orca::WebSocketResponse;

pub async fn parse_ws_message(value: Value) -> Result<WebSocketResponse, serde_json::Error> {
    debug!("Parsing WebSocket message: {}", value);
    let response = serde_json::from_value(value)?;
    debug!("Successfully parsed WebSocket message");
    Ok(response)
}

pub fn is_subscription_success(response: &WebSocketResponse) -> bool {
    let is_success = response.result.is_some() && response.id.is_some() && response.method.is_none();
    if is_success {
        debug!("Received successful subscription confirmation with id: {:?}", response.result);
    }
    is_success
}

pub async fn handle_orca_account_update(response: WebSocketResponse) {
    if let Some(params) = response.params {
        let slot = params.result.context.slot;
        let account = params.result.value.account;
        let pubkey = params.result.value.pubkey
            .parse()
            .unwrap_or_default();
        
        info!("Processing account update for slot: {}", slot);
        
        if account.data.1 == "base64+zstd" {
            match decode_base64_zstd(&account.data.0) {
                Ok(decompressed) => {
                    match parse_whirlpool_data(&decompressed) {
                        Ok(pool_data) => {
                            // TODO: Валидация данных пула
                            // - Проверка корректности цены
                            // - Проверка ликвидности
                            // - Валидация резервов
                            
                            let price = (pool_data.sqrt_price as f64).powi(2) / 2_f64.powi(64);
                            
                            // TODO: Нормализация и валидация цены
                            // - Проверка на аномальные значения
                            // - Сравнение с историческими данными
                            
                            GLOBAL_DATA.update_pool_state(
                                pubkey,
                                (pool_data.token_vault_a.to_string().parse().unwrap_or(0),
                                 pool_data.token_vault_b.to_string().parse().unwrap_or(0)),
                                price
                            );

                            // TODO: Расчет актуальных метрик
                            // - TVL на основе текущих резервов и цен
                            // - Объем за 24ч из исторических данных
                            // - Комиссии на основе объема
                            let metrics = crate::data::PoolMetrics {
                                tvl: 0.0,
                                volume_24h: 0.0,
                                fees_24h: 0.0,
                                price_impact: 0.0
                            };
                            
                            debug!("Updating pool metrics: {:?}", metrics);
                            GLOBAL_DATA.pool_metrics.insert(pubkey, metrics);

                            // TODO: Валидация токенов перед обновлением ребер
                            if let (Some(token_a), Some(token_b)) = (
                                GLOBAL_DATA.token_addresses.get(&pool_data.token_mint_a),
                                GLOBAL_DATA.token_addresses.get(&pool_data.token_mint_b)
                            ) {
                                let edge = crate::data::LiquidityEdge {
                                    pool_address: pubkey,
                                    token_in: token_a.clone(),
                                    token_out: token_b.clone(),
                                    liquidity: pool_data.liquidity as u64,
                                    fee_rate: pool_data.fee_rate as u64
                                };
                                GLOBAL_DATA.add_liquidity_edge(edge);
                            }
                        },
                        Err(e) => error!("Failed to parse pool data: {}", e)
                    }
                },
                Err(e) => error!("Failed to decode data: {}", e)
            }
        } else {
            warn!("Unexpected data encoding: {}", account.data.1);
        }
    }
}