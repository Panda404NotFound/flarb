// src/quote.rs

use jup_ag::{
    quote, QuoteConfig, SwapMode, Result as JupiterResult
};
use log::{info, debug};
use tokio::time::Instant;
use std::time::Duration;
use anchor_lang::prelude::Pubkey;
use std::str::FromStr;

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const AMOUNT_IN: u64 = 1_000_000_000;

pub async fn test_valid_pools() -> JupiterResult<()> {

    // Проверяем что URL установлен правильно
    let api_url = std::env::var("QUOTE_API_URL")
        .expect("QUOTE_API_URL must be set");
    debug!("Using Jupiter API URL: {}", api_url);

    info!("Waiting for 2 seconds before getting quote");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let start_time_quote = Instant::now();

    let orca_quote = quote(
        Pubkey::from_str(SOL_MINT)?,
        Pubkey::from_str(USDC_MINT)?,
        AMOUNT_IN,
        QuoteConfig {
            slippage_bps: Some(50),
            swap_mode: Some(SwapMode::ExactIn),
            only_direct_routes: true,
            ..Default::default()
        },
    ).await?;

    debug!("Получена котировка, детали: {:#?}", orca_quote);
    debug!("Latency получения котировки: {:?}", start_time_quote.elapsed());
    Ok(())
}