// src/jupiter/quote.rs

use crate::test_param::TestPoolConfig;
use jup_ag::{
    quote, QuoteConfig, SwapMode, Result as JupiterResult
};
use std::env;
use tokio::time::Instant;
use log::{info, debug};

pub async fn test_valid_pools() -> JupiterResult<()> {
    let start_time = Instant::now();

    let config = TestPoolConfig::default();
    let orca_params = config.get_orca_params();
    
    // Устанавливаем URL для локального Jupiter API и инициализируем клиент
    let base_url = env::var("LOCAL_API_HOST")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    env::set_var("QUOTE_API_URL", &base_url);
    
    // Используем уже инициализированный клиент из config
    let _client = config.client;
    
    debug!("Отправляем запрос к локальному Jupiter API");

    let orca_quote = quote(
        config.sol_mint,
        config.usdc_mint,
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

    info!("Получена котировка от локального Jupiter API");
    debug!("Детали котировки: {:#?}", orca_quote);
    println!("Time taken: {:?}", start_time.elapsed());

    Ok(())
}