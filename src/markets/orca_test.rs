// src/markets/orca_test.rs

use {
    crate::markets::whirlpool::{
        WhirlpoolAmm, QuoteParams, SwapMode, Quote,
        TickCache,
    },
    solana_program::{
        pubkey::Pubkey,
        account_info::AccountInfo,
    },
    solana_client::rpc_client::RpcClient,
    anchor_lang::prelude::Pubkey as AnchorPubkey,
    std::str::FromStr,
    anyhow::{Result, Context},
    log::{info, error},
};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const WHIRLPOOL_SOL_USDC: &str = "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ";

#[allow(dead_code)]
struct WhirlpoolTest {
    rpc_client: RpcClient,
    cache: TickCache,
    whirlpool: Option<WhirlpoolAmm>,
}

impl WhirlpoolTest {
    fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            cache: TickCache::new(60),
            whirlpool: None,
        }
    }

    async fn initialize(&mut self) -> Result<()> {
        let pool_pubkey = Pubkey::from_str(WHIRLPOOL_SOL_USDC)
            .context("Failed to parse pool pubkey")?;

        info!("Получение данных аккаунта пула {}...", pool_pubkey);
        let account = self.rpc_client.get_account(&AnchorPubkey::new_from_array(pool_pubkey.to_bytes()))
            .context("Failed to get pool account")?;

        let mut lamports = account.lamports;
        let mut data = account.data.clone();
        let owner = Pubkey::new_from_array(account.owner.to_bytes());
        let account_info = AccountInfo::new(
            &pool_pubkey,
            false,
            false,
            &mut lamports,
            &mut data,
            &owner,
            account.executable,
            account.rent_epoch,
        );
        info!("Инициализация WhirlpoolAmm...");
        self.whirlpool = Some(WhirlpoolAmm::new(pool_pubkey, &account_info)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?);
        
        Ok(())
    }
    async fn test_get_quote(&mut self, amount: u64) -> Result<Quote> {
        let whirlpool = self.whirlpool.as_mut()
            .context("Whirlpool not initialized")?;

        let params = QuoteParams {
            source_mint: Pubkey::from_str(SOL_MINT)?,
            destination_mint: Pubkey::from_str(USDC_MINT)?,
            amount,
            swap_mode: SwapMode::ExactIn,
        };

        info!("Получение котировки для {} SOL...", amount as f64 / 1e9);
        whirlpool.get_quote(&params)
            .map_err(|e| anyhow::anyhow!("Failed to get quote: {}", e))
    }

    async fn verify_pool_state(&self) -> Result<()> {
        let whirlpool = self.whirlpool.as_ref()
            .context("Whirlpool not initialized")?;

        let pool_data = &whirlpool.whirlpool_data;

        if pool_data.liquidity == 0 {
            error!("Пул не имеет ликвидности!");
            return Err(anyhow::anyhow!("Pool has no liquidity"));
        }

        if pool_data.fee_rate == 0 {
            error!("Некорректная комиссия пула!");
            return Err(anyhow::anyhow!("Invalid fee rate"));
        }

        info!("Верификация состояния пула успешна");
        info!("Текущий индекс тика: {}", pool_data.tick_current_index);
        info!("Ликвидность: {}", pool_data.liquidity);
        info!("Комиссия: {}", pool_data.fee_rate);
        
        Ok(())
    }
}

pub async fn run_whirlpool_test() -> Result<()> {
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let mut test = WhirlpoolTest::new(rpc_url);

    info!("Инициализация тестового окружения Whirlpool...");
    test.initialize().await?;

    info!("Проверка состояния пула...");
    test.verify_pool_state().await?;

    info!("Тестирование котировки для 1 SOL...");
    let quote = test.test_get_quote(1_000_000_000).await?;
    
    info!("Результаты котировки:");
    info!("Входящая сумма: {} SOL", quote.in_amount as f64 / 1e9);
    info!("Исходящая сумма: {} USDC", quote.out_amount as f64 / 1e6);
    info!("Комиссия: {} SOL", quote.fee_amount as f64 / 1e9);
    info!("Влияние на цену: {}%", quote.price_impact_pct);

    Ok(())
}