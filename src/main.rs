#![allow(unused)]

mod cli;
mod config;
mod models;
mod services;
mod utils;

use crate::cli::run;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}
