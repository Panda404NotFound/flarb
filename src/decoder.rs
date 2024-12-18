// src/decoder.rs
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::error::Error;
#[allow(unused_imports)]
use std::time::Instant;
use solana_program::pubkey::Pubkey;
use bytemuck::{Pod, Zeroable};
use bytes::BytesMut;
use rayon::prelude::*;

// Константы для оптимизации размеров чанков
// const CHUNK_SIZE: usize = 1024 * 64; // Размер чанка для параллельной обработки 64KB 
// const MIN_PARALLEL_SIZE: usize = 1024 * 256; // Минимальный размер для параллельной обработки 256KB
const CHUNK_SIZE: usize = 512; // Размер чанка для параллельной обработки 512 байт
const MIN_PARALLEL_SIZE: usize = 2048; // Минимальный размер для параллельной обработки 2KB

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C, packed)]
pub struct WhirlpoolData {
    // Базовые параметры пула
    pub token_mint_a: Pubkey,       // 32 bytes
    pub token_vault_a: Pubkey,      // 32 bytes
    pub token_mint_b: Pubkey,       // 32 bytes
    pub token_vault_b: Pubkey,      // 32 bytes
    pub tick_spacing: u16,          // 2 bytes
    pub fee_rate: u16,              // 2 bytes
    pub protocol_fee_rate: u16,     // 2 bytes
    pub liquidity: u128,            // 16 bytes

    // Ценовые параметры
    pub sqrt_price: u128,           // 16 bytes
    pub tick_current_index: i32,    // 4 bytes
    pub price_threshold: u64,       // 8 bytes
    pub fee_growth_global_a: u128,  // 16 bytes
    pub fee_growth_global_b: u128,  // 16 bytes

    // Протокольные параметры
    pub protocol_fee_owed_a: u64,   // 8 bytes
    pub protocol_fee_owed_b: u64,   // 8 bytes
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C, packed)]
pub struct RaydiumData {
    // Базовые параметры пула
    pub status: u64,                   // 8 bytes
    pub pool_state: u64,               // 8 bytes
    pub amm_id: Pubkey,               // 32 bytes
    pub market_id: Pubkey,            // 32 bytes
    
    // Токены и ликвидность
    pub token_a: Pubkey,              // 32 bytes 
    pub token_b: Pubkey,              // 32 bytes
    pub lp_mint: Pubkey,              // 32 bytes
    pub open_orders: Pubkey,          // 32 bytes
    
    // Параметры пула
    pub needs_withdraw: u64,          // 8 bytes
    pub recent_slot: u64,             // 8 bytes
    pub last_order_slot: u64,         // 8 bytes
    pub total_lp: u64,                // 8 bytes
    pub base_need_take: u64,          // 8 bytes
    pub quote_need_take: u64,         // 8 bytes
    
    // Ценовые параметры  
    pub base_decimal: u64,            // 8 bytes
    pub quote_decimal: u64,           // 8 bytes
    pub min_price: u64,               // 8 bytes
    pub max_price: u64,               // 8 bytes
    pub vol_max_cut_ratio: u64,       // 8 bytes
    
    // Параметры комиссий
    pub fee_numerator: u64,           // 8 bytes 
    pub fee_denominator: u64,         // 8 bytes
    pub ret_fee_numerator: u64,       // 8 bytes
    pub ret_fee_denominator: u64,     // 8 bytes
    
    // Дополнительные параметры
    pub punish_pc_amount: u64,        // 8 bytes
    pub punish_coin_amount: u64,      // 8 bytes
    pub orders_num: u64,              // 8 bytes
    pub depth: u64,                   // 8 bytes
    pub open_time: u64,               // 8 bytes
    pub switch_time: u64,             // 8 bytes
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C, packed)]
pub struct MeteoraData {
    // Основные параметры пула
    pub pool_id: Pubkey,               // 32 bytes
    pub authority: Pubkey,             // 32 bytes
    pub token_mint_a: Pubkey,          // 32 bytes
    pub token_mint_b: Pubkey,          // 32 bytes
    pub token_vault_a: Pubkey,         // 32 bytes
    pub token_vault_b: Pubkey,         // 32 bytes
    pub lp_mint: Pubkey,               // 32 bytes
    
    // Параметры ликвидности
    pub total_lp: u64,                 // 8 bytes
    pub liquidity: u128,               // 16 bytes
    pub sqrt_price: u128,              // 16 bytes
    pub current_tick_index: i32,       // 4 bytes
    pub tick_spacing: u16,             // 2 bytes
    pub fee_rate: u16,                 // 2 bytes
    
    // Параметры DLMM
    pub protocol_fee_rate: u16,        // 2 bytes
    
    // Накопленные комиссии и метрики
    pub fee_growth_global_a: u128,     // 16 bytes
    pub fee_growth_global_b: u128,     // 16 bytes
    pub fee_protocol_token_a: u64,     // 8 bytes 
    pub fee_protocol_token_b: u64,     // 8 bytes
    
    // Диапазоны цен и тиков
    pub max_price_sqrt: u128,          // 16 bytes
    pub min_price_sqrt: u128,          // 16 bytes
    pub max_tick_index: i32,           // 4 bytes 
    pub min_tick_index: i32,           // 4 bytes
    
    // Параметры динамической ликвидности
    pub dynamic_liquidity_mode: u8,    // 1 byte
    pub liquidity_cap: u128,           // 16 bytes
    pub liquidity_multiplier: u64,     // 8 bytes
    
    // Метаданные и статистика
    pub last_update_timestamp: i64,    // 8 bytes
    pub last_update_slot: u64,         // 8 bytes
    pub volume_24h: u64,               // 8 bytes
    pub fees_24h: u64,                 // 8 bytes
}

// Оптимизированный пул буферов
thread_local! {
    static DECODE_BUFFER: std::cell::RefCell<BytesMut> = std::cell::RefCell::new(BytesMut::with_capacity(1024 * 32));
    static BUFFER_POOL: std::cell::RefCell<Vec<BytesMut>> = std::cell::RefCell::new(Vec::new());
}

// Декодирование base64+zstd данных
pub fn decode_base64_zstd(encoded_data: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    // let decoding_time = Instant::now();
    // debug!("Starting base64+zstd decoding");
    // debug!("Input data length: {} chars", encoded_data.len());

    // Выбираем оптимальную стратегию декодирования
    let result = if encoded_data.len() >= MIN_PARALLEL_SIZE {
        decode_parallel(encoded_data)
    } else {
        decode_sequential(encoded_data)
    };

    match result {
        Ok(decompressed) => {
            // let decompression_duration = decoding_time.elapsed();
            // info!("ZSTD decompressed successfully, time: {:?}", decompression_duration);
            // debug!("Decompressed size: {} bytes", decompressed.len());
            
            if decompressed.len() < std::mem::size_of::<WhirlpoolData>() {
                warn!("Decompressed data might be too small for WhirlpoolData");
            }
            
            Ok(decompressed)
        }
        Err(e) => {
            error!("Decoding failed: {}", e);
            Err(e)
        }
    }
}

// Последовательное декодирование
fn decode_sequential(encoded_data: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    DECODE_BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.clear();
        
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        #[allow(deprecated)]
        let decoded = base64::decode(encoded_data)
            .map_err(|e| {
                error!("Failed to decode base64: {}", e);
                e
            })?;
        
        buffer.extend_from_slice(&decoded);
        
        zstd::decode_all(&buffer[..])
            .map_err(|e| {
                error!("Failed to decompress zstd: {}", e);
                Box::new(e) as Box<dyn Error + Send + Sync>
            })
    })
}

// Параллельное декодирование
fn decode_parallel(encoded_data: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let chunks: Vec<&str> = encoded_data
        .as_bytes()
        .chunks(CHUNK_SIZE)
        .map(|chunk| unsafe { std::str::from_utf8_unchecked(chunk) })
        .collect();

    let decoded: Result<Vec<Vec<u8>>, _> = chunks
        .par_iter()
        .map(|chunk| {
            #[allow(deprecated)]
            base64::decode(chunk)
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
        })
        .collect();

    let decoded = decoded?;
    let mut combined = Vec::with_capacity(decoded.iter().map(|v| v.len()).sum());
    decoded.iter().for_each(|chunk| combined.extend_from_slice(chunk));

    zstd::decode_all(&combined[..])
        .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
}

// Парсинг данных пула для Whirlpool
pub fn parse_whirlpool_data(data: &[u8]) -> Result<WhirlpoolData, Box<dyn Error + Send + Sync>> {
    // let parsing_time = Instant::now();
    if data.len() < std::mem::size_of::<WhirlpoolData>() {
        error!("Data too small for WhirlpoolData: got {} bytes, need {}", 
               data.len(), std::mem::size_of::<WhirlpoolData>());
        return Err("Insufficient data length".into());
    }

    let pool_data = bytemuck::try_from_bytes::<WhirlpoolData>(
        &data[..std::mem::size_of::<WhirlpoolData>()]
    ).map_err(|e| {
        error!("Failed to parse WhirlpoolData: {}", e);
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())) as Box<dyn Error + Send + Sync>
    })?;

    // info!("Parsed WhirlpoolData: {:?}", pool_data);

    // let parsing_duration = parsing_time.elapsed();
    // debug!("Whirlpool data parsing took: {:?}", parsing_duration);

    // ДАННЫЕ ЛОГИРОВАНИЯ МОЖНО БЕЗОПАСНО УДАЛИТЬ
    /*
    info!("Successfully decoded Whirlpool data:");
    info!("Token A: {}", pool_data.token_mint_a);
    info!("Token B: {}", pool_data.token_mint_b);
    info!("Vault A: {}", pool_data.token_vault_a);
    info!("Vault B: {}", pool_data.token_vault_b);
    
    // Копируем значения в локальные переменные для безопасного доступа
    let fee_rate = pool_data.fee_rate;
    let tick_spacing = pool_data.tick_spacing;
    let liquidity = pool_data.liquidity;
    let sqrt_price = pool_data.sqrt_price;
    let tick_current_index = pool_data.tick_current_index;
    let protocol_fee_rate = pool_data.protocol_fee_rate;
    let fee_growth_global_a = pool_data.fee_growth_global_a;
    let fee_growth_global_b = pool_data.fee_growth_global_b;
    
    debug!("Pool parameters:");
    debug!("Fee rate: {}", fee_rate);
    debug!("Tick spacing: {}", tick_spacing);
    debug!("Liquidity: {}", liquidity);
    debug!("Sqrt price: {}", sqrt_price);
    debug!("Current tick index: {}", tick_current_index);
    
    debug!("Additional data:");
    debug!("Protocol fee rate: {}", protocol_fee_rate);
    debug!("Fee growth global A: {}", fee_growth_global_a);
    debug!("Fee growth global B: {}", fee_growth_global_b);

    // Расчет реальной цены из sqrt_price (используем локальную переменную)
    let real_price = (sqrt_price as f64).powi(2) / 2_f64.powi(64);
    info!("Calculated real price: {}", real_price);

    // Проверка валидности данных (используем локальные пере��енные)
    if liquidity == 0 {
        warn!("Pool has zero liquidity");
    }
    if fee_rate > 10000 { // 100% = 10000
        warn!("Unusually high fee rate: {}", fee_rate);
    }
    */
    // ДАННЫЕ ЛОГИРОВАНИЯ МОЖНО БЕЗОПАСНО УДАЛИТЬ

    Ok(*pool_data)
}

// Парсинг данных пула для Raydium
pub fn parse_raydium_data(data: &[u8]) -> Result<RaydiumData, Box<dyn Error + Send + Sync>> {
    // let parsing_time = Instant::now();
    if data.len() < std::mem::size_of::<RaydiumData>() {
        error!("Data too small for RaydiumData: got {} bytes, need {}", 
               data.len(), std::mem::size_of::<RaydiumData>());
        return Err("Insufficient data length".into());
    }

    let pool_data = bytemuck::try_from_bytes::<RaydiumData>(
        &data[..std::mem::size_of::<RaydiumData>()]
    ).map_err(|e| {
        error!("Failed to parse RaydiumPoolState: {}", e);
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())) as Box<dyn Error + Send + Sync>
    })?;

    Ok(*pool_data)
}

// Парсинг данных пула для Meteora
pub fn parse_meteora_data(data: &[u8]) -> Result<MeteoraData, Box<dyn Error + Send + Sync>> {
    // let parsing_time = Instant::now();
    if data.len() < std::mem::size_of::<MeteoraData>() {
        error!("Data too small for MeteoraData: got {} bytes, need {}", 
               data.len(), std::mem::size_of::<MeteoraData>());
        return Err("Insufficient data length".into());
    }

    let pool_data = bytemuck::try_from_bytes::<MeteoraData>(
        &data[..std::mem::size_of::<MeteoraData>()]
    ).map_err(|e| {
        error!("Failed to parse MeteoraData: {}", e);
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())) as Box<dyn Error + Send + Sync>
    })?;

    Ok(*pool_data)
}

// TODO: Реализовать декодеры для других DEX:
// - Raydium CLMM decoder
// - Meteora decoder
// - Raydium V4 decoder

// TODO: Добавить структуры для хранения специфичных данных каждого DEX:
// - Raydium pool state
// - Meteora pool state
// - Raydium V4 pool state

// TODO: Реализовать конвертацию в общий формат для глобального хранения:
// - Конвертация цен в стандартный формат
// - Нормализация ликвидности
// - Расчет метрик (TVL, объем, комиссии)

// TODO: Оптимизировать декодирование:
// - Кэширование часто используемых данных
// - Параллельное декодирование
// - Zero-copy десериализация где возможно