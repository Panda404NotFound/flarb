// src/fetch_address.rs

use std::fs::File;
use std::str::FromStr;
use std::io::BufReader;
use serde_json::Value;
use solana_program::pubkey::Pubkey;
use crate::data::GLOBAL_DATA;
use crate::config::{INITIAL_TOKENS, MIN_TVL};
use anyhow::Result;
use log::{info, debug, warn};

pub async fn start_fetching() -> Result<()> {
    info!("Начало инициализации логики");
    
    // Загружаем токены
    let tokens = load_tokens()?;
    debug!("Токены успешно загружены");
    
    // Инициализируем начальные токены
    for token in INITIAL_TOKENS.iter() {
        if let Some(token_data) = find_token_in_json(&tokens, token) {
            let address = Pubkey::from_str(&token_data.0)?;
            GLOBAL_DATA.add_token(token_data.1.clone(), address);
            debug!("Добавлен токен {} с адресом {}", token, address);
        } else {
            warn!("Токен {} не найден в JSON", token);
        }
    }
    
    // Создаем все возможные пары
    for i in 0..INITIAL_TOKENS.len() {
        for j in i+1..INITIAL_TOKENS.len() {
            GLOBAL_DATA.add_token_pair(
                INITIAL_TOKENS[i].to_string(),
                INITIAL_TOKENS[j].to_string()
            );
            debug!("Создана пара токенов: {} - {}", INITIAL_TOKENS[i], INITIAL_TOKENS[j]);
        }
    }
    
    // Загружаем и обрабатываем пулы Orca
    let orca_pools = load_orca_pools()?;
    debug!("Пулы Orca успешно загружены");
    process_orca_pools(&orca_pools)?;

    info!("Инициализация логики успешно завершена");
    Ok(())
}

fn process_orca_pools(pools: &Value) -> Result<()> {
    info!("Начало обработки пулов Orca");
    let mut processed = 0;
    let mut skipped_low_tvl = 0;
    let mut skipped_existing = 0;

    if let Some(whirlpools) = pools["whirlpools"].as_array() {
        for pool in whirlpools {
            let token_a_symbol = pool["tokenA"]["symbol"].as_str().unwrap_or_default();
            let token_b_symbol = pool["tokenB"]["symbol"].as_str().unwrap_or_default();
            
            // Проверяем, что оба токена входят в наш список интересующих токенов
            if GLOBAL_DATA.tokens.contains_key(token_a_symbol) && 
               GLOBAL_DATA.tokens.contains_key(token_b_symbol) {
                
                let tvl = pool["tvl"].as_f64().unwrap_or_default();
                
                // Пропускаем пулы с низким TVL
                if tvl < MIN_TVL {
                    skipped_low_tvl += 1;
                    continue;
                }

                let pool_address = Pubkey::from_str(
                    pool["address"].as_str().unwrap_or_default()
                )?;
                let token_a_address = Pubkey::from_str(
                    pool["tokenA"]["mint"].as_str().unwrap_or_default()
                )?;
                let token_b_address = Pubkey::from_str(
                    pool["tokenB"]["mint"].as_str().unwrap_or_default()
                )?;
                
                if GLOBAL_DATA.add_orca_pool(
                    token_a_symbol.to_string(),
                    token_b_symbol.to_string(),
                    pool_address,
                    token_a_address,
                    token_b_address,
                    tvl
                ) {
                    processed += 1;
                } else {
                    skipped_existing += 1;
                }
            }
        }
    }

    info!("Обработка пулов Orca завершена. Обработано: {}, Пропущено по TVL: {}, Пропущено существующих: {}", 
        processed, skipped_low_tvl, skipped_existing);
    Ok(())
}

fn load_tokens() -> Result<Value> {
    info!("Загрузка токенов из файла");
    let file = File::open("./pools/tokens.json")?;
    let reader = BufReader::new(file);
    let tokens: Value = serde_json::from_reader(reader)?;
    debug!("Файл токенов успешно прочитан");
    Ok(tokens)
}

fn load_orca_pools() -> Result<Value> {
    info!("Загрузка пулов Orca из файла");
    let file = File::open("./pools/orca_pools.json")?;
    let reader = BufReader::new(file);
    let pools: Value = serde_json::from_reader(reader)?;
    debug!("Файл пулов Orca успешно прочитан");
    Ok(pools)
}

fn find_token_in_json(tokens: &Value, symbol: &str) -> Option<(String, String)> {
    debug!("Поиск токена {} в JSON", symbol);
    if let Value::Array(token_list) = tokens {
        for token in token_list {
            if token["symbol"].as_str() == Some(symbol) {
                debug!("Токен {} найден", symbol);
                return Some((
                    token["address"].as_str()?.to_string(),
                    token["symbol"].as_str()?.to_string()
                ));
            }
        }
    }
    warn!("Токен {} не найден в JSON", symbol);
    None
}