// src/markets/whirlpool.rs

use std::collections::HashMap;
use solana_program::{
    pubkey::Pubkey,
    account_info::AccountInfo,
    instruction::AccountMeta,
};
use spl_token::ID as TOKEN_PROGRAM_ID;
use anchor_lang::prelude::Pubkey as AnchorPubkey;

// Добавим импорты
use orca_whirlpools::SwapQuote;
use orca_whirlpools_core::{
    ExactOutSwapQuote, ExactInSwapQuote,
    try_apply_swap_fee, compute_swap,
    WhirlpoolFacade, TickArraySequence,
    get_tick_array_start_tick_index,
    MIN_TICK_INDEX, MAX_TICK_INDEX, TICK_ARRAY_SIZE, NUM_REWARDS,
    sqrt_price_to_tick_index, WhirlpoolRewardInfoFacade,
    get_next_initializable_tick_index, TickArrayFacade,
    get_prev_initializable_tick_index, TickFacade,
};
use orca_whirlpools_client::{
    SwapV2,
    SwapV2InstructionArgs
};

// Структуры для работы с Whirlpool
#[derive(Debug, Clone)]
pub struct Whirlpool {
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_current_index: i32,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub tick_spacing: u16,
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,
    pub reward_last_updated_timestamp: u64,
    pub reward_infos: [WhirlpoolRewardInfoFacade; NUM_REWARDS],
}

#[derive(Debug)]
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub price_impact: f64,
}

#[derive(Debug)]
pub struct OrcaSwapParams {
    pub amount: u64,
    pub sqrt_price_limit: Option<u128>,
    pub amount_specified_is_input: bool,
    pub a_to_b: bool,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SwapMode {
    ExactIn,
    ExactOut,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct QuoteParams {
    pub source_mint: Pubkey,
    pub destination_mint: Pubkey,
    pub amount: u64,
    pub swap_mode: SwapMode,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Quote {
    pub not_enough_liquidity: bool,
    pub in_amount: u64,
    pub out_amount: u64,
    pub fee_amount: u64,
    pub fee_mint: Pubkey,
    pub fee_pct: f64,
    pub price_impact_pct: f64,
}

// TODO: Структура для хранения аккаунтов для свопа
#[allow(dead_code)]
#[derive(Debug)]
pub struct SwapAccounts {
    pub token_program: Pubkey,
    pub token_authority: Pubkey,
    pub whirlpool: Pubkey,
    pub token_owner_account_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_owner_account_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_array0: Pubkey,
    pub tick_array1: Pubkey,
    pub tick_array2: Pubkey,
    pub oracle: Pubkey,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SwapLegType {
    // TODO: Добавить поддержку других типов свопов
    Whirlpool,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SwapOpportunity {
    // TODO: Реализовать для MEV
    pub pool_address: Pubkey,
    pub profit_estimate: u64,
    pub route: Vec<Pubkey>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct WhirlpoolAmm {
    pub address: Pubkey,
    pub id: String,
    pub label: String,
    pub whirlpool_data: Whirlpool,
    pub fee_pct: f64,
    pub tick_cache: Option<TickCache>,
}

// TODO: 
#[allow(dead_code)]
#[derive(Debug)]
pub struct SwapParams {
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit: u128,
    pub amount_specified_is_input: bool,
    pub a_to_b: bool,
    pub user_transfer_authority: Pubkey,
    pub user_source_token_account: Pubkey,
    pub user_destination_token_account: Pubkey,
}

// TODO: Реализовать трейт для кэширования
pub trait StateCache {
    fn update_pool_state(&mut self, pubkey: Pubkey, state: Vec<TickLiquidity>);
    fn get_pool_state(&self, pubkey: &Pubkey) -> Option<&Vec<TickLiquidity>>;
}

// TODO: Система кэширования состояний
#[allow(dead_code)]
pub trait PoolStateCache {
    fn update_state(&mut self, pubkey: &Pubkey, state: WhirlpoolFacade);
    fn get_state(&self, pubkey: &Pubkey) -> Option<&WhirlpoolFacade>;
    fn invalidate(&mut self, pubkey: &Pubkey);
    fn cleanup_old_states(&mut self);
}

// TODO: Система мониторинга через Geyser
#[allow(dead_code)]
pub trait GeyserMonitor {
    fn subscribe_to_updates(&mut self, accounts: Vec<Pubkey>);
    fn handle_account_update(&mut self, pubkey: &Pubkey, data: &[u8]);
    fn process_updates(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

// TODO: Интеграция с мемпулом
#[allow(dead_code)]
pub trait MempoolMonitor {
    fn subscribe_to_transactions(&mut self);
    fn handle_new_transaction(&mut self, tx: &[u8]);
    fn analyze_transaction(&self, tx: &[u8]) -> Result<bool, Box<dyn std::error::Error>>;
    fn get_potential_opportunities(&self) -> Vec<SwapOpportunity>;
}

// TODO: Валидация состояний
#[allow(dead_code)]
pub trait StateValidator {
    fn validate_pool_state(&self, state: &WhirlpoolFacade) -> Result<(), Box<dyn std::error::Error>>;
    fn validate_tick_array(&self, ticks: &[TickLiquidity]) -> Result<(), Box<dyn std::error::Error>>;
    fn validate_transaction(&self, tx: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}

// Определяем структуры на уровне модуля
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TickLiquidity {
    pub tick_index: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
}

// Реализация кэша для тиков
#[derive(Debug, Default)]
pub struct TickCache {
    ticks: HashMap<Pubkey, (Vec<TickLiquidity>, u64)>,
    max_age: u64,
}

impl TickCache {
    pub fn new(max_age: u64) -> Self {
        Self {
            ticks: HashMap::new(),
            max_age,
        }
    }

    fn is_valid(&self, timestamp: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - timestamp < self.max_age
    }
}

impl StateCache for TickCache {
    fn update_pool_state(&mut self, pubkey: Pubkey, state: Vec<TickLiquidity>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.ticks.insert(pubkey, (state, timestamp));
    }

    fn get_pool_state(&self, pubkey: &Pubkey) -> Option<&Vec<TickLiquidity>> {
        self.ticks.get(pubkey).and_then(|(ticks, timestamp)| {
            if self.is_valid(*timestamp) {
                Some(ticks)
            } else {
                None
            }
        })
    }
}

impl WhirlpoolAmm {
    pub fn new(address: Pubkey, account_info: &AccountInfo) -> Result<Self, Box<dyn std::error::Error>> {
        let whirlpool_data = Self::deserialize_whirlpool_data(&account_info.data.borrow())?;
        let fee_pct = whirlpool_data.fee_rate as f64 / 1_000_000.0;
        
        Ok(Self {
            address,
            id: address.to_string(),
            label: String::from("Orca (Whirlpools)"),
            whirlpool_data,
            fee_pct,
            tick_cache: Some(TickCache::new(60)), // 60 секунд TTL
        })
    }

    fn deserialize_whirlpool_data(data: &[u8]) -> Result<Whirlpool, Box<dyn std::error::Error>> {
        if data.len() < 8 {
            return Err("Invalid data length".into());
        }

        let mut offset = 8; // Пропускаем дискриминатор

        // whirlpool_bump
        offset += 1;

        // tick_spacing
        let tick_spacing = u16::from_le_bytes(data[offset..offset+2].try_into()?);
        offset += 2;

        // tick_spacing_seed
        offset += 2;

        // fee_rate
        let fee_rate = u16::from_le_bytes(data[offset..offset+2].try_into()?);
        offset += 2;

        // protocol_fee_rate
        let protocol_fee_rate = u16::from_le_bytes(data[offset..offset+2].try_into()?);
        offset += 2;

        // liquidity
        let liquidity = u128::from_le_bytes(data[offset..offset+16].try_into()?);
        offset += 16;

        // sqrt_price
        let sqrt_price = u128::from_le_bytes(data[offset..offset+16].try_into()?);
        offset += 16;

        // tick_current_index
        let tick_current_index = i32::from_le_bytes(data[offset..offset+4].try_into()?);
        offset += 4;

        // protocol_fee_owed_a, protocol_fee_owed_b
        offset += 16; // Пропускаем, так как не используем

        // token_mint_a
        let token_mint_a = Pubkey::new_from_array(data[offset..offset+32].try_into()?);
        offset += 32;

        // token_vault_a
        let token_vault_a = Pubkey::new_from_array(data[offset..offset+32].try_into()?);
        offset += 32;

        // fee_growth_global_a
        let fee_growth_global_a = u128::from_le_bytes(data[offset..offset+16].try_into()?);
        offset += 16;

        // token_mint_b
        let token_mint_b = Pubkey::new_from_array(data[offset..offset+32].try_into()?);
        offset += 32;

        // token_vault_b
        let token_vault_b = Pubkey::new_from_array(data[offset..offset+32].try_into()?);
        offset += 32;

        // fee_growth_global_b
        let fee_growth_global_b = u128::from_le_bytes(data[offset..offset+16].try_into()?);
        offset += 16;

        // reward_last_updated_timestamp
        let reward_last_updated_timestamp = u64::from_le_bytes(data[offset..offset+8].try_into()?);
        offset += 8;

        // reward_infos
        let mut reward_infos = [WhirlpoolRewardInfoFacade::default(); NUM_REWARDS];
        for reward_info in reward_infos.iter_mut() {
            reward_info.emissions_per_second_x64 = u128::from_le_bytes(data[offset..offset+16].try_into()?);
            offset += 16;
            reward_info.growth_global_x64 = u128::from_le_bytes(data[offset..offset+16].try_into()?);
            offset += 16;
        }

        // Валидация критических параметров
        if tick_spacing == 0 {
            return Err("Invalid pool data: tick_spacing cannot be zero".into());
        }

        Ok(Whirlpool {
            token_mint_a,
            token_mint_b,
            token_vault_a,
            token_vault_b,
            tick_current_index,
            sqrt_price,
            liquidity,
            fee_rate,
            protocol_fee_rate,
            tick_spacing,
            fee_growth_global_a,
            fee_growth_global_b,
            reward_last_updated_timestamp,
            reward_infos,
        })
    }

    // TODO: Методы для интеграции с Geyser
    #[allow(dead_code)]
    pub fn get_accounts_for_update(&self) -> Vec<Pubkey> {
        vec![self.address]
    }

    // TODO: Методы для интеграции с Geyser
    #[allow(dead_code)]
    pub fn update(&mut self, account_info_map: &HashMap<String, Option<AccountInfo>>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(Some(account_info)) = account_info_map.get(&self.address.to_string()) {
            self.whirlpool_data = Self::deserialize_whirlpool_data(&account_info.data.borrow())?;
        }
        Ok(())
    }

    pub fn get_quote(&mut self, params: &QuoteParams) -> Result<Quote, Box<dyn std::error::Error>> {
        // Получаем ликвидность в нужном диапазоне
        let available_liquidity = self.get_liquidity_in_range(
            MIN_TICK_INDEX,
            MAX_TICK_INDEX
        )?;

        // Проверяем достаточность ликвидности
        let not_enough_liquidity = match params.swap_mode {
            SwapMode::ExactIn => available_liquidity < (params.amount as u128),
            SwapMode::ExactOut => available_liquidity < (params.amount as u128),
        };

        if not_enough_liquidity {
            return Ok(Quote {
                not_enough_liquidity: true,
                in_amount: 0,
                out_amount: 0,
                fee_amount: 0,
                fee_mint: params.source_mint,
                fee_pct: self.fee_pct,
                price_impact_pct: 0.0,
            });
        }

        // Используем полученную ликвидность для расчета
        let a_to_b = params.source_mint == self.whirlpool_data.token_mint_a;

        // Получаем массивы тиков
        let tick_arrays = self.compute_tick_arrays(0, a_to_b)?;
        #[allow(unused_variables)]
        let tick_sequence: TickArraySequence<3> = TickArraySequence::new(
            tick_arrays.iter().map(|x| Some(TickArrayFacade {
                start_tick_index: get_tick_array_start_tick_index(
                    self.whirlpool_data.tick_current_index,
                    self.whirlpool_data.tick_spacing
                ),
                ticks: [TickFacade::default(); TICK_ARRAY_SIZE]
            })).collect::<Vec<_>>().try_into().unwrap(),
            self.whirlpool_data.tick_spacing
        )?;

        let orca_params = OrcaSwapParams {
            amount: params.amount,
            sqrt_price_limit: None,
            amount_specified_is_input: matches!(params.swap_mode, SwapMode::ExactIn),
            a_to_b,
        };

        let swap_result = self.calculate_swap_result(&orca_params)?;

        Ok(Quote {
            not_enough_liquidity: false,
            in_amount: swap_result.amount_in,
            out_amount: swap_result.amount_out,
            fee_amount: swap_result.fee_amount,
            fee_mint: params.source_mint,
            fee_pct: self.fee_pct,
            price_impact_pct: swap_result.price_impact.abs(),
        })
    }

    fn calculate_swap_result(&self, params: &OrcaSwapParams) -> Result<SwapResult, Box<dyn std::error::Error>> {
        let quote = if params.amount_specified_is_input {
            SwapQuote::ExactIn(ExactInSwapQuote {
                token_in: params.amount,
                token_est_out: u64::try_from(self.whirlpool_data.sqrt_price)?,
                token_min_out: u64::try_from(self.whirlpool_data.tick_current_index as u64)?,
                trade_fee: try_apply_swap_fee(params.amount, self.whirlpool_data.fee_rate)?,
            })
        } else {
            SwapQuote::ExactOut(ExactOutSwapQuote {
                token_out: params.amount,
                token_est_in: u64::try_from(self.whirlpool_data.sqrt_price)?,
                token_max_in: u64::try_from(self.whirlpool_data.tick_current_index as u64)?,
                trade_fee: try_apply_swap_fee(params.amount, self.whirlpool_data.fee_rate)?,
            })
        };

        // Расчет price_impact через compute_swap
        let whirlpool = WhirlpoolFacade {
            sqrt_price: self.whirlpool_data.sqrt_price,
            tick_current_index: self.whirlpool_data.tick_current_index,
            liquidity: self.whirlpool_data.liquidity,
            fee_rate: self.whirlpool_data.fee_rate,
            protocol_fee_rate: self.whirlpool_data.protocol_fee_rate,
            tick_spacing: self.whirlpool_data.tick_spacing,
            fee_growth_global_a: self.whirlpool_data.fee_growth_global_a,
            fee_growth_global_b: self.whirlpool_data.fee_growth_global_b,
            reward_last_updated_timestamp: self.whirlpool_data.reward_last_updated_timestamp,
            reward_infos: self.whirlpool_data.reward_infos
        };
        let tick_arrays = self.compute_tick_arrays(params.sqrt_price_limit.unwrap_or(0), params.a_to_b)?;
        let tick_sequence: TickArraySequence<3> = TickArraySequence::new(
            tick_arrays.iter().map(|_x| Some(TickArrayFacade {
                start_tick_index: get_tick_array_start_tick_index(
                    self.whirlpool_data.tick_current_index,
                    self.whirlpool_data.tick_spacing
                ),
                ticks: [TickFacade::default(); TICK_ARRAY_SIZE]
            })).collect::<Vec<_>>().try_into().unwrap(),
            self.whirlpool_data.tick_spacing
        )?;

        let swap_result = compute_swap(
            params.amount,
            params.sqrt_price_limit.unwrap_or(0),
            whirlpool,
            tick_sequence,
            params.a_to_b,
            params.amount_specified_is_input,
            0
        )?;

        let price_impact = if params.amount_specified_is_input {
            (swap_result.token_b as f64 / params.amount as f64) - 1.0
        } else {
            (params.amount as f64 / swap_result.token_a as f64) - 1.0
        };

        match quote {
            SwapQuote::ExactIn(q) => Ok(SwapResult {
                amount_in: q.token_in,
                amount_out: q.token_est_out,
                fee_amount: q.trade_fee,
                price_impact,
            }),
            SwapQuote::ExactOut(q) => Ok(SwapResult {
                amount_in: q.token_est_in,
                amount_out: q.token_out,
                fee_amount: q.trade_fee,
                price_impact,
            }),
        }
    }

    // TODO: Метод для создания транзакций свопа
    #[allow(dead_code)]
    pub fn get_swap_leg_and_accounts(&self, params: &SwapParams) -> Result<(SwapLegType, Vec<AccountMeta>), Box<dyn std::error::Error>> {
        // Получаем массивы тиков
        let tick_arrays = self.compute_tick_arrays(
            params.sqrt_price_limit,
            params.a_to_b
        )?;

        let swap_accounts = SwapV2 {
            token_program_a: AnchorPubkey::new_from_array(TOKEN_PROGRAM_ID.to_bytes()),
            token_program_b: AnchorPubkey::new_from_array(TOKEN_PROGRAM_ID.to_bytes()),
            memo_program: AnchorPubkey::new_from_array(solana_program::sysvar::clock::id().to_bytes()),
            token_authority: AnchorPubkey::new_from_array(params.user_transfer_authority.to_bytes()),
            whirlpool: AnchorPubkey::new_from_array(self.address.to_bytes()),
            token_mint_a: AnchorPubkey::new_from_array(self.whirlpool_data.token_mint_a.to_bytes()),
            token_mint_b: AnchorPubkey::new_from_array(self.whirlpool_data.token_mint_b.to_bytes()),
            token_owner_account_a: AnchorPubkey::new_from_array(params.user_source_token_account.to_bytes()),
            token_vault_a: AnchorPubkey::new_from_array(self.whirlpool_data.token_vault_a.to_bytes()),
            token_owner_account_b: AnchorPubkey::new_from_array(params.user_destination_token_account.to_bytes()),
            token_vault_b: AnchorPubkey::new_from_array(self.whirlpool_data.token_vault_b.to_bytes()),
            tick_array0: AnchorPubkey::new_from_array(tick_arrays[0].to_bytes()),
            tick_array1: AnchorPubkey::new_from_array(tick_arrays[1].to_bytes()),
            tick_array2: AnchorPubkey::new_from_array(tick_arrays[2].to_bytes()),
            oracle: AnchorPubkey::new_from_array(solana_program::sysvar::clock::id().to_bytes()),
        };

        let instruction = swap_accounts.instruction(SwapV2InstructionArgs {
            amount: params.amount,
            other_amount_threshold: params.other_amount_threshold,
            sqrt_price_limit: params.sqrt_price_limit,
            amount_specified_is_input: params.amount_specified_is_input,
            a_to_b: params.a_to_b,
            remaining_accounts_info: None,
        });

        let accounts = instruction.accounts.into_iter()
            .map(|meta| AccountMeta {
                pubkey: Pubkey::new_from_array(meta.pubkey.to_bytes()),
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect();

        Ok((SwapLegType::Whirlpool, accounts))
    }

    // Добавим метод для вычисления tick_array аккаунта
    fn compute_tick_arrays(&self, sqrt_price_limit: u128, a_to_b: bool) -> Result<[Pubkey; 3], Box<dyn std::error::Error>> {
        let current_array_start_index = get_tick_array_start_tick_index(
            self.whirlpool_data.tick_current_index,
            self.whirlpool_data.tick_spacing
        );
    
        let price_limit_tick = if a_to_b {
            if sqrt_price_limit == 0 { 
                MIN_TICK_INDEX 
            } else {
                sqrt_price_to_tick_index(sqrt_price_limit)
            }
        } else {
            if sqrt_price_limit == 0 { 
                MAX_TICK_INDEX 
            } else {
                sqrt_price_to_tick_index(sqrt_price_limit)
            }
        };
    
        let price_limit_array_start_index = get_tick_array_start_tick_index(
            price_limit_tick,
            self.whirlpool_data.tick_spacing
        );
    
        let (next_index, prev_index) = if a_to_b {
            if price_limit_array_start_index < current_array_start_index {
                (price_limit_array_start_index,
                 get_next_initializable_tick_index(current_array_start_index, self.whirlpool_data.tick_spacing))
            } else {
                (get_prev_initializable_tick_index(current_array_start_index, self.whirlpool_data.tick_spacing),
                 current_array_start_index)
            }
        } else {
            if price_limit_array_start_index > current_array_start_index {
                (price_limit_array_start_index,
                 get_prev_initializable_tick_index(current_array_start_index, self.whirlpool_data.tick_spacing))
            } else {
                (get_next_initializable_tick_index(current_array_start_index, self.whirlpool_data.tick_spacing),
                 current_array_start_index)
            }
        };
    
        let program_id = self.address;
        let mut tick_arrays = [Pubkey::default(); 3];
    
        for (i, start_tick) in [current_array_start_index, next_index, prev_index].iter().enumerate() {
            let seeds = [
                b"tick_array",
                self.address.as_ref(),
                &start_tick.to_le_bytes(),
            ];
            let (pubkey, _bump) = Pubkey::find_program_address(&seeds, &program_id);
            tick_arrays[i] = pubkey;
        }
    
        Ok(tick_arrays)
    }

    // Метод для работы с ликвидностью в диапазоне цен
    fn get_liquidity_in_range(&mut self, tick_lower: i32, tick_upper: i32) -> Result<u128, Box<dyn std::error::Error>> {
        // Проверяем tick_spacing
        if self.whirlpool_data.tick_spacing == 0 {
            return Err("Invalid whirlpool state: tick_spacing cannot be zero".into());
        }

        let current_tick = self.whirlpool_data.tick_current_index;
        let mut net_liquidity = self.whirlpool_data.liquidity;

        // Проверяем границы
        if tick_lower >= tick_upper {
            return Err("Invalid tick range".into());
        }

        // Получаем массивы тиков для диапазона
        let tick_arrays = self.compute_tick_arrays_for_range(
            tick_lower,
            tick_upper,
            self.whirlpool_data.tick_spacing
        )?;

        // Обрабатываем каждый тик в диапазоне
        for tick_array in tick_arrays {
            let ticks = self.load_tick_array(&tick_array)?;
            
            for tick in ticks {
                // Учитываем только тики в заданном диапазоне
                if tick.tick_index >= tick_lower && tick.tick_index <= tick_upper {
                    // Если текущий тик выше, добавляем ликвидность
                    if tick.tick_index <= current_tick {
                        net_liquidity = net_liquidity
                            .checked_add(tick.liquidity_net.unsigned_abs())
                            .ok_or("Liquidity overflow")?;
                    } else {
                        // Если ниже, вычитаем
                        net_liquidity = net_liquidity
                            .checked_sub(tick.liquidity_net.unsigned_abs())
                            .ok_or("Liquidity underflow")?;
                    }
                }
            }
        }

        Ok(net_liquidity)
    }

    // Вспомогательный метод для вычисления всех необходимых массивов тиков
    fn compute_tick_arrays_for_range(&self, tick_lower: i32, tick_upper: i32, tick_spacing: u16) 
        -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> 
    {
        // Проверяем tick_spacing на валидность
        if tick_spacing == 0 {
            return Err("Invalid tick spacing: cannot be zero".into());
        }

        let mut arrays = Vec::new();
        let current = tick_lower;

        // Получаем начальный индекс для первого массива
        let mut array_start_index = get_tick_array_start_tick_index(current, tick_spacing);

        while array_start_index <= tick_upper {
            let seeds = [
                b"tick_array",
                self.address.as_ref(),
                &array_start_index.to_le_bytes(),
            ];
            let (pubkey, _) = Pubkey::find_program_address(&seeds, &self.address);
            arrays.push(pubkey);

            // Переходим к следующему массиву тиков
            array_start_index = get_next_initializable_tick_index(
                array_start_index + (tick_spacing as i32 * TICK_ARRAY_SIZE as i32),
                tick_spacing
            );
        }

        Ok(arrays)
    }

    // Метод для загрузки данных тиков из аккаунта
    fn load_tick_array(&mut self, tick_array: &Pubkey) -> Result<Vec<TickLiquidity>, Box<dyn std::error::Error>> {
        // Пробуем получить из кэша
        if let Some(cache) = &self.tick_cache {
            if let Some(ticks) = cache.get_pool_state(tick_array) {
                return Ok(ticks.clone());
            }
        }

        // Получаем данные аккаунта
        let account_data = self.get_account_data(tick_array)?;
        
        // Пропускаем дискриминатор (8 байт)
        let mut ticks = Vec::new();
        let mut offset = 8;
        
        // Размер одного тика в байтах
        const TICK_SIZE: usize = 1 + 4 + 16 + 16 + 16 + 16;
        
        // Десериализуем тики
        while offset + TICK_SIZE <= account_data.len() {
            let initialized = account_data[offset] != 0;
            offset += 1;

            let index = i32::from_le_bytes(account_data[offset..offset+4].try_into()?);
            offset += 4;

            let liquidity_net = i128::from_le_bytes(account_data[offset..offset+16].try_into()?);
            offset += 16;

            let liquidity_gross = u128::from_le_bytes(account_data[offset..offset+16].try_into()?);
            offset += 16;

            let fee_growth_outside_a = u128::from_le_bytes(account_data[offset..offset+16].try_into()?);
            offset += 16;

            let fee_growth_outside_b = u128::from_le_bytes(account_data[offset..offset+16].try_into()?);
            offset += 16;

            if initialized {
                ticks.push(TickLiquidity {
                    tick_index: index,
                    liquidity_net,
                    liquidity_gross,
                    fee_growth_outside_a,
                    fee_growth_outside_b,
                });
            }
        }

        // Создаем WhirlpoolFacade для валидации
        let whirlpool = WhirlpoolFacade {
            sqrt_price: self.whirlpool_data.sqrt_price,
            tick_current_index: self.whirlpool_data.tick_current_index,
            liquidity: self.whirlpool_data.liquidity,
            fee_rate: self.whirlpool_data.fee_rate,
            protocol_fee_rate: self.whirlpool_data.protocol_fee_rate,
            tick_spacing: self.whirlpool_data.tick_spacing,
            fee_growth_global_a: 0,
            fee_growth_global_b: 0,
            reward_last_updated_timestamp: 0,
            reward_infos: [Default::default(); 3]
        };

        // Валидация
        self.validate_tick_spacing(&ticks, &whirlpool)?;
        self.validate_liquidity_consistency(&ticks, &whirlpool)?;

        // Кэшируем результат
        if let Some(cache) = &mut self.tick_cache {
            cache.update_pool_state(*tick_array, ticks.clone());
        }

        Ok(ticks)
    }

    // Добавляем методы валидации в WhirlpoolAmm
    fn validate_tick_spacing(&self, ticks: &[TickLiquidity], state: &WhirlpoolFacade) -> Result<(), Box<dyn std::error::Error>> {
        for tick in ticks {
            if tick.tick_index % state.tick_spacing as i32 != 0 {
                return Err("Invalid tick spacing alignment".into());
            }
            if tick.tick_index < MIN_TICK_INDEX || tick.tick_index > MAX_TICK_INDEX {
                return Err("Tick index out of bounds".into());
            }
            let array_start_index = get_tick_array_start_tick_index(tick.tick_index, state.tick_spacing);
            let array_end_index = array_start_index + (state.tick_spacing as i32 * TICK_ARRAY_SIZE as i32);
            if tick.tick_index < array_start_index || tick.tick_index >= array_end_index {
                return Err("Tick index outside of array bounds".into());
            }
        }
        Ok(())
    }

    fn validate_liquidity_consistency(&self, ticks: &[TickLiquidity], state: &WhirlpoolFacade) -> Result<(), Box<dyn std::error::Error>> {
        let mut net_liquidity = state.liquidity;
        let mut prev_tick_index = None;

        for tick in ticks {
            if let Some(prev_index) = prev_tick_index {
                if tick.tick_index <= prev_index {
                    return Err("Ticks must be strictly ascending".into());
                }
            }
            prev_tick_index = Some(tick.tick_index);

            if tick.tick_index <= state.tick_current_index {
                net_liquidity = net_liquidity
                    .checked_add(tick.liquidity_net.unsigned_abs())
                    .ok_or("Liquidity overflow")?;
            } else {
                if net_liquidity < tick.liquidity_net.unsigned_abs() {
                    return Err("Negative liquidity detected".into());
                }
                net_liquidity -= tick.liquidity_net.unsigned_abs();
            }
        }
        Ok(())
    }

    // Вспомогательный метод для получения данных аккаунта
    fn get_account_data(&self, _pubkey: &Pubkey) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // TODO: В реальной реализации здесь будет запрос к RPC или кэшу
        // Сейчас возвращаем заглушку для тестирования
        Err("Account data fetching not implemented".into())
    }
    
}