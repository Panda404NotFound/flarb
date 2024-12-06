// src/jupiter/orca_pool.rs

use std::str::FromStr;
use solana_program::pubkey::Pubkey;

/// Структура для хранения информации о пуле Orca
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OrcaPool {
    /// Адрес пула - JSON: address
    pub address: Pubkey,
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
    /// Комиссия пула - JSON: lpFeeRate + protocolFeeRate
    pub fee_rate: f64,
}

#[allow(dead_code)]
impl OrcaPool {
    /// Создает новый экземпляр пула из предоставленных данных
    pub fn new(
        address: &str,
        token_a_mint: &str,
        token_b_mint: &str,
        price: f64,
        decimals_a: u8,
        decimals_b: u8,
        fee_rate: f64,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            address: Pubkey::from_str(address).map_err(|_| "Invalid pool address")?,
            token_a_mint: Pubkey::from_str(token_a_mint).map_err(|_| "Invalid token A mint")?,
            token_b_mint: Pubkey::from_str(token_b_mint).map_err(|_| "Invalid token B mint")?,
            price,
            decimals_a,
            decimals_b,
            fee_rate,
        })
    }
}