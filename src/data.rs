// src/data.rs

use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use lazy_static::lazy_static;
use dashmap::{DashMap, DashSet};
use log::{debug, info, warn, error};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use crate::websocket::ws_data::{DexType, SlotInfo};
use crate::websocket::ws_parser::PoolData;
use crate::decoder::{WhirlpoolData, RaydiumData, MeteoraData};
use bitvec::prelude::*;
use hashbrown::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use petgraph::Graph;
use crate::graph::PoolEdge;
use crate::math::calculators;
use crate::math::weight_calculators::{calculate_orca_weight, calculate_raydium_weight, calculate_meteora_weight};
use crate::router::RouterEngine; 

// Структура для хранения информации о токене
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct TokenInfo {
    pub symbol: String,
    pub address: Pubkey,
}

// Структура для хранения пары токенов
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TokenPair {
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
}

#[allow(dead_code)]
// Структура для хранения информации о пулах 
#[derive(Debug, Clone)]
pub struct BasePoolInfo {
    pub pool_address: Pubkey,
}

// Структура для хранения информации о ребре ликвидности
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LiquidityEdge {
    pub pool_address: Pubkey,
    pub token_in: TokenInfo,
    pub token_out: TokenInfo,
    pub liquidity: u64,
    pub fee_rate: u64,
}

// Базовая структура состояния пула для Orca
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OrcaPoolStateBase {
    // Адрес пула не из data декодера
    pub pool_address: Pubkey,

    // Базовые параметры пула
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_spacing: u16,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,

    // Ценовые параметры
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub price_threshold: u64,
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,

    // Протокольные параметры
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,

    // Метрики
    pub volume_24h: u64,
    pub tvl: u64,
    pub fees_24h: f64,
    
    // Статус
    pub is_active: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RaydiumPoolStateBase {
    // Адрес пула не из data декодера
    pub pool_address: Pubkey,

    pub status: u64,            
    pub pool_state: u64,           
    pub amm_id: Pubkey,              
    pub market_id: Pubkey,         
    
    // Токены и ликвидность
    pub token_a: Pubkey,           
    pub token_b: Pubkey,            
    pub lp_mint: Pubkey,            
    pub open_orders: Pubkey,    
    
    // Параметры пула
    pub needs_withdraw: u64,      
    pub recent_slot: u64,           
    pub last_order_slot: u64,         
    pub total_lp: u64,            
    pub base_need_take: u64,        
    pub quote_need_take: u64,    
    
    // Ценовые параметры  
    pub base_decimal: u64,      
    pub quote_decimal: u64,        
    pub min_price: u64,               
    pub max_price: u64,            
    pub vol_max_cut_ratio: u64,      
    
    // Параметры комиссий
    pub fee_numerator: u64,      
    pub fee_denominator: u64,     
    pub ret_fee_numerator: u64,      
    pub ret_fee_denominator: u64,     
    
    // Дополнительные параметры
    pub punish_pc_amount: u64,        
    pub punish_coin_amount: u64,      
    pub orders_num: u64,              
    pub depth: u64,                   
    pub open_time: u64,               
    pub switch_time: u64,             
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MeteoraPoolStateBase {
    // Адрес пула не из data декодера
    pub pool_address: Pubkey,

    pub pool_id: Pubkey,              
    pub authority: Pubkey,        
    pub token_mint_a: Pubkey,         
    pub token_mint_b: Pubkey,         
    pub token_vault_a: Pubkey,        
    pub token_vault_b: Pubkey,        
    pub lp_mint: Pubkey,              
    
    // Параметры ликвидности
    pub total_lp: u64,                
    pub liquidity: u128,              
    pub sqrt_price: u128,             
    pub current_tick_index: i32,      
    pub tick_spacing: u16,            
    pub fee_rate: u16,                
    
    // Параметры DLMM
    pub protocol_fee_rate: u16,       
    
    // Накопленные комиссии и метрики
    pub fee_growth_global_a: u128,    
    pub fee_growth_global_b: u128,    
    pub fee_protocol_token_a: u64,     
    pub fee_protocol_token_b: u64,     
    
    // Диапазоны цен и тиков
    pub max_price_sqrt: u128,       
    pub min_price_sqrt: u128,    
    pub max_tick_index: i32,      
    pub min_tick_index: i32,         
    
    // Параметры динамической ликвидности
    pub dynamic_liquidity_mode: u8,    
    pub liquidity_cap: u128,           
    pub liquidity_multiplier: u64,     
    
    // Метаданные и статистика
    pub last_update_timestamp: i64,    
    pub last_update_slot: u64,      
    pub volume_24h: u64,           
    pub fees_24h: u64,                
}

// Добавляем метод get_address для PoolStateBase
impl PoolStateBase {
    pub fn get_address(&self) -> Pubkey {
        match self {
            PoolStateBase::Orca(state) => state.pool_address,
            PoolStateBase::Raydium(state) => state.pool_address,
            PoolStateBase::Meteora(state) => state.pool_address,
        }
    }
}

// Состояние для Processed данных
#[derive(Debug, Clone)]
pub struct ProcessedPoolState {
    pub base: PoolStateBase,
    pub processed_slot: u64,
    pub last_update_time: u64,
}

impl ProcessedPoolState {
    pub fn update(&mut self, pool_data: &PoolData, slot: u64) -> bool {
        let mut updated = false;
        let pool_address = self.base.get_address();

        // 1. Обновляем базовое состояние
        if self.base.update(pool_data) {
            self.processed_slot = slot;
            self.last_update_time = unix_timestamp();
            updated = true;

            // 2. Обновляем ребра в обоих графах
            for graph in [&GLOBAL_DATA.processed_graph, &GLOBAL_DATA.finalized_graph] {
                if let Some(mut g) = graph.get_mut("main") {
                    if let Some(edge_idx) = g.edge_indices()
                        .find(|&e| g[e].pool_address == pool_address) {
                        
                        let edge = &mut g[edge_idx];
                        
                        // Обновляем метрики в зависимости от типа DEX
                        match &self.base {
                            PoolStateBase::Orca(state) => {
                                let price = calculators::calculate_orca_price(state.sqrt_price);
                                let fee_rate = state.fee_rate as f64 / 10_000.0;
                                let liquidity = state.liquidity as f64;
                                let weight = calculate_orca_weight(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    state.tick_spacing
                                );

                                edge.update_metrics(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    weight,
                                    state.is_active,
                                    slot
                                );

                                debug!("Обновлены метрики для Orca пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                                    pool_address, price, fee_rate, liquidity, weight);
                            },
                            PoolStateBase::Raydium(state) => {
                                let price = calculators::calculate_raydium_price(state.min_price, state.max_price);
                                let fee_rate = calculators::calculate_raydium_fee(
                                    state.fee_numerator,
                                    state.fee_denominator
                                );
                                let liquidity = state.total_lp as f64;
                                let weight = calculate_raydium_weight(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    state.orders_num,
                                    state.depth
                                );

                                edge.update_metrics(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    weight,
                                    state.status != 0 && state.pool_state != 0,
                                    slot
                                );

                                debug!("Обновлены метрики для Raydium пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                                    pool_address, price, fee_rate, liquidity, weight);
                            },
                            PoolStateBase::Meteora(state) => {
                                let price = calculators::calculate_meteora_price(state.sqrt_price);
                                let fee_rate = state.fee_rate as f64 / 10_000.0;
                                let liquidity = state.liquidity as f64;
                                let weight = calculate_meteora_weight(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    state.liquidity_multiplier
                                );

                                edge.update_metrics(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    weight,
                                    state.dynamic_liquidity_mode != 0 && state.liquidity_cap > 0,
                                    slot
                                );

                                debug!("Обновлены метрики для Meteora пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                                    pool_address, price, fee_rate, liquidity, weight);
                            }
                        }
                    }
                }
            }
        }

        updated
    }
}

// Состояние для Finalized данных
#[derive(Debug, Clone)]
pub struct FinalizedPoolState {
    pub base: PoolStateBase,
    pub finalized_slot: u64,
    pub last_update_time: u64,
}

impl FinalizedPoolState {
    pub fn update(&mut self, pool_data: &PoolData, slot: u64) -> bool {
        let mut updated = false;
        let pool_address = self.base.get_address();

        if self.base.update(pool_data) {
            self.finalized_slot = slot;
            self.last_update_time = unix_timestamp();
            updated = true;

            // Обновляем ребра в обоих графах
            for graph in [&GLOBAL_DATA.processed_graph, &GLOBAL_DATA.finalized_graph] {
                if let Some(mut g) = graph.get_mut("main") {
                    if let Some(edge_idx) = g.edge_indices()
                        .find(|&e| g[e].pool_address == pool_address) {
                        
                        let edge = &mut g[edge_idx];
                        
                        // Обновляем метрики в зависимости от типа DEX
                        match &self.base {
                            PoolStateBase::Orca(state) => {
                                let price = calculators::calculate_orca_price(state.sqrt_price);
                                let fee_rate = state.fee_rate as f64 / 10_000.0;
                                let liquidity = state.liquidity as f64;
                                let weight = calculate_orca_weight(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    state.tick_spacing
                                );

                                edge.update_metrics(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    weight,
                                    state.is_active,
                                    slot
                                );

                                debug!("Обновлены метрики для Orca пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                                    pool_address, price, fee_rate, liquidity, weight);
                            },
                            PoolStateBase::Raydium(state) => {
                                let price = calculators::calculate_raydium_price(state.min_price, state.max_price);
                                let fee_rate = calculators::calculate_raydium_fee(
                                    state.fee_numerator,
                                    state.fee_denominator
                                );
                                let liquidity = state.total_lp as f64;
                                let weight = calculate_raydium_weight(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    state.orders_num,
                                    state.depth
                                );

                                edge.update_metrics(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    weight,
                                    state.status != 0 && state.pool_state != 0,
                                    slot
                                );

                                debug!("Обновлены метрики для Raydium пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                                    pool_address, price, fee_rate, liquidity, weight);
                            },
                            PoolStateBase::Meteora(state) => {
                                let price = calculators::calculate_meteora_price(state.sqrt_price);
                                let fee_rate = state.fee_rate as f64 / 10_000.0;
                                let liquidity = state.liquidity as f64;
                                let weight = calculate_meteora_weight(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    state.liquidity_multiplier
                                );

                                edge.update_metrics(
                                    price,
                                    fee_rate,
                                    liquidity,
                                    weight,
                                    state.dynamic_liquidity_mode != 0 && state.liquidity_cap > 0,
                                    slot
                                );

                                debug!("Обновлены метрики для Meteora пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                                    pool_address, price, fee_rate, liquidity, weight);
                            }
                        }
                    }
                }
            }
        }

        updated
    }
}

// Состояние сети для каждого DEX
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct NetworkState {
    pub current_slot: u64,
    pub parent_slot: u64,
    pub root_slot: u64,
    pub last_processed_slot: u64,
    pub last_update_time: u64,
}

impl NetworkState {
    pub fn new() -> Self {
        Self {
            current_slot: 0,
            parent_slot: 0,
            root_slot: 0,
            last_processed_slot: 0,
            last_update_time: unix_timestamp(),
        }
    }
}

// Структура для быстрого поиска индекса по адресу пула
#[derive(Debug, Default)]
pub struct PoolLookupTable {
    // Хеш-таблица для быстрого поиска индекса по адресу пула
    lookup: HashMap<(DexType, Pubkey), u32>,
    // Битовый массив для хранения статуса существования
    existence_mask: BitVec,
    // Счетчик для генерации новых индексов
    next_index: AtomicU32,
}

impl PoolLookupTable {
    pub fn new() -> Self {
        Self {
            lookup: HashMap::with_capacity(1024), // Предварительное выделение памяти
            existence_mask: BitVec::with_capacity(1024),
            next_index: AtomicU32::new(0),
        }
    }

    // Проверка существования пула
    #[inline]
    pub fn exists(&self, dex: DexType, pool_address: &Pubkey) -> bool {
        self.lookup
            .get(&(dex, *pool_address))
            .map(|&idx| self.existence_mask[idx as usize])
            .unwrap_or(false)
    }

    // Добавление пула в таблицу
    #[inline]
    pub fn insert_pool(&mut self, dex: DexType, pool_address: Pubkey) {
        let idx = self.next_index.fetch_add(1, Ordering::Relaxed);
        if (idx as usize) >= self.existence_mask.len() {
            self.existence_mask.push(true);
        } else {
            self.existence_mask.set(idx as usize, true);
        }
        self.lookup.insert((dex, pool_address), idx);
    }

    // Удаление пула из таблицы
    #[allow(dead_code)]
    #[inline]
    pub fn remove_pool(&mut self, dex: DexType, pool_address: &Pubkey) {
        if let Some(&idx) = self.lookup.get(&(dex, *pool_address)) {
            self.existence_mask.set(idx as usize, false);
        }
    }
}

// Глобальная структура данных
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct GlobalData {
    // Общие данные для всех DEX
    pub tokens: Arc<DashMap<String, TokenInfo>>,
    pub token_addresses: Arc<DashMap<Pubkey, TokenInfo>>,
    pub token_pairs: Arc<DashSet<TokenPair>>,
    
    // DEX-специфичные данные основного хранилища данных
    pub dex_pools: Arc<DashMap<DexType, DashMap<TokenPair, Vec<BasePoolInfo>>>>,
    pub processed_pool_states: Arc<DashMap<DexType, DashMap<Pubkey, ProcessedPoolState>>>,
    pub finalized_pool_states: Arc<DashMap<DexType, DashMap<Pubkey, FinalizedPoolState>>>,

    // Общие данные для сети по ключу DexType (не понятно чем отличаются
    pub network_states: Arc<DashMap<DexType, NetworkState>>,
    // Общие данные для сети по ключу String (не понятно чем отличаются)
    pub network_state: Arc<DashMap<String, NetworkState>>,
    
    // Общие данные для маршрутизации
    pub liquidity_edges: Arc<DashMap<Pubkey, Vec<LiquidityEdge>>>,

    // Структура для хранения быстрого доступа к пулам
    pub pool_lookup: Arc<DashMap<DexType, PoolLookupTable>>,

    // Структуры для хранения цепочек
    pub chains_4: Arc<DashSet<Vec<String>>>,
    pub chains_5: Arc<DashSet<Vec<String>>>,

    // Графы для processed и finalized состояний
    pub processed_graph: Arc<DashMap<String, Graph<String, PoolEdge>>>,
    pub finalized_graph: Arc<DashMap<String, Graph<String, PoolEdge>>>,

    // Быстрый поиск цепочек по адресу пула
    pub chain_references: Arc<DashMap<Pubkey, Vec<usize>>>,

    // Индексированное хранилище цепочек
    pub chain_storage_4: Arc<DashMap<usize, Vec<String>>>,
    pub chain_storage_5: Arc<DashMap<usize, Vec<String>>>,
}

// Глобальный экземпляр данных
lazy_static! {
    pub static ref GLOBAL_DATA: GlobalData = GlobalData {
        tokens: Arc::new(DashMap::new()),
        token_addresses: Arc::new(DashMap::new()),
        token_pairs: Arc::new(DashSet::new()),
        dex_pools: Arc::new(DashMap::new()),
        processed_pool_states: Arc::new(DashMap::new()),
        finalized_pool_states: Arc::new(DashMap::new()),
        network_states: Arc::new(DashMap::new()),
        liquidity_edges: Arc::new(DashMap::new()),
        network_state: Arc::new(DashMap::new()),
        pool_lookup: Arc::new(DashMap::new()),
        chains_4: Arc::new(DashSet::new()),
        chains_5: Arc::new(DashSet::new()),
        processed_graph: Arc::new(DashMap::new()),
        finalized_graph: Arc::new(DashMap::new()),
        chain_references: Arc::new(DashMap::new()),
        chain_storage_4: Arc::new(DashMap::new()),
        chain_storage_5: Arc::new(DashMap::new()),
    };
}

// Вспомогательная функция для получения unix timestamp
pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl GlobalData {
    // Инициализация структуры DEX для пулов
    pub fn initialize_pool_states(&self, dex: DexType) {
        // Инициализируем структуры для finalized состояний если их еще нет
        if !self.finalized_pool_states.contains_key(&dex) {
            self.finalized_pool_states.insert(dex, DashMap::new());
            info!("Initialized finalized pool states for {:?}", dex);
        }

        // Инициализируем структуры для processed состояний если их еще нет
        if !self.processed_pool_states.contains_key(&dex) {
            self.processed_pool_states.insert(dex, DashMap::new());
            info!("Initialized processed pool states for {:?}", dex);
        }
    }
    // Добавление токена
    pub fn add_token(&self, symbol: String, address: Pubkey) {
        let token_info = TokenInfo {
            symbol: symbol.clone(),
            address,
        };
        
        // debug!("Добавление токена: {} с адресом {}", symbol, address);
        
        // Сохраняем токен в обоих мапах для быстрого доступа
        self.tokens.insert(symbol, token_info.clone());
        self.token_addresses.insert(address, token_info);
    }

    // Добавление пары токенов
    pub fn add_token_pair(&self, token_a_symbol: String, token_b_symbol: String) {
        if let (Some(token_a), Some(token_b)) = (
            self.tokens.get(&token_a_symbol),
            self.tokens.get(&token_b_symbol)
        ) {
            let pair = if token_a.symbol < token_b.symbol {
                TokenPair {
                    token_a: token_a.clone(),
                    token_b: token_b.clone(),
                }
            } else {
                TokenPair {
                    token_a: token_b.clone(),
                    token_b: token_a.clone(),
                }
            };
            
            // debug!("Добавление пары токенов: {:?}", pair);
            self.token_pairs.insert(pair);
        }
    }

    // Добавление пулов всех DEX
    pub fn add_pools(
        &self,
        token_a_symbol: String,
        token_b_symbol: String,
        pool_address: Pubkey,
        _token_a_address: Pubkey,  // Теперь эти параметры не нужны, так как информация
        _token_b_address: Pubkey,  // о токенах уже есть в TokenInfo
        tvl: f64,
        dex: DexType
    ) -> bool {
        // Проверяем TVL
        if tvl < 100000.0 {
            debug!("Пропуск пула с низким TVL ({}) для пары {}-{}", tvl, token_a_symbol, token_b_symbol);
            return false;
        }

        // Получаем информацию о токенах
        if let (Some(token_a), Some(token_b)) = (
            self.tokens.get(&token_a_symbol),
            self.tokens.get(&token_b_symbol)
        ) {
            let pair = if token_a.symbol < token_b.symbol {
                TokenPair {
                    token_a: token_a.clone(),
                    token_b: token_b.clone(),
                }
            } else {
                TokenPair {
                    token_a: token_b.clone(),
                    token_b: token_a.clone(),
                }
            };

            // info!("Добавление пула для пары {:?} с адресом {}", pair, pool_address);
            
            // Используем dex_pools 
            self.dex_pools
                .entry(dex)
                .or_insert_with(DashMap::new)
                .entry(pair)
                .or_insert_with(Vec::new)
                .push(BasePoolInfo {
                    pool_address,
                });

            // Безопасное добавление в PoolLookupTable
            self.pool_lookup
                .entry(dex)
                .or_insert_with(PoolLookupTable::new)
                .insert_pool(dex, pool_address);

            // debug!("[POOL LOOKUP] Добавлен пул {} в PoolLookupTable для DEX {:?}", pool_address, dex);
            true
        } else {
            debug!("Не найдена информация о токенах для пары {}-{}", token_a_symbol, token_b_symbol);
            false
        }
    }

    // Проверка существования пула
    pub fn pool_exists(&self, dex: DexType, pool_address: &Pubkey) -> bool {
        // warn!("Проверка существования пула и вызов pool_exists");
        // Используем DexType как ключ для поиска в PoolLookupTable
        if let Some(lookup_table) = self.pool_lookup.get(&dex) {
            lookup_table.exists(dex, pool_address)
        } else {
            false
        }
    }

    // Обновление состояния сети
    pub fn update_network_state(&self, slot_info: SlotInfo) {
        // Проверяем задержку обновлений
        let current_time = unix_timestamp();
        
        // Получаем или создаем состояние для текущего слота
        let mut state = self.network_state
            .entry("current".to_string())
            .or_insert_with(|| NetworkState::new());
        
        let time_since_last_update = current_time - state.last_update_time;
        
        if time_since_last_update > 1 {
            warn!("Slot updates delayed by {}s", time_since_last_update);
        }

        // Проверяем пропущенные слоты
        let slots_missed = if slot_info.slot > state.current_slot + 1 {
            slot_info.slot - state.current_slot - 1
        } else {
            0
        };

        if slots_missed > 0 {
            warn!("Missed {} slots between {} and {}", 
                  slots_missed, state.current_slot, slot_info.slot);
        }

        // Обновляем состояние
        state.current_slot = slot_info.slot;
        state.parent_slot = slot_info.parent;
        state.root_slot = slot_info.root;
        state.last_update_time = current_time;
    }

    // Метод проверки актуальности данных
    // TODO: Добавить реализацию и логику для регулирования
    pub fn validate_slot_consistency(&self, update_slot: u64) -> bool {
        if let Some(state) = self.network_state.get("current") {
            if state.current_slot > update_slot + 10 {
                // warn!("Processing outdated data: current_slot={}, update_slot={}", state.current_slot, update_slot);
                // return false;
            }
            true
        } else {
            // Если состояние еще не инициализировано, пропускаем валидацию
            true
        }
    }

    /// Находит адрес пула по символам токенов
    pub fn find_pool_address_by_symbols(&self, dex: DexType, symbol_a: &str, symbol_b: &str) -> Option<Pubkey> {
        // Получаем TokenInfo для обоих символов
        let token_a = self.tokens.get(symbol_a)?;
        let token_b = self.tokens.get(symbol_b)?;

        // Ищем пул в обоих вариантах порядка токенов
        if let Some(dex_pools) = self.dex_pools.get(&dex) {
            // Пробуем первый вариант порядка
            let pair1 = TokenPair {
                token_a: token_a.clone(),
                token_b: token_b.clone(),
            };
            
            // Пробуем второй вариант порядка
            let pair2 = TokenPair {
                token_a: token_b.clone(),
                token_b: token_a.clone(),
            };

            // Проверяем оба варианта
            if let Some(pool_infos) = dex_pools.get(&pair1) {
                if let Some(first_pool) = pool_infos.first() {
                    return Some(first_pool.pool_address);
                }
            }

            if let Some(pool_infos) = dex_pools.get(&pair2) {
                if let Some(first_pool) = pool_infos.first() {
                    return Some(first_pool.pool_address);
                }
            }
        }
        None
    }

    // Находит адрес пула по адресам токенов
    #[allow(dead_code)]
    pub fn find_pool_address_by_addresses(&self, dex: DexType, address_a: &Pubkey, address_b: &Pubkey) -> Option<Pubkey> {
        // Получаем TokenInfo для обоих адресов
        let token_a = self.token_addresses.get(address_a)?;
        let token_b = self.token_addresses.get(address_b)?;

        // Ищем пул в обоих вариантах порядка токенов
        if let Some(dex_pools) = self.dex_pools.get(&dex) {
            // Пробуем первый вариант порядка
            let pair1 = TokenPair {
                token_a: token_a.clone(),
                token_b: token_b.clone(),
            };
            
            // Пробуем второй вариант порядка
            let pair2 = TokenPair {
                token_a: token_b.clone(),
                token_b: token_a.clone(),
            };

            // Проверяем оба варианта
            if let Some(pool_infos) = dex_pools.get(&pair1) {
                if let Some(first_pool) = pool_infos.first() {
                    return Some(first_pool.pool_address);
                }
            }

            if let Some(pool_infos) = dex_pools.get(&pair2) {
                if let Some(first_pool) = pool_infos.first() {
                    return Some(first_pool.pool_address);
                }
            }
        }
        None
    }

    // Функция проверки и валидация графов
    pub fn validate_graphs(&self) -> bool {
        for graph in [&self.processed_graph, &self.finalized_graph] {
            if let Some(g) = graph.get("main") {
                info!("Валидация графа: {} вершин, {} ребер", 
                      g.node_count(), g.edge_count());
                
                // Проверяем все ребра
                for edge in g.edge_indices() {
                    let edge_ref = &g[edge];
                    if !self.pool_exists(DexType::Orca, &edge_ref.pool_address) &&
                       !self.pool_exists(DexType::Raydium, &edge_ref.pool_address) &&
                       !self.pool_exists(DexType::Meteora, &edge_ref.pool_address) {
                        error!("Найдено ребро с несуществующим пулом: {}", 
                               edge_ref.pool_address);
                        return false;
                    }
                }
            } else {
                error!("Граф 'main' не найден");
                return false;
            }
        }
        true
    }
}

// Базовый трейт для всех пулов
pub trait PoolState: Send + Sync {
    fn update(&mut self, new_state: &PoolData) -> bool;
    fn from_pool_data(pool_address: Pubkey, data: &PoolData) -> Self where Self: Sized;
}

// Общая структура состояния пула
#[derive(Debug, Clone)]
pub enum PoolStateBase {
    Orca(OrcaPoolStateBase),
    Raydium(RaydiumPoolStateBase),
    Meteora(MeteoraPoolStateBase),
}

// Реализация трейта для каждого типа пула
impl PoolState for PoolStateBase {
    fn update(&mut self, new_state: &PoolData) -> bool {
        match (self, new_state) {
            (PoolStateBase::Orca(base), PoolData::Whirlpool(data)) => base.update(data),
            (PoolStateBase::Raydium(base), PoolData::Raydium(data)) => base.update(data),
            (PoolStateBase::Meteora(base), PoolData::Meteora(data)) => base.update(data),
            _ => {
                warn!("Mismatched pool types in update");
                false
            }
        }
    }

    fn from_pool_data(pool_address: Pubkey, data: &PoolData) -> Self {
        match data {
            PoolData::Whirlpool(data) => PoolStateBase::Orca(OrcaPoolStateBase::from_whirlpool(pool_address, data)),
            PoolData::Raydium(data) => PoolStateBase::Raydium(RaydiumPoolStateBase::from_raydium(pool_address, data)),
            PoolData::Meteora(data) => PoolStateBase::Meteora(MeteoraPoolStateBase::from_meteora(pool_address, data)),
        }
    }
}

impl OrcaPoolStateBase {
    pub fn update(&mut self, new_state: &WhirlpoolData) -> bool {
        let mut updated = false;
        let mut needs_update = false;

        // Проверка активности пула
        let is_dead_address = "11111111111111111111111111111111";
        let new_is_active = ![
            new_state.token_mint_a.to_string(),
            new_state.token_mint_b.to_string(),
            new_state.token_vault_a.to_string(), 
            new_state.token_vault_b.to_string()
        ].contains(&is_dead_address.to_string());
        
        // Проверка изменений пула
        if self.is_active != new_is_active {
            info!("[ORCA STATE] Pool {} activity changed: {} -> {}", 
                  self.pool_address, self.is_active, new_is_active);
            self.is_active = new_is_active;
            needs_update = true;
        }

        // Обновление ценовых параметров
        let new_sqrt_price = new_state.sqrt_price;
        if self.sqrt_price != new_sqrt_price {
            debug!("[ORCA STATE] Pool {} sqrt_price update: {} -> {}", 
                   self.pool_address, self.sqrt_price, new_sqrt_price);
            self.sqrt_price = new_sqrt_price;
            needs_update = true;
        }

        // Обновление пороговой цены
        let new_price_threshold = new_state.price_threshold;
        if self.price_threshold != new_price_threshold {
            debug!("[ORCA STATE] Pool {} price_threshold update: {} -> {}", 
                   self.pool_address, self.price_threshold, new_price_threshold);
            self.price_threshold = new_price_threshold;
            needs_update = true;
        }

        // Обновление ликвидности
        let new_liquidity = new_state.liquidity;
        if self.liquidity != new_liquidity {
            debug!("[ORCA STATE] Pool {} liquidity update: {} -> {}", 
                   self.pool_address, self.liquidity, new_liquidity);
            self.liquidity = new_liquidity;
            needs_update = true;
        }

        // Обновление тика
        let new_tick_current_index = new_state.tick_current_index;
        if self.tick_current_index != new_tick_current_index {
            debug!("[ORCA STATE] Pool {} tick_index update: {} -> {}", 
                   self.pool_address, self.tick_current_index, new_tick_current_index);
            self.tick_current_index = new_tick_current_index;
            needs_update = true;
        }

        // Обновление комиссий
        let new_fee_rate = new_state.fee_rate;
        if self.fee_rate != new_fee_rate {
            debug!("[ORCA STATE] Pool {} fee_rate update: {} -> {}", 
                   self.pool_address, self.fee_rate, new_fee_rate);
            self.fee_rate = new_fee_rate;
            needs_update = true;
        }

        // Обновление протокольной комиссии
        let new_protocol_fee_rate = new_state.protocol_fee_rate;
        if self.protocol_fee_rate != new_protocol_fee_rate {
            debug!("[ORCA STATE] Pool {} protocol_fee_rate update: {} -> {}", 
                   self.pool_address, self.protocol_fee_rate, new_protocol_fee_rate);
            self.protocol_fee_rate = new_protocol_fee_rate;
            needs_update = true;
        }

        // Обновление накопленных комиссий
        let new_fee_growth_global_a = new_state.fee_growth_global_a;
        if self.fee_growth_global_a != new_fee_growth_global_a {
            debug!("[ORCA STATE] Pool {} fee_growth_a update: {} -> {}", 
                   self.pool_address, self.fee_growth_global_a, new_fee_growth_global_a);
            self.fee_growth_global_a = new_fee_growth_global_a;
            needs_update = true;
        }

        // Обновление накопленных комиссий для токена B
        let new_fee_growth_global_b = new_state.fee_growth_global_b;
        if self.fee_growth_global_b != new_fee_growth_global_b {
            debug!("[ORCA STATE] Pool {} fee_growth_b update: {} -> {}", 
                   self.pool_address, self.fee_growth_global_b, new_fee_growth_global_b);
            self.fee_growth_global_b = new_fee_growth_global_b;
            needs_update = true;
        }

        // Обновление протокольных комиссий
        let new_protocol_fee_owed_a = new_state.protocol_fee_owed_a;
        if self.protocol_fee_owed_a != new_protocol_fee_owed_a {
            debug!("[ORCA STATE] Pool {} protocol_fee_owed_a update: {} -> {}", 
                   self.pool_address, self.protocol_fee_owed_a, new_protocol_fee_owed_a);
            self.protocol_fee_owed_a = new_protocol_fee_owed_a;
            needs_update = true;
        }

        // Обновление протокольных комиссий для токена B
        let new_protocol_fee_owed_b = new_state.protocol_fee_owed_b;
        if self.protocol_fee_owed_b != new_protocol_fee_owed_b {
            debug!("[ORCA STATE] Pool {} protocol_fee_owed_b update: {} -> {}", 
                   self.pool_address, self.protocol_fee_owed_b, new_protocol_fee_owed_b);
            self.protocol_fee_owed_b = new_protocol_fee_owed_b;
            needs_update = true;
        }

        if needs_update {
            updated = true;
        }

        if updated {
            // Обновляем ребро в обоих графах
            for graph in [&GLOBAL_DATA.processed_graph, &GLOBAL_DATA.finalized_graph] {
                if let Some(mut g) = graph.get_mut("main") {
                    if let Some(edge_idx) = g.edge_indices()
                        .find(|&e| g[e].pool_address == self.pool_address) {
                                                
                        let edge = &mut g[edge_idx];
                        let price = calculators::calculate_orca_price(self.sqrt_price);
                        let fee_rate = self.fee_rate as f64 / 10_000.0;
                        let liquidity = self.liquidity as f64;
                        let weight = calculate_orca_weight(
                            price,
                            fee_rate,
                            liquidity,
                            self.tick_spacing
                        );

                        edge.update_metrics(
                            price,
                            fee_rate,
                            liquidity,
                            weight,
                            self.is_active,
                            GLOBAL_DATA.network_state
                                .get("current")
                                .map(|s| s.current_slot)
                                .unwrap_or(0)
                        );

                        debug!("Обновлены метрики ребер для Orca пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                            self.pool_address, price, fee_rate, liquidity, weight);
                    }
                }
            }
            // Обновляем все цепочки, использующие этот пул
            // TODO: ВЫЗОВ СТАРОЙ ФУНКЦИИ. ДОБАВИТЬ ТРИГГЕР ПЕРЕСЧЕТА ЦЕПОЧЕК
            // GLOBAL_DATA.update_affected_chains(self.pool_address);

            // НОВЫЙ ВЫЗОВ:
            RouterEngine::update_affected_chains(self.pool_address);
        }
        
        updated
    }

    pub fn from_whirlpool(pool_address: Pubkey, data: &WhirlpoolData) -> Self {
        Self {
            pool_address,
            token_mint_a: data.token_mint_a,
            token_vault_a: data.token_vault_a,
            token_mint_b: data.token_mint_b,
            token_vault_b: data.token_vault_b,
            tick_spacing: data.tick_spacing,
            fee_rate: data.fee_rate,
            protocol_fee_rate: data.protocol_fee_rate,
            liquidity: data.liquidity,
            sqrt_price: data.sqrt_price,
            tick_current_index: data.tick_current_index,
            price_threshold: data.price_threshold,
            fee_growth_global_a: data.fee_growth_global_a,
            fee_growth_global_b: data.fee_growth_global_b,
            protocol_fee_owed_a: data.protocol_fee_owed_a,
            protocol_fee_owed_b: data.protocol_fee_owed_b,
            volume_24h: 0,
            tvl: 0,
            fees_24h: 0.0,
            is_active: true,
        }
    }
}

impl RaydiumPoolStateBase {
    pub fn update(&mut self, new_state: &RaydiumData) -> bool {
        let mut updated = false;
        let mut needs_update = false;

        // Проверка активности пула
        let is_dead_address = Pubkey::default().to_string();
        let new_is_active = ![
            new_state.token_a.to_string(),
            new_state.token_b.to_string(),
        ].contains(&is_dead_address) 
            && new_state.status != 0 
            && new_state.pool_state != 0
            && new_state.open_time != 0;

        // Обновление статуса активности
        let new_status = new_state.status;
        if self.status != new_status || 
           (new_status != 0 && !new_is_active) {
            debug!("[RAYDIUM STATE] Pool {} activity changed: status={}, is_active={}", 
                   self.pool_address, new_status, new_is_active);
            
            // Обновляем статус только если пул активен
            if new_is_active {
                self.status = new_status;
            } else {
                self.status = 0; // Устанавливаем статус в 0 для неактивного пула
            }
            needs_update = true;
        }

        // Обновление состояния пула
        let new_pool_state = new_state.pool_state;
        if self.pool_state != new_pool_state {
            debug!("[RAYDIUM STATE] Pool {} pool_state update: {} -> {}", 
                   self.pool_address, self.pool_state, new_pool_state);
            self.pool_state = new_pool_state;
            needs_update = true;
        }

        // Обновление total_lp
        let new_total_lp = new_state.total_lp;
        if self.total_lp != new_total_lp {
            debug!("[RAYDIUM STATE] Pool {} total_lp update: {} -> {}", 
                   self.pool_address, self.total_lp, new_total_lp);
            self.total_lp = new_total_lp;
            needs_update = true;
        }

        // Обновление параметров цены
        let new_min_price = new_state.min_price;
        if self.min_price != new_min_price {
            debug!("[RAYDIUM STATE] Pool {} min_price update: {} -> {}", 
                   self.pool_address, self.min_price, new_min_price);
            self.min_price = new_min_price;
            needs_update = true;
        }

        // Обновление максимальной цены
        let new_max_price = new_state.max_price;
        if self.max_price != new_max_price {
            debug!("[RAYDIUM STATE] Pool {} max_price update: {} -> {}", 
                   self.pool_address, self.max_price, new_max_price);
            self.max_price = new_max_price;
            needs_update = true;
        }

        // Обновление глубины ордеров
        let new_orders_num = new_state.orders_num;
        let new_depth = new_state.depth;
        if self.orders_num != new_orders_num || self.depth != new_depth {
            debug!("[RAYDIUM STATE] Pool {} orders update: count={}, depth={}", 
                self.pool_address, new_orders_num, new_depth);
            self.orders_num = new_state.orders_num;
            self.depth = new_state.depth;
            needs_update = true;
        }

        // Отслеживание необходимости вывода
        let new_base_need_take = new_state.base_need_take;
        let new_quote_need_take = new_state.quote_need_take;
        if self.base_need_take != new_base_need_take || 
        self.quote_need_take != new_quote_need_take {
            debug!("[RAYDIUM STATE] Pool {} needs take update: base={}, quote={}", 
                self.pool_address, new_base_need_take, new_quote_need_take);
            self.base_need_take = new_base_need_take;
            self.quote_need_take = new_quote_need_take;
            needs_update = true;
        }

        // Обновление комиссий
        let new_fee_numerator = new_state.fee_numerator;
        let new_fee_denominator = new_state.fee_denominator;
        if self.fee_numerator != new_fee_numerator || 
           self.fee_denominator != new_fee_denominator {
            debug!("[RAYDIUM STATE] Pool {} fee update: {}/{} -> {}/{}", 
                   self.pool_address, self.fee_numerator, self.fee_denominator,
                   new_fee_numerator, new_fee_denominator);
            self.fee_numerator = new_fee_numerator;
            self.fee_denominator = new_fee_denominator;
            needs_update = true;
        }

        // Обновление слотов
        let new_recent_slot = new_state.recent_slot;
        if self.recent_slot != new_recent_slot {
            debug!("[RAYDIUM STATE] Pool {} recent_slot update: {} -> {}", 
                   self.pool_address, self.recent_slot, new_recent_slot);
            self.recent_slot = new_recent_slot;
            needs_update = true;
        }

        if needs_update {
            updated = true;
        }

        if updated {
            // Обновляем ребро в обоих графах
            for graph in [&GLOBAL_DATA.processed_graph, &GLOBAL_DATA.finalized_graph] {
                if let Some(mut g) = graph.get_mut("main") {
                    if let Some(edge_idx) = g.edge_indices()
                        .find(|&e| g[e].pool_address == self.pool_address) {
                        
                        let edge = &mut g[edge_idx];
                        let price = calculators::calculate_raydium_price(self.min_price, self.max_price);
                        let fee_rate = calculators::calculate_raydium_fee(
                            self.fee_numerator,
                            self.fee_denominator
                        );
                        let liquidity = self.total_lp as f64;
                        let weight = calculate_raydium_weight(
                            price,
                            fee_rate,
                            liquidity,
                            self.orders_num,
                            self.depth
                        );

                        edge.update_metrics(
                            price,
                            fee_rate,
                            liquidity,
                            weight,
                            self.status != 0 && self.pool_state != 0,
                            GLOBAL_DATA.network_state
                                .get("current")
                                .map(|s| s.current_slot)
                                .unwrap_or(0)
                        );

                        debug!("Обновлены метрики ребер для Raydium пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                            self.pool_address, price, fee_rate, liquidity, weight);
                    }
                }
            }
            // Обновляем все цепочки, использующие этот пул
            // TODO: ВЫЗОВ СТАРОЙ ФУНКЦИИ. ДОБАВИТЬ ТРИГГЕР ПЕРЕСЧЕТА ЦЕПОЧЕК
            // GLOBAL_DATA.update_affected_chains(self.pool_address);

            // НОВЫЙ ВЫЗОВ:
            RouterEngine::update_affected_chains(self.pool_address);
        }
        
        updated
    }

    pub fn from_raydium(pool_address: Pubkey, data: &RaydiumData) -> Self {
        Self {
            pool_address,
            status: data.status,
            pool_state: data.pool_state,
            amm_id: data.amm_id,
            market_id: data.market_id,
            token_a: data.token_a,
            token_b: data.token_b,
            lp_mint: data.lp_mint,
            open_orders: data.open_orders,
            needs_withdraw: data.needs_withdraw,
            recent_slot: data.recent_slot,
            last_order_slot: data.last_order_slot,
            total_lp: data.total_lp,
            base_need_take: data.base_need_take,
            quote_need_take: data.quote_need_take,
            base_decimal: data.base_decimal,
            quote_decimal: data.quote_decimal,
            min_price: data.min_price,
            max_price: data.max_price,
            vol_max_cut_ratio: data.vol_max_cut_ratio,
            fee_numerator: data.fee_numerator,
            fee_denominator: data.fee_denominator,
            ret_fee_numerator: data.ret_fee_numerator,
            ret_fee_denominator: data.ret_fee_denominator,
            punish_pc_amount: data.punish_pc_amount,
            punish_coin_amount: data.punish_coin_amount,
            orders_num: data.orders_num,
            depth: data.depth,
            open_time: data.open_time,
            switch_time: data.switch_time,
        }
    }
}

impl MeteoraPoolStateBase {
    pub fn update(&mut self, new_state: &MeteoraData) -> bool {
        let mut updated = false;
        let mut needs_update = false;

        // Проверка активности пула
        let new_is_active = new_state.authority != Pubkey::default() 
            && new_state.token_vault_a != Pubkey::default()
            && new_state.token_vault_b != Pubkey::default()
            && new_state.dynamic_liquidity_mode != 0
            && new_state.liquidity_cap > 0;

        // Обновление статуса активности
        if self.dynamic_liquidity_mode != new_state.dynamic_liquidity_mode || 
           (new_state.dynamic_liquidity_mode != 0 && !new_is_active) {
            debug!("[METEORA STATE] Pool {} activity changed: mode={}, is_active={}", 
                   self.pool_address, new_state.dynamic_liquidity_mode, new_is_active);
            
            // Обновляем режим ликвидности только если пул активен
            if new_is_active {
                self.dynamic_liquidity_mode = new_state.dynamic_liquidity_mode;
            } else {
                self.dynamic_liquidity_mode = 0; // Устанавливаем режим в 0 для неактивного пула
            }
            needs_update = true;
        }

        // Дополнительная проверка изменения authority
        if self.authority != new_state.authority {
            debug!("[METEORA STATE] Pool {} authority changed: {} -> {}", 
                   self.pool_address, self.authority, new_state.authority);
            self.authority = new_state.authority;
            needs_update = true;
        }

        // Обновление ликвидности
        let new_liquidity = new_state.liquidity;
        if self.liquidity != new_liquidity {
            debug!("[METEORA STATE] Pool {} liquidity update: {} -> {}", 
                   self.pool_address, self.liquidity, new_liquidity);
            self.liquidity = new_liquidity;
            needs_update = true;
        }

        // Обновление sqrt_price
        let new_sqrt_price = new_state.sqrt_price;
        if self.sqrt_price != new_sqrt_price {
            debug!("[METEORA STATE] Pool {} sqrt_price update: {} -> {}", 
                   self.pool_address, self.sqrt_price, new_sqrt_price);
            self.sqrt_price = new_sqrt_price;
            needs_update = true;
        }

        // Обновление текущего тика
        let new_current_tick_index = new_state.current_tick_index;
        if self.current_tick_index != new_current_tick_index {
            debug!("[METEORA STATE] Pool {} tick_index update: {} -> {}", 
                   self.pool_address, self.current_tick_index, new_current_tick_index);
            self.current_tick_index = new_current_tick_index;
            needs_update = true;
        }

        // Обновление множителя ликвидности
        let new_liquidity_multiplier = new_state.liquidity_multiplier;
        if self.liquidity_multiplier != new_liquidity_multiplier {
            debug!("[METEORA STATE] Pool {} multiplier update: {} -> {}", 
                self.pool_address, self.liquidity_multiplier, new_liquidity_multiplier);
            self.liquidity_multiplier = new_liquidity_multiplier;
            needs_update = true;
        }

        // Проверка диапазона тиков
        let new_max_tick_index = new_state.max_tick_index;
        let new_min_tick_index = new_state.min_tick_index;
        if self.max_tick_index != new_max_tick_index ||
        self.min_tick_index != new_min_tick_index {
            debug!("[METEORA STATE] Pool {} tick range update: [{}, {}]", 
                self.pool_address, new_min_tick_index, new_max_tick_index);
            self.max_tick_index = new_max_tick_index;
            self.min_tick_index = new_min_tick_index;
            needs_update = true;
        }

        // Обновление комиссий
        let new_fee_rate = new_state.fee_rate;
        if self.fee_rate != new_fee_rate {
            debug!("[METEORA STATE] Pool {} fee_rate update: {} -> {}", 
                   self.pool_address, self.fee_rate, new_fee_rate);
            self.fee_rate = new_fee_rate;
            needs_update = true;
        }

        // Обновление протокольных комиссий
        let new_protocol_fee_rate = new_state.protocol_fee_rate;
        if self.protocol_fee_rate != new_protocol_fee_rate {
            debug!("[METEORA STATE] Pool {} protocol_fee_rate update: {} -> {}", 
                   self.pool_address, self.protocol_fee_rate, new_protocol_fee_rate);
            self.protocol_fee_rate = new_protocol_fee_rate;
            needs_update = true;
        }

        // Обновление глобальных комиссий
        let new_fee_growth_global_a = new_state.fee_growth_global_a;
        if self.fee_growth_global_a != new_fee_growth_global_a {
            debug!("[METEORA STATE] Pool {} fee_growth_a update: {} -> {}", 
                   self.pool_address, self.fee_growth_global_a, new_fee_growth_global_a);
            self.fee_growth_global_a = new_fee_growth_global_a;
            needs_update = true;
        }

        // Обновление протокольных комиссий
        let new_fee_growth_global_b = new_state.fee_growth_global_b;
        if self.fee_growth_global_b != new_fee_growth_global_b {
            debug!("[METEORA STATE] Pool {} fee_growth_b update: {} -> {}", 
                   self.pool_address, self.fee_growth_global_b, new_fee_growth_global_b);
            self.fee_growth_global_b = new_fee_growth_global_b;
            needs_update = true;
        }

        // Обновление параметров динамической ликвидности
        let new_dynamic_liquidity_mode = new_state.dynamic_liquidity_mode;
        if self.dynamic_liquidity_mode != new_dynamic_liquidity_mode {
            debug!("[METEORA STATE] Pool {} dynamic_mode update: {} -> {}", 
                   self.pool_address, self.dynamic_liquidity_mode, new_dynamic_liquidity_mode);
            self.dynamic_liquidity_mode = new_dynamic_liquidity_mode;
            needs_update = true;
        }

        // Обновление метрик
        let new_volume_24h = new_state.volume_24h;
        if self.volume_24h != new_volume_24h {
            debug!("[METEORA STATE] Pool {} volume_24h update: {} -> {}", 
                   self.pool_address, self.volume_24h, new_volume_24h);
            self.volume_24h = new_volume_24h;
            needs_update = true;
        }

        // Обновление комиссий за 24 часа
        let new_fees_24h = new_state.fees_24h;
        if self.fees_24h != new_fees_24h {
            debug!("[METEORA STATE] Pool {} fees_24h update: {} -> {}", 
                   self.pool_address, self.fees_24h, new_fees_24h);
            self.fees_24h = new_fees_24h;
            needs_update = true;
        }

        if needs_update {
            updated = true;
        }

        if updated {
            for graph in [&GLOBAL_DATA.processed_graph, &GLOBAL_DATA.finalized_graph] {
                if let Some(mut g) = graph.get_mut("main") {
                    if let Some(edge_idx) = g.edge_indices()
                        .find(|&e| g[e].pool_address == self.pool_address) {
                        
                        let edge = &mut g[edge_idx];
                        let price = calculators::calculate_meteora_price(self.sqrt_price);
                        let fee_rate = self.fee_rate as f64 / 10_000.0;
                        let liquidity = self.liquidity as f64;
                        let weight = calculate_meteora_weight(
                            price,
                            fee_rate,
                            liquidity,
                            self.liquidity_multiplier
                        );

                        edge.update_metrics(
                            price,
                            fee_rate,
                            liquidity,
                            weight,
                            self.dynamic_liquidity_mode != 0 && self.liquidity_cap > 0,
                            GLOBAL_DATA.network_state
                                .get("current")
                                .map(|s| s.current_slot)
                                .unwrap_or(0)
                        );

                        debug!("Обновлены метрики ребер для Meteora пула {}: price={}, fee_rate={}, liquidity={}, weight={}", 
                            self.pool_address, price, fee_rate, liquidity, weight);
                    }
                }
            }
            // Обновляем все цепочки, использующие этот пул
            // TODO: ВЫЗОВ СТАРОЙ ФУНКЦИИ. ДОБАВИТЬ ТРИГГЕР ПЕРЕСЧЕТА ЦЕПОЧЕК
            // GLOBAL_DATA.update_affected_chains(self.pool_address);

            // НОВЫЙ ВЫЗОВ:
            RouterEngine::update_affected_chains(self.pool_address);
        }
        
        updated
    }

    pub fn from_meteora(pool_address: Pubkey, data: &MeteoraData) -> Self {
        Self {
            pool_address,
            pool_id: data.pool_id,
            authority: data.authority,
            token_mint_a: data.token_mint_a,
            token_mint_b: data.token_mint_b,
            token_vault_a: data.token_vault_a,
            token_vault_b: data.token_vault_b,
            lp_mint: data.lp_mint,
            total_lp: data.total_lp,
            liquidity: data.liquidity,
            sqrt_price: data.sqrt_price,
            current_tick_index: data.current_tick_index,
            tick_spacing: data.tick_spacing,
            fee_rate: data.fee_rate,
            protocol_fee_rate: data.protocol_fee_rate,
            fee_growth_global_a: data.fee_growth_global_a,
            fee_growth_global_b: data.fee_growth_global_b,
            fee_protocol_token_a: data.fee_protocol_token_a,
            fee_protocol_token_b: data.fee_protocol_token_b,
            max_price_sqrt: data.max_price_sqrt,
            min_price_sqrt: data.min_price_sqrt,
            max_tick_index: data.max_tick_index,
            min_tick_index: data.min_tick_index,
            dynamic_liquidity_mode: data.dynamic_liquidity_mode,
            liquidity_cap: data.liquidity_cap,
            liquidity_multiplier: data.liquidity_multiplier,
            last_update_timestamp: data.last_update_timestamp,
            last_update_slot: data.last_update_slot,
            volume_24h: data.volume_24h,
            fees_24h: data.fees_24h,
        }
    }
}