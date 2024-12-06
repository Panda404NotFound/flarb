// src/jupiter/rayduim_pool.rs

use std::str::FromStr;
use solana_program::pubkey::Pubkey;

/// Структура для хранения информации о пуле Raydium
#[derive(Debug, Clone)]
pub struct RaydiumPool {
    /// ID пула - JSON: id
    pub pool_id: Pubkey,
    /// Адрес первого токена в пуле - JSON: tokenA.mint
    pub token_a_mint: Pubkey,
    /// Адрес второго токена в пуле - JSON: tokenB.mint
    pub token_b_mint: Pubkey,
    /// Текущая цена токена A относительно токена B - JSON: price
    pub price: f64,
    /// Кол-во знаков для токена A - JSON: tokenA.decimals
    pub decimals_a: u8,
    /// Кол-во знаков для токена B - JSON: tokenB.decimals
    pub decimals_b: u8,
    /// Конфигурация AMM пула - JSON: ammConfig.id
    pub amm_config: Pubkey,
    /// Комиссия за торговлю (в процентах) - JSON: ammConfig.tradeFeeRate
    pub trade_fee_rate: f64,
}

impl RaydiumPool {
    /// Создает новый экземпляр пула из предоставленных данных
    pub fn new(
        pool_id: &str,
        token_a_mint: &str,
        token_b_mint: &str,
        price: f64,
        decimals_a: u8,
        decimals_b: u8,
        amm_config: &str,
        trade_fee_rate: f64,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            pool_id: Pubkey::from_str(pool_id).map_err(|_| "Invalid pool ID")?,
            token_a_mint: Pubkey::from_str(token_a_mint).map_err(|_| "Invalid token A mint")?,
            token_b_mint: Pubkey::from_str(token_b_mint).map_err(|_| "Invalid token B mint")?,
            price,
            decimals_a,
            decimals_b,
            amm_config: Pubkey::from_str(amm_config).map_err(|_| "Invalid AMM config")?,
            trade_fee_rate,
        })
    }
}