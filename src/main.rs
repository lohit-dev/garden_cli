#![allow(unused)]

mod cli;
mod config;
mod models;
mod services;
mod utils;

use crate::cli::run;
use crate::utils::logging;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize JSON logging
    logging::init_json_logging();

    // Run the application
    run().await
}
