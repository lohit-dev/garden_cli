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
    // Initialize tracing
    let term = Term::stdout();

    // 🌱 Welcome message
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

    // 📄 Load dummy orders
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

    // 🔗 Chain pair selection
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

    // Get initiator source address
    let initiator_source_address: String = Input::new()
        .with_prompt(
            &style("🏢 Enter initiator source address")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    // Get initiator destination address
    let initiator_destination_address: String = Input::new()
        .with_prompt(
            &style("🏢 Enter initiator destination address")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    // 🔑 Get private key for signing transactions
    let private_key: String = Input::new()
        .with_prompt(
            &style("🔑 Enter your private key for signing transactions")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    // Find the quote for the selected chain pair
    let quote = find_quote_by_chains(&dummy_quotes, &selected_pair.0, &selected_pair.1)
        .expect("No quote found for selected chain pair");

    // Get the number of iterations to run (0 for infinite)
    let iterations: u32 = Input::new()
        .with_prompt(
            &style("🔄 How many iterations to run? (0 for infinite)")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    // Get delay between iterations
    let delay_seconds: u64 = Input::new()
        .with_prompt(
            &style("⏱️ Delay between iterations (seconds)")
                .cyan()
                .to_string(),
        )
        .default(5)
        .interact_text()?;

    // Initialize the order service
    let order_service = OrderService::new();

    // Start the continuous loop
    println!(
        "{}",
        style("🔄 Starting continuous order process...")
            .yellow()
            .bold()
    );

    let mut iteration_count = 0;
    loop {
        // Check if we've reached the desired number of iterations
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

        // STEP 1: Create Order
        println!("{}", style("📦 Creating order...").yellow());

        let order_pair = quote.order_pair.clone();
        let amount = quote.amount.clone();

        // Parse the order pair to extract chain information
        let parts: Vec<&str> = order_pair.split("::").collect();
        if parts.len() != 2 {
            println!(
                "{}",
                style(format!("❌ Invalid order pair format: {}", order_pair)).red()
            );
            continue;
        }

        let (src, dst) = (parts[0], parts[1]);
        let src_parts: Vec<&str> = src.split(':').collect();

        if src_parts.len() < 1 {
            println!(
                "{}",
                style(format!("❌ Invalid source chain format: {}", src)).red()
            );
            continue;
        }

        let source_chain = src_parts[0];

        // Create a custom implementation to override the default behavior in OrderService
        let order_service_clone = order_service.clone();

        let quote = match order_service_clone
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

                // Create order with the complete flow (create, initiate, redeem)
                println!("{}", style("📦 Creating and processing order...").yellow());

                match order_service_clone
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
                        private_key.to_string(),
                    )
                    .await
                {
                    Ok((order_id, secret)) => {
                        println!(
                            "{}",
                            style(format!("✅ Order processed with ID: {}", order_id))
                                .green()
                                .bold()
                        );
                        println!(
                            "{}",
                            style(format!("🔑 Order secret: {}", secret)).green().dim()
                        );

                        // Check if the order has been successfully initiated on the source chain
                        println!(
                            "{}",
                            style("🔍 Checking if order has been initiated on source chain...")
                                .yellow()
                        );

                        // Try up to 5 times with 3 seconds between attempts
                        let max_retries = 5;
                        let mut is_initiated = false;

                        // Create a new OrderService instance for checking initiation status
                        let status_service = OrderService::new();

                        for retry in 1..=max_retries {
                            println!(
                                "{}",
                                style(format!(
                                    "🔄 Checking source initiation (attempt {}/{})",
                                    retry, max_retries
                                ))
                                .cyan()
                            );

                            match status_service.is_source_initiated(&order_id).await {
                                Ok(true) => {
                                    println!(
                                        "{}",
                                        style("✅ Order successfully initiated on source chain!")
                                            .green()
                                            .bold()
                                    );
                                    is_initiated = true;
                                    break;
                                }
                                Ok(false) => {
                                    println!(
                                        "{}",
                                        style(
                                            "⏳ Order not yet initiated on source chain, waiting..."
                                        )
                                        .yellow()
                                    );
                                }
                                Err(e) => {
                                    println!(
                                        "{}",
                                        style(format!("❌ Error checking order initiation: {}", e))
                                            .red()
                                    );
                                }
                            }

                            if retry < max_retries {
                                println!(
                                    "{}",
                                    style("⏱️ Waiting 3 seconds before next check...").dim()
                                );
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            }
                        }

                        if !is_initiated {
                            println!(
                                "{}",
                                style("⚠️ Could not confirm source chain initiation after multiple attempts")
                                    .yellow()
                                    .bold()
                            );
                        }
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            style(format!("❌ Failed to process order: {}", e)).red()
                        );
                    }
                }
            }
            Err(e) => {
                println!("{}", style(format!("❌ Failed to get quote: {}", e)).red());
            }
        };

        // Wait before the next iteration
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
