// src/data.rs

use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use lazy_static::lazy_static;
use dashmap::{DashMap, DashSet};
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;

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

// Структура для хранения состояния пула
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolState {
    pub last_update: u64,
    pub reserves: (u64, u64),
    pub last_price: f64,
    pub volume_24h: u64,
}

// Структура для хранения метрик пула
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub tvl: f64,
    pub volume_24h: f64,
    pub fees_24h: f64,
    pub price_impact: f64,
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
    pub pool_states: Arc<DashMap<Pubkey, PoolState>>,
    // Граф ликвидности
    pub liquidity_edges: Arc<DashMap<Pubkey, Vec<LiquidityEdge>>>,
    // Кэш популярных маршрутов
    pub route_cache: Arc<DashMap<(Pubkey, Pubkey), Vec<Route>>>,
    // Метрики пулов
    pub pool_metrics: Arc<DashMap<Pubkey, PoolMetrics>>,
}

// Глобальный экземпляр данных
lazy_static! {
    pub static ref GLOBAL_DATA: GlobalData = GlobalData {
        tokens: Arc::new(DashMap::new()),
        token_addresses: Arc::new(DashMap::new()),
        token_pairs: Arc::new(DashSet::new()),
        orca_pools: Arc::new(DashMap::new()),
        pool_states: Arc::new(DashMap::new()),
        liquidity_edges: Arc::new(DashMap::new()),
        route_cache: Arc::new(DashMap::new()),
        pool_metrics: Arc::new(DashMap::new()),
    };
}

// Вспомогательная функция для получения unix timestamp
fn unix_timestamp() -> u64 {
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

    // Обновление состояния пула
    #[allow(dead_code)]
    pub fn update_pool_state(
        &self,
        pool_address: Pubkey,
        reserves: (u64, u64),
        price: f64
    ) {
        debug!("Обновление состояния пула {}: reserves={:?}, price={}", 
               pool_address, reserves, price);
        
        let state = PoolState {
            last_update: unix_timestamp(),
            reserves,
            last_price: price,
            volume_24h: 0,
        };
        self.pool_states.insert(pool_address, state);
    }

    // Добавление ребра ликвидности
    #[allow(dead_code)]
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

    // Обновление кэша популярных маршрутов
    #[allow(dead_code)]
    pub fn update_route_cache(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        routes: Vec<Route>
    ) {
        debug!("Обновление кэша маршрутов для {} -> {}", token_in, token_out);
        self.route_cache.insert((token_in, token_out), routes);
    }

    // Поиск оптимального маршрута
    #[allow(dead_code)]
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
}
