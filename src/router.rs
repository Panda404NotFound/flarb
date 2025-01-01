// src/router.rs

// src/router.rs

use log::{debug, warn, info};
use solana_program::pubkey::Pubkey;
use petgraph::graph::EdgeIndex;
use crate::graph::PoolEdge;
use crate::data::{GLOBAL_DATA, TokenInfo, unix_timestamp};
use crate::websocket::ws_data::DexType;
use crate::math::weight_calculators::*;
use crate::math::calculators::*;
use crate::config::INITIAL_BALANCE;

// -----------------------------------------
// Опционально: Структуры для хранения
// более "полных" данных внутри hops
// -----------------------------------------
#[derive(Debug, Clone)]
pub struct ExtendedPoolInfo {
    pub pool_address: Pubkey,
    pub price: f64,
    pub fee_rate: f64,
    pub liquidity: f64,
    pub weight: f64,
    pub is_active: bool,
    pub last_update_slot: u64,
    pub last_update_time: u64,
}

/// HopData расширяем, чтобы вместо (Pubkey, f64) хранился список ExtendedPoolInfo
#[derive(Debug, Clone)]
pub struct HopData {
    pub from_token: String,
    pub to_token: String,

    /// Все пулы для данного хопа
    pub pools: Vec<ExtendedPoolInfo>,

    /// Ссылка на лучший пул (по текущему весу или любой другой метрике)
    pub best_pool: Option<ExtendedPoolInfo>,
}

// TODO: Добавить в GlobalData 
#[derive(Debug, Clone)]
pub struct ChainResult {
    pub last_update: u64,
    pub chain_tokens: Vec<String>,  // Храним, какие вообще токены идут в цепочке

    // Список хопов, где внутри уже не просто (Pubkey, weight),
    // а более детальная информация о каждом пуле (PoolEdge).
    pub hops: Vec<HopData>,

    /// TODO: Здесь можно хранить "общий" результат по цепочке:
    pub total_weight: f64,
    pub simulated_amount: u64,
}

/// Движок-модуль для работы с цепочками:
/// 1) Изначально связывает цепочки (tokens) с реальными ребрами PoolEdge.
/// 2) Позже можем считать лучший маршрут, моделировать сделки, т.д.
pub struct RouterEngine;

impl RouterEngine {
    /// Инициализирует/пересчитывает одну конкретную цепочку:
    /// - Собирает HopData (добавляет все пулы = PoolEdge) для каждого (tokenA -> tokenB).
    /// - Находит best_pool (по весу).
    /// - Вычисляет total_weight и т. п. (пока упрощённо).
    pub fn recalc_chain(
        chain: &[String],
        is_finalized: bool
    ) -> Option<ChainResult> 
    {
        // 1. Определяем, с каким графом работаем
        let graph_map = if is_finalized {
            &GLOBAL_DATA.finalized_graph
        } else {
            &GLOBAL_DATA.processed_graph
        };

        let graph = match graph_map.get("main") {
            Some(g) => g,
            None => {
                warn!("recalc_chain: Graph 'main' not found (finalized={})", is_finalized);
                return None;
            }
        };

        let mut result_hops: Vec<HopData> = Vec::with_capacity(chain.len() - 1);
        let mut total_weight_acc = 1.0;

        // 2. Идём по всем парам
        for window in chain.windows(2) {
            let from_token = &window[0];
            let to_token = &window[1];

            let mut hop_data = HopData {
                from_token: from_token.clone(),
                to_token: to_token.clone(),
                pools: Vec::new(),
                best_pool: None,
            };

            // Перебираем все DEX, ищем пул (pool_address) и соответствующий PoolEdge
            for dex in [DexType::Orca, DexType::Raydium, DexType::Meteora] {
                if let Some(pubkey) = GLOBAL_DATA.find_pool_address_by_symbols(dex, from_token, to_token) {
                    // Находим edge в графе
                    if let Some(edge_idx) = graph.edge_indices().find(|&e| graph[e].pool_address == pubkey)
                    {
                        let edge = &graph[edge_idx];
                        // Сохраняем подробные данные
                        let info = ExtendedPoolInfo {
                            pool_address: pubkey,
                            price: edge.price,
                            fee_rate: edge.fee_rate,
                            liquidity: edge.liquidity,
                            weight: edge.weight,
                            is_active: edge.is_active,
                            last_update_slot: edge.last_update_slot,
                            last_update_time: edge.last_update_time,
                        };
                        hop_data.pools.push(info);
                    }
                }
            }

            // Выбираем лучший пул (по weight, например)
            let mut best_opt: Option<ExtendedPoolInfo> = None;
            for pinfo in &hop_data.pools {
                if pinfo.is_active {
                    if let Some(ref best) = best_opt {
                        if pinfo.weight > best.weight {
                            best_opt = Some(pinfo.clone());
                        }
                    } else {
                        best_opt = Some(pinfo.clone());
                    }
                }
            }

            hop_data.best_pool = best_opt.clone();
            if let Some(ref best_edge) = best_opt {
                total_weight_acc *= best_edge.weight;
            } else {
                // Если нет активных пулов - значит цепочка невалидна
                warn!("recalc_chain: no active pool for hop {}->{}", from_token, to_token);
                return None;
            }

            result_hops.push(hop_data);
        }

        // 3. Дополнительно можно вычислить simulate_amount
        let simulated_amount = (INITIAL_BALANCE as f64 * total_weight_acc) as u64;

        // 4. Формируем результат
        let chain_res = ChainResult {
            last_update: unix_timestamp(),
            chain_tokens: chain.to_vec(),
            hops: result_hops,
            total_weight: total_weight_acc,
            simulated_amount,
        };

        Some(chain_res)
    }

    /// Обновляет (пересчитывает) все цепочки, в которых участвует `pool_address`.
    #[allow(unused_variables)]
    pub fn update_affected_chains(pool_address: Pubkey) {
        if let Some(chain_indices) = GLOBAL_DATA.chain_references.get(&pool_address) {
            let offset_4 = GLOBAL_DATA.chain_storage_4.len();

            for &idx in chain_indices.iter() {
                if idx < offset_4 {
                    // Цепочка длины 4
                    if let Some(chain) = GLOBAL_DATA.chain_storage_4.get(&idx) {
                        let tokens = chain.value();
                        // Пересчитываем processed
                        let r1 = Self::recalc_chain(tokens, false);
                        debug!("router: обновили цепочку длины 4 [{}] для {}", idx, false);
                        // Пересчитываем finalized
                        let r2 = Self::recalc_chain(tokens, true);
                        debug!("router: обновили цепочку длины 4 [{}] для {}", idx, true);

                        // TODO: Вызвать арбитражную симуляцию, если нужно
                        // Self::simulate_arbitrage_if_needed(&r1, &r2, ...);
                    }
                } else {
                    // Цепочка длины 5
                    let idx_in_5 = idx - offset_4;
                    if let Some(chain) = GLOBAL_DATA.chain_storage_5.get(&idx_in_5) {
                        let tokens = chain.value();
                        let r1 = Self::recalc_chain(tokens, false);
                        debug!("router: обновили цепочку длины 5 [{}] для {}", idx_in_5, false);
                        let r2 = Self::recalc_chain(tokens, true);
                        debug!("router: обновили цепочку длины 5 [{}] для {}", idx_in_5, true);

                        // TODO: Вызвать арбитражную симуляцию, если нужно
                        // Self::simulate_arbitrage_if_needed(&r1, &r2, ...);
                    }
                }
            }
        }
    }

    /// Пример заглушки для вызова арбитражной логики:
    /// (Пока пусто, “TODO”).
    #[allow(dead_code)]
    #[allow(unused_variables)]
    fn simulate_arbitrage_if_needed(
        processed_chain: &Option<ChainResult>,
        finalized_chain: &Option<ChainResult>,
    ) {
        // TODO: Реализовать симуляцию арбитража, используя данные best_pool
        //       и т. п.
        //       Могут понадобиться вызовы DEX-калькуляторов, расчет проскальзываний...
        //       Использование кеша для оптимизации и ускорения работы.
    }

    // Поиск лучшего маршрута для заданной цепочки.
    // fn find_best_route(chain: &[String]) -> Option<BestRoute> {
        // TODO: Реализовать поиск лучшего маршрута
        //       Использование кеша для оптимизации и ускорения работы.
    // }
}


/*

TODO: Реализовать полный Snapshot данных перед иницилизацией чтобы хранить все
состояния цепочек и пулов, имея общую картину. Затем после snapshot реализовать кеширование данных по пункту 3.

2. Какой объём данных у ChainResult
ChainResult хранит текущую картину по одной цепочке (списку токенов). То есть при вызове recalc_chain мы берём последние значения PoolEdge из графа и складываем в “hops” (HopData).
ChainResult не содержит “полной копии” всех пулов системы — только те, что участвуют в конкретных переходах (hop’ах).
Для будущего арбитража ChainResult может содержать всю нужную информацию (стоимость, комиссию, вес, ликвидность) для каждого хопа, чтобы дальше моделировать сделки.


TODO:

3. Идея о кешировании ChainResult
Почему вообще может понадобиться кэш:

Если цепочек очень много (или пересчёты дорогие), а пул не менялся, то нет смысла пересчитывать цепочку заново. Можно взять готовый ChainResult из кэша.
Аналогично, если мы хотим быстро ответить “каков итоговый вес” цепочки через минуту после предыдущего пересчёта, а за это время ничего не изменилось — можно вернуть старый результат.
Какая польза, если данные и так “актуальны” в PoolEdge?

В PoolEdge — только сырые метрики пула. А ChainResult — агрегированный расчёт, связанный именно с конкретной последовательностью хопов, включающей потенциально несколько пулов на шаге.
Если у нас нет кэша, придётся каждый раз (при любом запросе) проходиться по всем шагам цепочки, смотреть все DEX, находить лучший пул и т. д.
Если у нас есть кэш, мы можем просто проверить, “не изменилась ли версия” пулов в цепочке. Если нет, возвращаем готовый ChainResult.
Где хранить кэш:

Либо внутри router.rs (например, static ref ROUTER_CACHE: DashMap<Vec<String>, ChainResult>).
Либо в GlobalData (например, Arc<DashMap<Vec<String>, ChainResult>>).
В любом случае, кэш — это дополнительный механизм оптимизации, не отменяющий того, что PoolEdge хранит самые последние данные.

*/