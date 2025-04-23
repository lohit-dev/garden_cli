use crate::models::order::Order;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyOrders {
    pub orders: Vec<Order>,
}

pub fn load_dummy_orders(file_path: &Path) -> Result<DummyOrders> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let orders: DummyOrders = serde_json::from_reader(reader)?;
    Ok(orders)
}

pub fn find_order_by_chains(
    orders: &DummyOrders,
    source_chain: &str,
    destination_chain: &str,
) -> Option<Order> {
    orders
        .orders
        .iter()
        .find(|order| {
            order.source_chain == source_chain && order.destination_chain == destination_chain
        })
        .cloned()
}

pub fn list_available_chain_pairs(orders: &DummyOrders) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for order in &orders.orders {
        let pair = (order.source_chain.clone(), order.destination_chain.clone());
        if !pairs.contains(&pair) {
            pairs.push(pair);
        }
    }
    pairs
}
