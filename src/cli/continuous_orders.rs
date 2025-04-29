use crate::services::order_service::OrderService;
use crate::utils::dummy_orders::{
    find_quote_by_chains, list_available_chain_pairs, load_dummy_orders,
};
use crate::utils::file_utils;
use alloy::hex::ToHexExt;
use console::Term;
use console::style;
use dialoguer::{Confirm, Input, Select};
use eyre::Result;
use std::path::Path;
use std::process;
use std::time::Duration;
use tracing::info;

pub async fn run_continuous_orders() -> Result<()> {
    let term = Term::stdout();

    term.write_line(
        &style("🌼 Welcome to the Garden SDK Continuous Order CLI!")
            .green()
            .bold()
            .to_string(),
    )?;
    term.write_line(
        &style("🔄 This will create orders in a continuous loop")
            .yellow()
            .dim()
            .to_string(),
    )?;
    term.write_line("")?;

    let dummy_orders_path = Path::new("data/dummy_orders.json");
    let dummy_quotes = match load_dummy_orders(dummy_orders_path) {
        Ok(quotes) => quotes,
        Err(e) => {
            eprintln!(
                "{}",
                style(format!("❌ Failed to load dummy quotes: {}", e)).red()
            );
            process::exit(1);
        }
    };

    let chain_pairs = list_available_chain_pairs(&dummy_quotes);
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

    let initiator_source_address: String = Input::new()
        .with_prompt(
            &style("🏢 Enter initiator source address")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    let initiator_destination_address: String = Input::new()
        .with_prompt(
            &style("🏢 Enter initiator destination address")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    let private_key: String = Input::new()
        .with_prompt(
            &style("🔑 Enter your private key for signing transactions")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    let quote = find_quote_by_chains(&dummy_quotes, &selected_pair.0, &selected_pair.1)
        .expect("No quote found for selected chain pair");

    let iterations: u32 = Input::new()
        .with_prompt(
            &style("🔄 How many iterations to run? (0 for infinite)")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    let delay_seconds: u64 = Input::new()
        .with_prompt(
            &style("⏱️ Delay between iterations (seconds)")
                .cyan()
                .to_string(),
        )
        .default(5)
        .interact_text()?;

    let order_service = OrderService::new();

    println!(
        "{}",
        style("🔄 Starting continuous order process...")
            .yellow()
            .bold()
    );

    let mut iteration_count = 0;
    loop {
        if iterations > 0 && iteration_count >= iterations {
            println!(
                "{}",
                style(format!("✅ Completed {} iterations", iterations))
                    .green()
                    .bold()
            );
            break;
        }

        iteration_count += 1;
        println!(
            "{}",
            style(format!("🔄 Starting iteration {}", iteration_count))
                .cyan()
                .bold()
        );

        println!("{}", style("📦 Getting quote...").yellow());

        match order_service
            .get_quote(&quote.order_pair, &quote.amount, quote.exact_out)
            .await
        {
            Ok((strategy_id, input_price, output_price, destination_amount)) => {
                println!(
                    "{}",
                    style(format!(
                        "✅ Quote received with strategy ID: {}",
                        strategy_id
                    ))
                    .green()
                );
                println!(
                    "{}",
                    style(format!(
                        "💰 Input token price: {}, Output token price: {}",
                        input_price, output_price
                    ))
                    .green()
                );
                println!(
                    "{}",
                    style(format!("💰 Destination amount: {}", destination_amount)).green()
                );

                println!("{}", style("📦 Creating order...").yellow());

                match order_service
                    .create_order_with_custom_addresses(
                        strategy_id,
                        input_price,
                        output_price,
                        &quote.order_pair,
                        &quote.amount,
                        quote.exact_out,
                        destination_amount,
                        &initiator_source_address,
                        &initiator_destination_address,
                        private_key.clone(),
                    )
                    .await
                {
                    Ok((order_id, secret)) => {
                        println!(
                            "{}",
                            style(format!("✅ Order created with ID: {}", order_id))
                                .green()
                                .bold()
                        );
                        println!(
                            "{}",
                            style(format!("🔑 Order secret: {}", secret)).green().dim()
                        );
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            style(format!("❌ Failed to create order: {}", e)).red()
                        );
                    }
                }
            }
            Err(e) => {
                println!("{}", style(format!("❌ Failed to get quote: {}", e)).red());
            }
        }

        if iterations == 0 || iteration_count < iterations {
            println!(
                "{}",
                style(format!(
                    "⏱️ Waiting {} seconds before next iteration...",
                    delay_seconds
                ))
                .dim()
            );
            tokio::time::sleep(Duration::from_secs(delay_seconds)).await;
        }
    }

    println!(
        "{}",
        style("🙏 Thank you for using the Garden SDK Continuous Order CLI!").magenta()
    );

    Ok(())
}
