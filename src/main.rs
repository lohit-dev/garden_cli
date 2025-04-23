#![allow(unused)]

mod cli;
mod config;
mod models;
mod services;
mod utils;

use crate::cli::args::Args;
use crate::models::order::Order;
use crate::utils::dummy_orders::{list_available_chain_pairs, load_dummy_orders};
use clap::Parser;
use console::Term;
use console::style;
use dialoguer::{Confirm, Input, Select};
use eyre::Result;
use std::path::Path;
use std::process;

fn main() -> Result<()> {
    let term = Term::stdout();

    // 🌱 Welcome message
    term.write_line(
        &style("🌼 Welcome to the Garden SDK CLI Application!")
            .green()
            .bold()
            .to_string(),
    )?;
    term.write_line(
        &style("🌿 This is a garden of features, ready to bloom!")
            .yellow()
            .dim()
            .to_string(),
    )?;
    term.write_line("")?;

    // 🌟 Start confirmation
    if !Confirm::new()
        .with_prompt(
            &style("🚀 Do you want to start the order creation process?")
                .green()
                .bold()
                .to_string(),
        )
        .default(true)
        .interact()?
    {
        term.write_line(&style("👋 Exiting application...").red().to_string())?;
        return Ok(());
    }

    // Parse CLI arguments
    let args = Args::parse();

    // 🧑‍💼 Client & Order input
    let num_clients: u32 = Input::new()
        .with_prompt(
            &style("👥 How many clients do you want to create?")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    let orders_per_client: u32 = Input::new()
        .with_prompt(
            &style("🧾 How many orders should each client make?")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    // 📄 Load dummy orders
    let dummy_orders_path = Path::new("data/dummy_orders.json");
    let dummy_orders = match load_dummy_orders(dummy_orders_path) {
        Ok(orders) => orders,
        Err(e) => {
            eprintln!(
                "{}",
                style(format!("❌ Failed to load dummy orders: {}", e)).red()
            );
            process::exit(1);
        }
    };

    // 🔗 Chain pair selection
    let chain_pairs = list_available_chain_pairs(&dummy_orders);
    let chain_pair_options: Vec<String> = chain_pairs
        .iter()
        .map(|(src, dst)| format!("{} -> {}", src, dst))
        .collect();

    let selection = Select::new()
        .with_prompt(
            &style("🔗 Select source chain -> destination chain")
                .blue()
                .to_string(),
        )
        .items(&chain_pair_options)
        .default(0)
        .interact()?;

    let selected_pair = &chain_pairs[selection];
    println!(
        "{}",
        style(format!(
            "✅ Selected chain pair: {} -> {}",
            selected_pair.0, selected_pair.1
        ))
        .green()
    );

    // 🛠️ Order creation confirmation
    let total_orders = num_clients * orders_per_client;
    let prompt = format!(
        "\n🌸 You are about to create {} {} with {} {}.\nDo you want to proceed?",
        total_orders,
        if total_orders == 1 { "order" } else { "orders" },
        num_clients,
        if num_clients == 1 {
            "client"
        } else {
            "clients"
        },
    );

    let order_ids: Vec<String> = if Confirm::new()
        .with_prompt(&style(prompt).magenta().bold().to_string())
        .default(true)
        .interact()?
    {
        println!("{}", style("📦 Creating orders...").yellow());
        vec![] // Placeholder
    } else {
        println!("{}", style("🛑 Order creation skipped.").red());
        return Ok(());
    };

    // 🔧 Initiate Orders
    if Confirm::new()
        .with_prompt(
            &style("⚙️ Do you want to initiate the created orders?")
                .cyan()
                .to_string(),
        )
        .default(true)
        .interact()?
    {
        println!("{}", style("🔧 Initiating orders...").yellow());
    } else {
        println!("{}", style("⏭️ Skipping order initiation.").dim());
        return Ok(());
    }

    // 🎁 Redeem Orders
    if Confirm::new()
        .with_prompt(
            &style("🎉 Do you want to redeem the orders?")
                .cyan()
                .to_string(),
        )
        .default(true)
        .interact()?
    {
        println!("{}", style("💸 Redeeming orders...").yellow());

        // ✅ Final Success Message
        println!(
            "{}",
            style(format!(
                "✅ Congratulations! You have successfully created, initiated, and redeemed {} order(s)!",
                order_ids.len()
            ))
            .green()
            .bold()
        );
        println!(
            "{}",
            style("🌐 You can now view the order status in the dashboard.").blue()
        );
        println!(
            "{}",
            style("🙏 Thank you for using the Garden SDK CLI Application!").magenta()
        );
    } else {
        println!("{}", style("⏭️ Skipping order redemption.").dim());
    }

    Ok(())
}

// Function to fetch attested quote
fn fetch_attested_quote(source_chain: &str, destination_chain: &str) -> Result<()> {
    println!(
        "Fetching attested quote for {} -> {}",
        source_chain, destination_chain
    );
    // TODO: Implement quote fetching
    todo!()
}

// Function to create orders
fn create_orders(num_clients: u32, orders_per_client: u32) -> Result<Vec<String>> {
    println!(
        "Creating {} orders for {} clients",
        orders_per_client, num_clients
    );
    // TODO: Implement order creation
    todo!()
}

// Function to initiate orders
fn initiate_orders(order_ids: &[String]) -> Result<()> {
    println!("Initiating {} orders", order_ids.len());
    // TODO: Implement order initiation with custom signing
    todo!()
}

// Function to redeem orders
fn redeem_orders(order_ids: &[String]) -> Result<()> {
    println!("Redeeming {} orders", order_ids.len());
    // TODO: Implement order redemption
    todo!()
}
