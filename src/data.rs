// src/data.rs

use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use lazy_static::lazy_static;
use dashmap::{DashMap, DashSet};
use log::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use crate::ws_orca::SlotInfo;
use crate::decoder::WhirlpoolData;

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
// Структура для хранения информации о пуле Orca
#[derive(Debug, Clone)]
pub struct OrcaPoolInfo {
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

// Структура для хранения информации о маршруте
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Route {
    pub hops: Vec<LiquidityEdge>,
    pub total_fee: u64,
    pub estimated_price_impact: f64,
}

// Базовая структура состояния пула
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OrcaPoolStateBase {
    // Основные идентификаторы
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

// TODO: Метрики для MEV

// pub price_impact_threshold: f64,
// pub execution_probability: f64,
// pub slippage_tolerance: f64,

// TODO: Добавить структуры для geyser Mev :
// pub struct PriceImpactEvent;  // Событие влияния на цену
// pub struct LiquidityEvent;    // Событие изменения ликвидности
// pub struct ArbitrageEvent;    // Событие арбитражной возможности

// Состояние для finalized данных (programSubscribe)
#[derive(Debug, Clone)]
pub struct FinalizedOrcaPoolState {
    pub base: OrcaPoolStateBase,
    pub finalized_slot: u64,
    pub last_update_time: u64,
}

// Состояние для processed данных (accountSubscribe)
#[derive(Debug, Clone)]
pub struct ProcessedOrcaPoolState {
    pub base: OrcaPoolStateBase,
    pub processed_slot: u64,
    pub last_update_time: u64,
}

// Добавляем структуру для состояния сети
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

// Глобальная структура данных
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct GlobalData {
    // Маппинг символ -> информация о токене
    pub tokens: Arc<DashMap<String, TokenInfo>>,
    // Маппинг адрес -> информация о токене
    pub token_addresses: Arc<DashMap<Pubkey, TokenInfo>>,
    // Множество всех пар токенов
    pub token_pairs: Arc<DashSet<TokenPair>>,
    // Маппинг пары токенов -> список пулов
    pub orca_pools: Arc<DashMap<TokenPair, Vec<OrcaPoolInfo>>>,
    // Кэш состояний пулов
    pub finalized_pool_states: Arc<DashMap<Pubkey, FinalizedOrcaPoolState>>,
    pub processed_pool_states: Arc<DashMap<Pubkey, ProcessedOrcaPoolState>>,
    // Граф ликвидности
    pub liquidity_edges: Arc<DashMap<Pubkey, Vec<LiquidityEdge>>>,
    // Кэш популярных маршрутов
    pub route_cache: Arc<DashMap<(Pubkey, Pubkey), Vec<Route>>>,
    // Добавляем в GlobalData
    pub network_state: Arc<DashMap<String, NetworkState>>,
}

// Глобальный экземпляр данных
lazy_static! {
    pub static ref GLOBAL_DATA: GlobalData = GlobalData {
        tokens: Arc::new(DashMap::new()),
        token_addresses: Arc::new(DashMap::new()),
        token_pairs: Arc::new(DashSet::new()),
        orca_pools: Arc::new(DashMap::new()),
        finalized_pool_states: Arc::new(DashMap::new()),
        processed_pool_states: Arc::new(DashMap::new()),
        liquidity_edges: Arc::new(DashMap::new()),
        route_cache: Arc::new(DashMap::new()),
        network_state: Arc::new(DashMap::new()),
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
    // Добавление токена
    pub fn add_token(&self, symbol: String, address: Pubkey) {
        let token_info = TokenInfo {
            symbol: symbol.clone(),
            address,
        };
        
        debug!("Добавление токена: {} с адресом {}", symbol, address);
        
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
            
            debug!("Добавление пары токенов: {:?}", pair);
            self.token_pairs.insert(pair);
        }
    }

    // Добавление пула Orca
    pub fn add_orca_pool(
        &self,
        token_a_symbol: String,
        token_b_symbol: String,
        pool_address: Pubkey,
        _token_a_address: Pubkey,  // Теперь эти параметры не нужны, так как информация
        _token_b_address: Pubkey,  // о токенах уже есть в TokenInfo
        tvl: f64,
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

            info!("Добавление пула Orca для пары {:?} с адресом {}", pair, pool_address);
            
            // Добавляем пул в список пулов для данной пары
            self.orca_pools.entry(pair).or_default().push(OrcaPoolInfo {
                pool_address,
            });
            
            true
        } else {
            debug!("Не найдена информация о токенах для пары {}-{}", token_a_symbol, token_b_symbol);
            false
        }
    }

    #[allow(dead_code)]
    // Вспомогательные методы для получения информации про пул через токен символ
    pub fn get_pools_by_token_symbol(&self, symbol: &str) -> Vec<(TokenPair, Vec<OrcaPoolInfo>)> {
        let mut result = Vec::new();
        
        for entry in self.orca_pools.iter() {
            let pair = entry.key();
            if pair.token_a.symbol == symbol || pair.token_b.symbol == symbol {
                result.push((pair.clone(), entry.value().clone()));
            }
        }
        
        result
    }

    #[allow(dead_code)]
    // Вспомогательные методы для получения информации про пул через токен адресс
    pub fn get_pools_by_token_address(&self, address: &Pubkey) -> Vec<(TokenPair, Vec<OrcaPoolInfo>)> {
        let mut result = Vec::new();
        
        for entry in self.orca_pools.iter() {
            let pair = entry.key();
            if pair.token_a.address == *address || pair.token_b.address == *address {
                result.push((pair.clone(), entry.value().clone()));
            }
        }
        
        result
    }

    #[allow(dead_code)]
    // Добавление ребра ликвидности
    // TODO: Реализовать логику для добавления ребра ликвидности
    pub fn add_liquidity_edge(
        &self,
        edge: LiquidityEdge
    ) {
        debug!("Добавление ребра ликвидности: {} -> {}", 
               edge.token_in.symbol, edge.token_out.symbol);
               
        self.liquidity_edges
            .entry(edge.token_in.address)
            .or_default()
            .push(edge);
    }

    #[allow(dead_code)]
    // Обновление кэша популярных маршрутов
    // TODO: Реализовать логику для обновления кэша популярных маршрутов
    pub fn update_route_cache(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        routes: Vec<Route>
    ) {
        debug!("Обновление кэша маршрутов для {} -> {}", token_in, token_out);
        self.route_cache.insert((token_in, token_out), routes);
    }

    #[allow(dead_code)]
    // Поиск оптимального маршрута
    // TODO: Реализовать логику для поиска оптимального маршрута
    pub fn find_best_route(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        amount: u64
    ) -> Option<Route> {
        debug!("Поиск оптимального маршрута {} -> {} для {}", 
               token_in, token_out, amount);
        None // TODO: имплементировать алгоритм поиска
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
                warn!("Processing outdated data: current_slot={}, update_slot={}", 
                      state.current_slot, update_slot);
                return false;
            }
            true
        } else {
            // Если состояние еще не инициализировано, пропускаем валидацию
            true
        }
    }
}

// Общая логика обновления для базового состояния
impl OrcaPoolStateBase {
    // Обновление состояния пула для Orca
    // TODO: добавить дополнительные обязательные переменные для MEV бота
        
    pub fn update(&mut self, new_state: &WhirlpoolData) -> bool {
        let mut updated = false;
        
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
            info!("[STATE] Pool {} activity changed: {} -> {}", 
                  self.pool_address, self.is_active, new_is_active);
            self.is_active = new_is_active;
            updated = true;
        }

        // Обновление ценовых параметров
        let new_sqrt_price = new_state.sqrt_price;
        if self.sqrt_price != new_sqrt_price {
            debug!("[STATE] Pool {} sqrt_price update: {} -> {}", 
                   self.pool_address, self.sqrt_price, new_sqrt_price);
            self.sqrt_price = new_sqrt_price;
            updated = true;
        }

        // Обновление пороговой цены
        let new_price_threshold = new_state.price_threshold;
        if self.price_threshold != new_price_threshold {
            debug!("[STATE] Pool {} price_threshold update: {} -> {}", 
                   self.pool_address, self.price_threshold, new_price_threshold);
            self.price_threshold = new_price_threshold;
            updated = true;
        }

        // Обновление ликвидности
        let new_liquidity = new_state.liquidity;
        if self.liquidity != new_liquidity {
            debug!("[STATE] Pool {} liquidity update: {} -> {}", 
                   self.pool_address, self.liquidity, new_liquidity);
            self.liquidity = new_liquidity;
            updated = true;
        }

        // Обновление тика
        let new_tick_current_index = new_state.tick_current_index;
        if self.tick_current_index != new_tick_current_index {
            debug!("[STATE] Pool {} tick_index update: {} -> {}", 
                   self.pool_address, self.tick_current_index, new_tick_current_index);
            self.tick_current_index = new_tick_current_index;
            updated = true;
        }

        // Обновление комиссий
        let new_fee_rate = new_state.fee_rate;
        if self.fee_rate != new_fee_rate {
            debug!("[STATE] Pool {} fee_rate update: {} -> {}", 
                   self.pool_address, self.fee_rate, new_fee_rate);
            self.fee_rate = new_fee_rate;
            updated = true;
        }

        // Обновление протокольной комиссии
        let new_protocol_fee_rate = new_state.protocol_fee_rate;
        if self.protocol_fee_rate != new_protocol_fee_rate {
            debug!("[STATE] Pool {} protocol_fee_rate update: {} -> {}", 
                   self.pool_address, self.protocol_fee_rate, new_protocol_fee_rate);
            self.protocol_fee_rate = new_protocol_fee_rate;
            updated = true;
        }

        // Обновление накопленных комиссий
        let new_fee_growth_global_a = new_state.fee_growth_global_a;
        if self.fee_growth_global_a != new_fee_growth_global_a {
            debug!("[STATE] Pool {} fee_growth_a update: {} -> {}", 
                   self.pool_address, self.fee_growth_global_a, new_fee_growth_global_a);
            self.fee_growth_global_a = new_fee_growth_global_a;
            updated = true;
        }

        // Обновление накопленных комиссий для токена B
        let new_fee_growth_global_b = new_state.fee_growth_global_b;
        if self.fee_growth_global_b != new_fee_growth_global_b {
            debug!("[STATE] Pool {} fee_growth_b update: {} -> {}", 
                   self.pool_address, self.fee_growth_global_b, new_fee_growth_global_b);
            self.fee_growth_global_b = new_fee_growth_global_b;
            updated = true;
        }

        // Обновление протокольных комиссий
        let new_protocol_fee_owed_a = new_state.protocol_fee_owed_a;
        if self.protocol_fee_owed_a != new_protocol_fee_owed_a {
            debug!("[STATE] Pool {} protocol_fee_owed_a update: {} -> {}", 
                   self.pool_address, self.protocol_fee_owed_a, new_protocol_fee_owed_a);
            self.protocol_fee_owed_a = new_protocol_fee_owed_a;
            updated = true;
        }

        // Обновление протокольных комиссий для токена B
        let new_protocol_fee_owed_b = new_state.protocol_fee_owed_b;
        if self.protocol_fee_owed_b != new_protocol_fee_owed_b {
            debug!("[STATE] Pool {} protocol_fee_owed_b update: {} -> {}", 
                   self.pool_address, self.protocol_fee_owed_b, new_protocol_fee_owed_b);
            self.protocol_fee_owed_b = new_protocol_fee_owed_b;
            updated = true;
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

// Реализация для finalized состояния
impl FinalizedOrcaPoolState {
    pub fn update(&mut self, new_state: &WhirlpoolData, slot: u64) -> bool {
        let updated = self.base.update(new_state);
        if updated {
            self.finalized_slot = slot;
            self.last_update_time = unix_timestamp();
            info!("[FINALIZED] Pool {} state updated at slot {} ({})", 
                  self.base.pool_address, slot, self.last_update_time);
        }
        updated
    }
}

// Реализация для processed состояния
impl ProcessedOrcaPoolState {
    pub fn update(&mut self, new_state: &WhirlpoolData, slot: u64) -> bool {
        let updated = self.base.update(new_state);
        if updated {
            self.processed_slot = slot;
            self.last_update_time = unix_timestamp();
            info!("[PROCESSED] Pool {} state updated at slot {} ({})", 
                  self.base.pool_address, slot, self.last_update_time);
        }
        updated
    }
}