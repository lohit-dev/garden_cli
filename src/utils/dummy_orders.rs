use crate::models::order::LoadOrder;
use crate::models::quote::QuoteRequest;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyOrders {
    pub orders: Vec<LoadOrder>,
}

pub fn load_dummy_orders_data(file_path: &Path) -> Result<DummyOrders> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let orders: DummyOrders = serde_json::from_reader(reader)?;
    Ok(orders)
}

pub fn find_order_by_chains(
    orders: &DummyOrders,
    source_chain: &str,
    destination_chain: &str,
) -> Option<LoadOrder> {
    orders
        .orders
        .iter()
        .find(|order| {
            order.source_chain == source_chain && order.destination_chain == destination_chain
        })
        .cloned()
}

pub fn list_available_chain_pairs_for_orders(orders: &DummyOrders) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for order in &orders.orders {
        let pair = (order.source_chain.clone(), order.destination_chain.clone());
        if !pairs.contains(&pair) {
            pairs.push(pair);
        }
    }
    pairs
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyQuotes {
    pub quotes: Vec<QuoteRequest>,
}

pub fn load_dummy_orders(file_path: &Path) -> Result<DummyQuotes> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let quotes: DummyQuotes = serde_json::from_reader(reader)?;
    Ok(quotes)
}

pub fn find_quote_by_chains(
    quotes: &DummyQuotes,
    source_chain: &str,
    destination_chain: &str,
) -> Option<QuoteRequest> {
    quotes
        .quotes
        .iter()
        .find(|quote| {
            let parts: Vec<&str> = quote.order_pair.split("::").collect();
            if parts.len() != 2 {
                return false;
            }

            let source_parts: Vec<&str> = parts[0].split(":").collect();
            let dest_parts: Vec<&str> = parts[1].split(":").collect();

            if source_parts.len() < 1 || dest_parts.len() < 1 {
                return false;
            }

            source_parts[0] == source_chain && dest_parts[0] == destination_chain
        })
        .cloned()
}

pub fn list_available_chain_pairs(quotes: &DummyQuotes) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for quote in &quotes.quotes {
        let parts: Vec<&str> = quote.order_pair.split("::").collect();
        if parts.len() == 2 {
            let source_parts: Vec<&str> = parts[0].split(":").collect();
            let dest_parts: Vec<&str> = parts[1].split(":").collect();

            if source_parts.len() >= 1 && dest_parts.len() >= 1 {
                let pair = (source_parts[0].to_string(), dest_parts[0].to_string());
                if !pairs.contains(&pair) {
                    pairs.push(pair);
                }
            }
        }
    }
    pairs
}
