// src/jupiter/quote.rs

/*
TODO:
Параметры Jupiter Quote API и их соответствие в наших структурах:

inputMint:   token_a_mint или token_b_mint - адрес входного токена
            (например: "So11111111111111111111111111111111111111112" для SOL)
            
outputMint:  token_a_mint или token_b_mint - адрес выходного токена
            (например: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" для USDC)
            
amount:     количество входного токена с учетом decimals
           (decimals_a или decimals_b в зависимости от направления)
           Например: SOL = 9 decimals, USDC = 6 decimals
           
slippageBps: допустимое проскальзывание в базисных пунктах (1 bps = 0.01%)
            (опциональный параметр)

Примеры значений из реальных пулов:
- Orca SOL/USDC:   price ≈ 227.39, fee_rate = 0.05% (0.0004 + 0.0001)
- Raydium SOL/USDC: tradeFeeRate = 0.05% (500 = 0.05%)
*/


// src/jupiter/quote.rs

use crate::test_param::TestPoolConfig;
use tokio::time::Instant;
use jup_ag::{
    quote,
    QuoteConfig,
    SwapMode,
    Result as JupiterResult,
};

pub async fn test_valid_pools() -> JupiterResult<()> {
    let start_time = Instant::now();
    let config = TestPoolConfig::default();
    
    // Тестируем Orca пул SOL -> USDC
    let orca_params = config.get_orca_params();
    
    // Создаем запрос для котировки Orca строго по документации
    let orca_quote = quote(
        config.sol_mint,    // input_mint
        config.usdc_mint,   // output_mint
        orca_params.amount_in,
        QuoteConfig {
            slippage_bps: Some(orca_params.slippage_bps),
            swap_mode: Some(SwapMode::ExactIn),
            dexes: None,
            exclude_dexes: None, 
            only_direct_routes: false,
            as_legacy_transaction: None,
            platform_fee_bps: None,
            max_accounts: None,
        },
    ).await?;

    println!("Orca SOL -> USDC Quote: {:#?}", orca_quote);
    println!("Time taken: {:?}", start_time.elapsed());

    /*
    // Тестируем Raydium пул USDC -> SOL
    let raydium_params = config.get_raydium_params();
    
    let raydium_quote = quote(
        config.usdc_mint,   // input_mint
        config.sol_mint,    // output_mint
        raydium_params.amount_in,
        QuoteConfig {
            slippage_bps: Some(raydium_params.slippage_bps),
            swap_mode: Some(SwapMode::ExactIn),
            dexes: None,
            exclude_dexes: None,
            only_direct_routes: false,
            as_legacy_transaction: None,
            platform_fee_bps: None,
            max_accounts: None,
        },
    ).await?;

    println!("Raydium USDC -> SOL Quote: {:#?}", raydium_quote);
    */

    Ok(())
}