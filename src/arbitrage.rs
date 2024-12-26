// src/arbitrage.rs
use crate::data::GLOBAL_DATA;
use crate::graph::PoolEdge;
use crate::config::START_END_TOKEN_FOR_CHAINS;
use log::{info, debug};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

const SIMULATION_AMOUNT: u64 = 1_000_000_000; // 1 SOL

pub struct ArbitrageOpportunity {
    pub chain: Vec<String>,
    pub total_return: f64,
    pub expected_profit: f64,
    pub pools: Vec<(Pubkey, f64)>, // (pool_address, amount_in)
}

pub fn monitor_arbitrage_opportunities() {
    // Получаем все цепочки длины 4 и 5
    let chains_4 = GLOBAL_DATA.chains_4.iter().collect::<Vec<_>>();
    let chains_5 = GLOBAL_DATA.chains_5.iter().collect::<Vec<_>>();

    for chain in chains_4.iter().chain(chains_5.iter()) {
        if let Some(opportunity) = simulate_chain(chain, SIMULATION_AMOUNT) {
            if opportunity.expected_profit > 0.0 {
                info!("Found arbitrage opportunity:");
                info!("Chain: {:?}", opportunity.chain);
                info!("Expected return: {} SOL", opportunity.total_return / 1e9);
                info!("Expected profit: {} SOL", opportunity.expected_profit / 1e9);
            }
        }
    }
}

fn simulate_chain(chain: &[String], amount_in: u64) -> Option<ArbitrageOpportunity> {
    if let Some(graph) = GLOBAL_DATA.processed_graph.get("main") {
        let mut current_amount = amount_in as f64;
        let mut pools = Vec::new();

        for (i, window) in chain.windows(2).enumerate() {
            let token_in = &window[0];
            let token_out = &window[1];
            
            let is_first_swap = i == 0;
            let is_last_swap = i == chain.len() - 2;

            let best_edge = find_best_edge(
                &graph, 
                token_in, 
                token_out,
                is_first_swap,
                is_last_swap
            )?;
            
            let amount_out = simulate_swap(
                current_amount,
                best_edge.price,
                best_edge.fee_rate,
                best_edge.liquidity
            );

            pools.push((best_edge.pool_address, current_amount));
            current_amount = amount_out;
        }

        let profit = current_amount - amount_in as f64;
        
        Some(ArbitrageOpportunity {
            chain: chain.to_vec(),
            total_return: current_amount,
            expected_profit: profit,
            pools,
        })
    } else {
        None
    }
}

fn find_best_edge<'a>(
    graph: &'a Graph<String, PoolEdge>,
    token_in: &str,
    token_out: &str,
    is_first_swap: bool,
    is_last_swap: bool
) -> Option<&'a PoolEdge> {
    // Находим индексы вершин
    let start_idx = graph.node_indices()
        .find(|&i| graph[i] == token_in)?;
    let end_idx = graph.node_indices()
        .find(|&i| graph[i] == token_out)?;

    let edges: Vec<_> = graph.edges_connecting(start_idx, end_idx).collect();
    
    if edges.is_empty() {
        return None;
    }

    // Для первого свопа и промежуточных - ищем максимальную цену
    if is_first_swap || (!is_first_swap && !is_last_swap) {
        edges.into_iter()
            .max_by(|e1, e2| e1.weight().price.partial_cmp(&e2.weight().price).unwrap())
            .map(|e| e.weight())
    } 
    // Для последнего свопа - ищем минимальную цену
    else {
        edges.into_iter()
            .min_by(|e1, e2| e1.weight().price.partial_cmp(&e2.weight().price).unwrap())
            .map(|e| e.weight())
    }
}

fn simulate_swap(
    amount_in: f64,
    price: f64,
    fee_rate: f64,
    liquidity: f64
) -> f64 {
    let fee_amount = amount_in * fee_rate;
    let net_amount = amount_in - fee_amount;
    
    // Учитываем проскальзывание на основе размера свопа и ликвидности
    let slippage = (net_amount / liquidity).min(0.02); // max 2% slippage
    let effective_price = price * (1.0 - slippage);
    
    net_amount * effective_price
}