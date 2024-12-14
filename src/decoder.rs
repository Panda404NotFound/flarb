// src/decoder.rs
use log::{debug, error};
use std::error::Error;
use solana_program::pubkey::Pubkey;
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C, packed)]
pub struct WhirlpoolData {
    // Токены пула
    pub token_mint_a: Pubkey,      // 32 bytes
    pub token_mint_b: Pubkey,      // 32 bytes
    pub token_vault_a: Pubkey,     // 32 bytes
    pub token_vault_b: Pubkey,     // 32 bytes
    
    // Параметры пула
    pub fee_rate: u16,             // 2 bytes
    pub tick_spacing: u16,         // 2 bytes
    pub liquidity: u128,           // 16 bytes
    pub sqrt_price: u128,          // 16 bytes
    pub tick_current_index: i32,   // 4 bytes
    
    // Дополнительные данные
    pub protocol_fee_rate: u16,    // 2 bytes
    pub fee_growth_global_a: u128, // 16 bytes
    pub fee_growth_global_b: u128  // 16 bytes
}

pub fn decode_base64_zstd(encoded_data: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    debug!("Decoding base64+zstd data");
    
    // 1. Декодируем base64
    #[allow(deprecated)]
    let decoded = base64::decode(encoded_data)
        .map_err(|e| {
            error!("Failed to decode base64: {}", e);
            e
        })?;
    
    debug!("Successfully decoded base64, size: {} bytes", decoded.len());
    
    // 2. Распаковываем zstd
    let decompressed = zstd::decode_all(&decoded[..])
        .map_err(|e| {
            error!("Failed to decompress zstd: {}", e);
            e
        })?;
        
    debug!("Successfully decompressed data, size: {} bytes", decompressed.len());
    
    Ok(decompressed)
}

pub fn parse_whirlpool_data(data: &[u8]) -> Result<WhirlpoolData, Box<dyn Error>> {
    if data.len() < std::mem::size_of::<WhirlpoolData>() {
        error!("Data too small for WhirlpoolData");
        return Err("Insufficient data length".into());
    }

    let pool_data = bytemuck::try_from_bytes::<WhirlpoolData>(
        &data[..std::mem::size_of::<WhirlpoolData>()]
    ).map_err(|e| {
        error!("Failed to parse WhirlpoolData: {}", e);
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    })?;

    debug!("Successfully parsed WhirlpoolData");
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