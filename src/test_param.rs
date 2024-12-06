// src/test_param.rs

// Формат ответа. Может быть с несколькими RoutePlan

/*
Quote {
    // Адрес токена, который отправляем (в данном случае SOL)
    input_mint: So11111111111111111111111111111111111111112,
    
    // Количество входного токена в минимальных единицах (1 SOL = 1_000_000_000 лампортов)
    in_amount: 1000000000,
    
    // Адрес токена, который получаем (в данном случае USDC)
    output_mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v,
    
    // Ожидаемое количество выходного токена (≈240.22 USDC)
    out_amount: 240216686,
    
    // Минимальное гарантированное количество с учетом проскальзывания (≈239.02 USDC)
    other_amount_threshold: 239015603,
    
    // Режим свопа: ExactIn означает точное входное количество
    swap_mode: "ExactIn",
    
    // Допустимое проскальзывание в базисных пунктах (0.5%)
    slippage_bps: 50,
    
    // Комиссия платформы Jupiter (в данном случае отсутствует)
    platform_fee: None,
    
    // Процент влияния сделки на цену в пуле
    price_impact_pct: 0.0,
    
    // Массив с планом маршрутизации сделки
    route_plan: [
        RoutePlan {
            swap_info: SwapInfo {
                // Адрес пула AMM для свопа
                amm_key: 71GHcjkwmtM7HSWBuqzjEp96prcNNwu1wpUywXiytREU,
                
                // Название протокола DEX
                label: Some("Lifinity V2"),
                
                // Адреса входного и выходного токенов для данного этапа маршрута
                input_mint: So11111111111111111111111111111111111111112,
                output_mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v,
                
                // Количества входного и выходного токенов для данного этапа
                in_amount: 1000000000,
                out_amount: 240216686,
                
                // Комиссия пула (0.0001 SOL)
                fee_amount: 100000,
                
                // Токен, в котором взимается комиссия (SOL)
                fee_mint: So11111111111111111111111111111111111111112,
            },
            // Процент от общего объема, который идет через данный маршрут
            percent: 100,
        },
    ],
    
    // Номер слота Solana, в котором получена котировка
    context_slot: Some(305571766),
    
    // Время выполнения запроса в секундах
    time_taken: Some(0.004383402),
} 
*/


use solana_program::pubkey::Pubkey;
use std::str::FromStr;

// Общие параметры для DEX
pub struct TestDexParams {
    pub slippage_bps: u64,
    pub amount_in: u64,
    pub amount_out: u64,
}

// Специфичные параметры Orca
pub struct TestOrcaDexConfig {
    pub pool_address: Pubkey,
    pub params: TestDexParams,
    pub fee_rate: f64,
}

// Специфичные параметры Raydium
pub struct TestRaydiumDexConfig {
    pub pool_id: Pubkey,
    pub amm_config: Pubkey,
    pub trade_fee_rate: f64,
    pub params: TestDexParams,
}

pub struct TestPoolConfig {
    pub orca: TestOrcaDexConfig,
    pub raydium: TestRaydiumDexConfig,
    pub usdc_mint: Pubkey,
    pub sol_mint: Pubkey,
}

impl Default for TestPoolConfig {
    fn default() -> Self {
        // Базовые параметры для обеих DEX
        let orca_params = TestDexParams {
            slippage_bps: 50,  // 0.5%
            amount_in: 1_000_000_000,  // 1 SOL (9 decimals)
            amount_out: 1_000_000,     // 1 USDC (6 decimals)
        };

        let raydium_params = TestDexParams {
            slippage_bps: 50,  // 0.5%
            amount_in: 1_000_000,      // 1 USDC (6 decimals)
            amount_out: 1_000_000_000, // 1 SOL (9 decimals)
        };

        Self {
            orca: TestOrcaDexConfig {
                // Адрес пула Orca SOL/USDC
                pool_address: Pubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE")
                    .expect("Invalid Orca pool address"),
                params: orca_params,
                fee_rate: 0.0005, // lpFeeRate (0.0004) + protocolFeeRate (0.0001) = 0.0005
            },
            raydium: TestRaydiumDexConfig {
                // Вы заполните эти адреса
                pool_id: Pubkey::from_str("12DEJAmaxj58FYFAF6uAfP2v3256CCf1ZwAQLNvBP7EC")
                    .expect("Invalid Raydium pool ID"),
                amm_config: Pubkey::from_str("3XCQJQryqpDvvZBfGxR7CLAw5dpGJ9aa7kt1jRLdyxuZ")
                    .expect("Invalid Raydium AMM config"),
                trade_fee_rate: 0.0005, // 0.05%
                params: raydium_params,
            },
            usdc_mint: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
                .expect("Invalid USDC mint"),
            sol_mint: Pubkey::from_str("So11111111111111111111111111111111111111112")
                .expect("Invalid SOL mint"),
        }
    }
}

impl TestPoolConfig {
    // Вспомогательные методы для получения параметров
    pub fn get_orca_params(&self) -> &TestDexParams {
        &self.orca.params
    }

    pub fn get_raydium_params(&self) -> &TestDexParams {
        &self.raydium.params
    }
}