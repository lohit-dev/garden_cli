#![allow(unused)]

mod cli;
mod config;
mod models;
mod services;
mod utils;

use crate::cli::run;
use crate::models::order::Order;
use crate::services::order_service::OrderService;
use crate::utils::dummy_orders::{
    find_quote_by_chains, list_available_chain_pairs, load_dummy_orders,
};
use alloy::hex::ToHexExt;
use alloy::primitives::{Bytes, FixedBytes};
use bigdecimal::BigDecimal;
use chrono::{TimeDelta, Utc};
use console::Term;
use console::style;
use dialoguer::{Confirm, Input, Select};
use eyre::Result;
use futures::{StreamExt, stream::FuturesUnordered};
use std::path::Path;
use std::sync::Arc;
use std::{process, string};
use tokio::sync::Mutex;
use tracing::{info, warn};
use tracing_subscriber::fmt::format;

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
