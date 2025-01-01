// src/graph.rs

#[allow(unused_imports)]
use log::{info, error, debug, warn};
use hashbrown::{HashSet, HashMap};
use petgraph::Graph;
use solana_program::pubkey::Pubkey;

use crate::data::GLOBAL_DATA;
use crate::websocket::ws_data::DexType;

const MIN_LEN: usize = crate::config::MIN_CHAIN_LENGTH; 
const MAX_LEN: usize = crate::config::MAX_CHAIN_LENGTH;

// Структура для хранения информации о пуле и его весе
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolEdge {
    pub pool_address: Pubkey,
    pub weight: f64,
    // Все метрики, влияющие на вес (цена, ликвидность, комиссия и т.д.)
    pub price: f64,
    pub fee_rate: f64,
    pub liquidity: f64,
    pub is_active: bool,
    pub current_amount: u64,
    pub chain_position: Option<usize>,
    pub last_update_slot: u64,
    pub last_update_time: u64, // Добавляем для отслеживания времени
}

impl PoolEdge {
    #[allow(dead_code)]
    pub fn new(pool_address: Pubkey) -> Self {
        Self {
            pool_address,
            price: 0.0,
            fee_rate: 0.0,
            liquidity: 0.0,
            weight: 0.0,
            is_active: true,
            current_amount: 0,
            chain_position: None,
            last_update_slot: 0,
            last_update_time: crate::data::unix_timestamp(),
        }
    }

    pub fn update_metrics(&mut self, 
        price: f64, 
        fee_rate: f64, 
        liquidity: f64,
        weight: f64,
        is_active: bool,
        slot: u64
    ) {
        self.price = price;
        self.fee_rate = fee_rate;
        self.liquidity = liquidity;
        self.weight = weight;
        self.is_active = is_active;
        self.last_update_slot = slot;
        self.last_update_time = crate::data::unix_timestamp();
    }
}

// Добавим новую структуру для хранения валидных пар
#[derive(Debug)]
struct ValidPairs {
    pairs: HashSet<(String, String)>,
}

impl ValidPairs {
    fn new() -> Self {
        Self {
            pairs: HashSet::new()
        }
    }

    // Добавляем пару в обоих направлениях, так как они изоморфны
    fn add_pair(&mut self, token1: &str, token2: &str) {
        self.pairs.insert((token1.to_string(), token2.to_string()));
        self.pairs.insert((token2.to_string(), token1.to_string()));
    }

    // Проверяем, что пара валидна
    fn is_valid_pair(&self, token1: &str, token2: &str) -> bool {
        self.pairs.contains(&(token1.to_string(), token2.to_string())) ||
        self.pairs.contains(&(token2.to_string(), token1.to_string()))
    }
}

// Основная функция для построения и поиска цепочек
pub fn build_and_find_chains() {
    info!("Начинаем инициализацию графов");

    // Явно создаем графы
    GLOBAL_DATA.processed_graph.insert("main".to_string(), Graph::new());
    GLOBAL_DATA.finalized_graph.insert("main".to_string(), Graph::new());

    // Валидируем токены и строим цепочки
    let initial_tokens = crate::config::INITIAL_TOKENS;
    let valid_tokens = validate_tokens_across_dex(&initial_tokens);
    
    if valid_tokens.len() != initial_tokens.len() {
        warn!("Обнаружены невалидные токены. Продолжаем только с {} из {} токенов", valid_tokens.len(), initial_tokens.len());
    }

    // Проверяем что стартовый токен валидный
    let start_token = crate::config::START_END_TOKEN_FOR_CHAINS[0];
    if !valid_tokens.contains(&start_token) {
        error!("Стартовый токен {} не валиден! Построение цепочек невозможно", start_token);
        return;
    }

    // Подготавливаем все группы изоморфных пар
    // Каждая группа: ((tokenA, tokenB), (tokenB, tokenA))
    let mut isomorphic_pairs = vec![];
    for (i, &t1) in valid_tokens.iter().enumerate() {
        for &t2 in &valid_tokens[i+1..] {
            // Каждые два токена порождают группу изоморфных пар
            isomorphic_pairs.push(((t1, t2), (t2, t1)));
        }
    }

    // Вектора для хранения цепочек разной длины (4,5)
    let mut chains_by_4: Vec<Vec<String>> = Vec::new();
    let mut chains_by_5: Vec<Vec<String>> = Vec::new();

    // Создаем структуру для хранения валидных пар
    let mut valid_pairs = ValidPairs::new();
    
    // Собираем все существующие пары из пулов
    for dex_type in [DexType::Orca, DexType::Raydium, DexType::Meteora] {
        if let Some(dex_pools) = GLOBAL_DATA.dex_pools.get(&dex_type) {
            for pair in dex_pools.iter() {
                valid_pairs.add_pair(&pair.key().token_a.symbol, &pair.key().token_b.symbol);
            }
        }
    }

    /*
    info!("Найдены следующие валидные пары токенов:");
    for (token1, token2) in &valid_pairs.pairs {
        info!("  {} <-> {}", token1, token2);
    }
    */

    // Модифицируем функцию DFS для проверки валидности пар
    fn dfs(
        current_token: &str,
        chain: &mut Vec<String>,
        used_groups: &mut Vec<usize>,
        pairs: &Vec<((&str, &str), (&str, &str))>,
        chains_4: &mut Vec<Vec<String>>,
        chains_5: &mut Vec<Vec<String>>,
        valid_pairs: &ValidPairs,  // Добавляем valid_pairs
    ) {
        let chain_len = chain.len();

        // Проверяем валидность цепочки при достижении нужной длины
        if chain_len >= MIN_LEN && current_token == crate::config::START_END_TOKEN_FOR_CHAINS[0] && chain_len > 1 {
            // Проверяем, что все последовательные пары в цепочке валидны
            let mut is_valid_chain = true;
            for i in 0..chain_len-1 {
                if !valid_pairs.is_valid_pair(&chain[i], &chain[i+1]) {
                    is_valid_chain = false;
                    warn!("Невалидная пара в цепочке: {} -> {}. Возможна проблема с ликвидностью или активностью пула", chain[i], chain[i+1]);
                    break;
                }
            }

            if is_valid_chain {
                match chain_len {
                    4 => {
                        // info!("Найдена валидная цепочка длины 4: {:?}", chain);
                        chains_4.push(chain.clone());
                    }
                    5 => {
                        // info!("Найдена валидная цепочка длины 5: {:?}", chain);
                        chains_5.push(chain.clone());
                    }
                    _ => {}
                }
            }
        }

        // Если достигли максимальной длины токенов, завершаем ветку
        if chain_len == MAX_LEN {
            return;
        }

        // Перебираем все группы изоморфных пар
        for (group_idx, group) in pairs.iter().enumerate() {
            // Проверяем, не использовалась ли эта группа ранее
            if used_groups.contains(&group_idx) {
                continue;
            }

            let (direct, inverse) = group;
            // direct = (tokenA, tokenB)
            // inverse = (tokenB, tokenA)

            // Проверяем валидность пары перед добавлением в цепочку
            if direct.0 == current_token && valid_pairs.is_valid_pair(direct.0, direct.1) {
                let next_token = direct.1;
                used_groups.push(group_idx);
                chain.push(next_token.to_string());

                // info!("Шаг -> {:?} => {:?}", current_token, next_token);

                dfs(
                    next_token,
                    chain,
                    used_groups,
                    pairs,
                    chains_4,
                    chains_5,
                    valid_pairs,  // Передаем valid_pairs
                );

                // Откатываем состояние
                chain.pop();
                used_groups.pop();
            }

            // Если текущий токен - direct.1 (перевёрнутое направление)
            if direct.1 == current_token && valid_pairs.is_valid_pair(direct.1, direct.0) {
                let next_token = direct.0;
                used_groups.push(group_idx);
                chain.push(next_token.to_string());

                // info!("Шаг -> {:?} => {:?}", current_token, next_token);

                dfs(
                    next_token,
                    chain,
                    used_groups,
                    pairs,
                    chains_4,
                    chains_5,
                    valid_pairs,  // Передаем valid_pairs
                );

                chain.pop();
                used_groups.pop();
            }

            // Аналогично для inverse
            if inverse.0 == current_token && valid_pairs.is_valid_pair(inverse.0, inverse.1) {
                let next_token = inverse.1;
                used_groups.push(group_idx);
                chain.push(next_token.to_string());

                // info!("Шаг -> {:?} => {:?}", current_token, next_token);

                dfs(
                    next_token,
                    chain,
                    used_groups,
                    pairs,
                    chains_4,
                    chains_5,
                    valid_pairs,  // Передаем valid_pairs
                );

                chain.pop();
                used_groups.pop();
            }

            if inverse.1 == current_token && valid_pairs.is_valid_pair(inverse.1, inverse.0) {
                let next_token = inverse.0;
                used_groups.push(group_idx);
                chain.push(next_token.to_string());

                // info!("Шаг -> {:?} => {:?}", current_token, next_token);

                dfs(
                    next_token,
                    chain,
                    used_groups,
                    pairs,
                    chains_4,
                    chains_5,
                    valid_pairs,  // Передаем valid_pairs
                );

                chain.pop();
                used_groups.pop();
            }
        }
    }

    info!("Начинаем построение цепочек на основе valid_tokens: {:?}", valid_tokens);

    // Запускаем DFS, начиная с start_token
    let mut current_chain = vec![start_token.to_string()];
    let mut used_groups = vec![];
    dfs(
        start_token,
        &mut current_chain,
        &mut used_groups,
        &isomorphic_pairs,
        &mut chains_by_4,
        &mut chains_by_5,
        &valid_pairs,  // Передаем valid_pairs
    );

    // Удаляем дубликаты перед финальным логированием
    deduplicate(&mut chains_by_4);
    deduplicate(&mut chains_by_5);

    info!("Сохранение найденных цепочек в глобальную структуру данных");
    
    // Сохраняем цепочки длины 4 в оба хранилища
    for (i, chain) in chains_by_4.iter().enumerate() {
        GLOBAL_DATA.chain_storage_4.insert(i, chain.clone());
        GLOBAL_DATA.chains_4.insert(chain.clone());
        // debug!("Сохранили цепочку длины 4 [{}]: {:?}", i, chain);
    }
    
    let offset_4 = chains_by_4.len();
    
    // Сохраняем цепочки длины 5 в оба хранилища
    for (i, chain) in chains_by_5.iter().enumerate() {
        GLOBAL_DATA.chain_storage_5.insert(i, chain.clone());
        GLOBAL_DATA.chains_5.insert(chain.clone());
        // debug!("Сохранили цепочку длины 5 [{}]: {:?}", i, chain);
    }

    info!("Построение графа и поиск цепочек завершено. После фильтрации - всего уникальных цепочек длины 4: {} и длины 5: {}", GLOBAL_DATA.chains_4.len(), GLOBAL_DATA.chains_5.len());    

    // Индексируем цепочки для быстрого поиска
    for (chain_idx, chain) in chains_by_4.iter().enumerate() {
        for window in chain.windows(2) {
            let token_a = &window[0];
            let token_b = &window[1];
            
            // Проверяем все DEX для этой пары
            for dex in [DexType::Orca, DexType::Raydium, DexType::Meteora] {
                if let Some(pool_address) = GLOBAL_DATA.find_pool_address_by_symbols(dex, token_a, token_b) {
                    GLOBAL_DATA.chain_references
                        .entry(pool_address)
                        .or_insert_with(Vec::new)
                        .push(chain_idx); // Прямой индекс для цепочек длины 4
                }
            }
        }
    }

    // Индексируем цепочки длины 5 со смещением
    for (chain_idx, chain) in chains_by_5.iter().enumerate() {
        for window in chain.windows(2) {
            let token_a = &window[0];
            let token_b = &window[1];
            
            for dex in [DexType::Orca, DexType::Raydium, DexType::Meteora] {
                if let Some(pool_address) = GLOBAL_DATA.find_pool_address_by_symbols(dex, token_a, token_b) {
                    GLOBAL_DATA.chain_references
                        .entry(pool_address)
                        .or_insert_with(Vec::new)
                        .push(offset_4 + chain_idx); // Индекс со смещением для цепочек длины 5
                }
            }
        }
    }

    // После создания цепочек инициализируем графы
    for graph in [&GLOBAL_DATA.processed_graph, &GLOBAL_DATA.finalized_graph] {
        if let Some(mut g) = graph.get_mut("main") {
            // Для каждой цепочки создаем ребра
            for chain in chains_by_4.iter().chain(chains_by_5.iter()) {
                for window in chain.windows(2) {
                    let token1 = &window[0];
                    let token2 = &window[1];
                    
                    // Находим все пулы для этой пары
                    for dex_type in [DexType::Orca, DexType::Raydium, DexType::Meteora] {
                        if let Some(pool_address) = GLOBAL_DATA.find_pool_address_by_symbols(dex_type, token1, token2) {
                            // Добавляем вершины если их еще нет
                            let idx1 = g.add_node(token1.clone());
                            let idx2 = g.add_node(token2.clone());
                            // debug!("Добавили вершины: {} -> {} из graph.rs файл", token1, token2);
                            
                            // Создаем базовое ребро
                            g.add_edge(idx1, idx2, PoolEdge {
                                pool_address,
                                weight: 0.0,
                                price: 0.0,
                                fee_rate: 0.0,
                                liquidity: 0.0,
                                is_active: true,
                                current_amount: 0,
                                chain_position: None,
                                last_update_slot: 0,
                                last_update_time: 0,
                            });
                        }
                    }
                }
            }
        }
    }
    GLOBAL_DATA.validate_graphs();
    info!("Инициализация графов завершена");
}

// Добавим небольшую функцию для удаления дубликатов
fn deduplicate(chains: &mut Vec<Vec<String>>) {
    let mut seen = HashSet::new();
    chains.retain(|chain| seen.insert(chain.clone()));
}

// Валидация токенов - достаточно наличия пула хотя бы в одном DEX
fn validate_tokens_across_dex<'a>(initial_tokens: &'a [&str]) -> Vec<&'a str> {
    let dex_types = [DexType::Orca, DexType::Raydium, DexType::Meteora];
    
    // Для каждого токена храним статус по каждому DEX
    let mut token_validity: HashMap<&str, HashMap<DexType, bool>> = HashMap::new();
    
    // Инициализация структуры
    for &token in initial_tokens {
        let mut dex_status = HashMap::new();
        for &dex in &dex_types {
            dex_status.insert(dex, false);
        }
        token_validity.insert(token, dex_status);
    }

    // Проверяем каждую пару токенов в каждом DEX
    for (i, &token1) in initial_tokens.iter().enumerate() {
        for &token2 in initial_tokens.iter().skip(i + 1) {
            for &dex in &dex_types {
                if let Some(_pool_address) = GLOBAL_DATA.find_pool_address_by_symbols(dex, token1, token2) {
                    // Если нашли пул, помечаем оба токена как валидные для этого DEX
                    token_validity.get_mut(token1).unwrap().insert(dex, true);
                    token_validity.get_mut(token2).unwrap().insert(dex, true);
                }
            }
        }
    }

    // Собираем валидные токены (достаточно иметь пул хотя бы в одном DEX)
    let valid_tokens: Vec<&str> = initial_tokens
        .iter()
        .filter(|&&token| {
            let dex_statuses = token_validity.get(token).unwrap();
            let is_valid = dex_types.iter().any(|dex| *dex_statuses.get(dex).unwrap());
            
            if !is_valid {
                // info!("Токен {} не имеет пулов ни в одном из DEX", token);
            } else {
                #[allow(unused_variables)]
                let active_dex: Vec<DexType> = dex_types
                    .iter()
                    .filter(|&&dex| *dex_statuses.get(&dex).unwrap())
                    .copied()
                    .collect();
                
                // : {:?}", token, active_dex);
            }
            
            is_valid
        })
        .copied()
        .collect();

    info!("Валидация токенов завершена. Валидных токенов (имеющих пул хотя бы в одном DEX): {}/{}", 
          valid_tokens.len(), initial_tokens.len());

    valid_tokens
}