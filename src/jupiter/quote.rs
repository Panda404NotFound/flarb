// src/jupiter/quote.rs

use crate::config;
use crate::test_param::TestPoolConfig;
use jup_ag::{
    quote, QuoteConfig, route_map, price,
    SwapMode, Result as JupiterResult
};
use log::{info, debug};
use tokio::time::Instant;
use std::time::Duration;
use crate::config::{INITIALIZE_HTTP_CLIENT, DEFAULT_QUOTE_API_URL};

pub async fn test_valid_pools() -> JupiterResult<()> {
    let start_time = Instant::now();

    let config = TestPoolConfig::default();
    let orca_params = config.get_orca_params();
    
    // Получаем URL в зависимости от настроек
    let api_url = if INITIALIZE_HTTP_CLIENT {
        std::env::var("QUOTE_API_URL")
            .expect("QUOTE_API_URL must be set")
    } else {
        DEFAULT_QUOTE_API_URL.to_string()
    };
    debug!("Using Jupiter API URL: {}", api_url);
    debug!("Latency подключения и проверки: {:?}", start_time.elapsed());

    let start_time_quote = Instant::now();

    let orca_quote = quote(
        config.sol_mint,
        config.usdc_mint,
        orca_params.amount_in,
        QuoteConfig {
            slippage_bps: Some(orca_params.slippage_bps),
            swap_mode: Some(SwapMode::ExactIn),
            only_direct_routes: false,
            ..Default::default()
        },
    ).await?;

    debug!("Получена котировка, детали: {:#?}", orca_quote);
    debug!("Latency получения котировки: {:?}", start_time_quote.elapsed());
    Ok(())
}