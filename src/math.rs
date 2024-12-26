// src/math.rs
use solana_program::pubkey::Pubkey;
use log::debug;

// Структура для хранения метрик пула
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub price: f64,
    pub liquidity: f64,
    pub fee_rate: f64,
}

// Специфичные калькуляторы для каждого DEX
pub mod calculators {
    pub fn calculate_orca_price(sqrt_price: u128) -> f64 {
        let price = sqrt_price as f64;
        price * price / 2.0f64.powi(128)
    }

    pub fn calculate_raydium_price(min_price: u64, max_price: u64) -> f64 {
        (min_price + max_price) as f64 / 2.0
    }

    pub fn calculate_meteora_price(sqrt_price: u128) -> f64 {
        let price = sqrt_price as f64;
        price * price / 2.0f64.powi(128)
    }

    pub fn calculate_raydium_fee(numerator: u64, denominator: u64) -> f64 {
        if denominator == 0 {
            0.0
        } else {
            numerator as f64 / denominator as f64
        }
    }
}

// Функция для обновления метрик пула
pub fn update_pool_metrics(
    pool_address: Pubkey,
    new_metrics: PoolMetrics,
    old_metrics: Option<&PoolMetrics>,
) -> bool {
    let mut updated = false;

    if let Some(old) = old_metrics {
        // Проверяем существенные изменения в метриках
        if (new_metrics.price - old.price).abs() / old.price > 0.001 {
            debug!("Pool {} price changed by {:.2}%", 
                   pool_address, 
                   (new_metrics.price - old.price) * 100.0 / old.price);
            updated = true;
        }

        if (new_metrics.liquidity - old.liquidity).abs() / old.liquidity > 0.01 {
            debug!("Pool {} liquidity changed by {:.2}%",
                   pool_address,
                   (new_metrics.liquidity - old.liquidity) * 100.0 / old.liquidity);
            updated = true;
        }

        if (new_metrics.fee_rate - old.fee_rate).abs() > f64::EPSILON {
            debug!("Pool {} fee rate changed: {} -> {}", 
                   pool_address, old.fee_rate, new_metrics.fee_rate);
            updated = true;
        }
    } else {
        // Если это новые метрики, считаем что обновление произошло
        updated = true;
    }

    updated
}

// Расчет весов для пулов
pub mod weight_calculators {
    // Базовый расчет веса для всех DEX
    fn calculate_base_weight(price: f64, fee_rate: f64, liquidity: f64) -> f64 {
        let fee_factor = 1.0 - fee_rate;
        let liquidity_factor = (liquidity / 1_000_000.0).min(1.0); // Нормализуем к 1M
        
        price * fee_factor * liquidity_factor
    }

    // Расчет веса для Orca
    pub fn calculate_orca_weight(
        price: f64,
        fee_rate: f64,
        liquidity: f64,
        tick_spacing: u16
    ) -> f64 {
        let base = calculate_base_weight(price, fee_rate, liquidity);
        // Меньший tick_spacing = лучшая точность цены
        let precision_factor = 1.0 - (tick_spacing as f64 / 100.0).min(0.5);
        base * precision_factor
    }

    // Расчет веса для Raydium
    #[allow(unused_variables)]
    pub fn calculate_raydium_weight(
        price: f64,
        fee_rate: f64,
        liquidity: f64,
        orders_num: u64,
        depth: u64
    ) -> f64 {
        let base = calculate_base_weight(price, fee_rate, liquidity);
        // Учитываем глубину книги ордеров
        let depth_factor = (depth as f64 / 1_000_000.0).min(1.0);
        base * (1.0 + depth_factor)
    }

    // Расчет веса для Meteora
    pub fn calculate_meteora_weight(
        price: f64,
        fee_rate: f64,
        liquidity: f64,
        multiplier: u64
    ) -> f64 {
        let base = calculate_base_weight(price, fee_rate, liquidity);
        // Учитываем множитель ликвидности
        let multiplier_factor = (multiplier as f64 / 100.0).min(2.0);
        base * (1.0 + multiplier_factor)
    }
}