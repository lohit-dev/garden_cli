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
        &style("🔄 This will create, initiate, and redeem orders in a continuous loop")
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

    // Get delay between operation steps (create -> initiate -> redeem)
    let step_delay_seconds: u64 = Input::new()
        .with_prompt(
            &style("⏱️ Delay between operation steps (seconds)")
                .cyan()
                .to_string(),
        )
        .default(2)
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

        // STEP 1: Get quote
        println!("{}", style("📦 Getting quote...").yellow());

        let quote_result = order_service
            .get_quote(&quote.order_pair, &quote.amount, quote.exact_out)
            .await;

        match quote_result {
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

                // STEP 2: Create order
                println!("{}", style("📦 Creating order...").yellow());

                let create_result = order_service
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
                    .await;

                match create_result {
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

                        // Wait before initiating
                        println!(
                            "{}",
                            style(format!(
                                "⏱️ Waiting {} seconds before initiating order...",
                                step_delay_seconds
                            ))
                            .dim()
                        );
                        tokio::time::sleep(Duration::from_secs(step_delay_seconds)).await;

                        // STEP 3: Initiate order
                        println!("{}", style("🚀 Initiating order...").yellow());

                        match order_service.initiate_order(&order_id, &private_key).await {
                            Ok(tx_hash) => {
                                println!(
                                    "{}",
                                    style(format!(
                                        "✅ Order initiated with transaction hash: {}",
                                        tx_hash
                                    ))
                                    .green()
                                    .bold()
                                );

                                // Wait before checking if order is ready for redemption
                                println!(
                                    "{}",
                                    style("⏳ Waiting for order to be ready for redemption...")
                                        .yellow()
                                );

                                // Poll for order readiness with a timeout
                                let max_poll_attempts = 10;
                                let mut is_ready = false;

                                for attempt in 1..=max_poll_attempts {
                                    println!(
                                        "{}",
                                        style(format!("🔍 Checking if order is ready for redemption (attempt {}/{})", 
                                            attempt, max_poll_attempts))
                                        .dim()
                                    );

                                    match order_service
                                        .is_order_ready_for_redemption(&order_id)
                                        .await
                                    {
                                        Ok(ready) => {
                                            if ready {
                                                is_ready = true;
                                                println!(
                                                    "{}",
                                                    style("✅ Order is ready for redemption!")
                                                        .green()
                                                );
                                                break;
                                            }

                                            println!(
                                                "{}",
                                                style("⏳ Order not yet ready for redemption, waiting...")
                                                    .yellow()
                                            );
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                style(format!(
                                                    "⚠️ Error checking order readiness: {}",
                                                    e
                                                ))
                                                .yellow()
                                            );
                                        }
                                    }

                                    // Wait before next check
                                    tokio::time::sleep(Duration::from_secs(step_delay_seconds))
                                        .await;
                                }

                                if is_ready {
                                    // STEP 4: Redeem order
                                    println!("{}", style("💎 Redeeming order...").yellow());

                                    // Use retry_redeem_order for better reliability
                                    let max_redeem_attempts = 3;
                                    match order_service
                                        .retry_redeem_order(&order_id, &secret, max_redeem_attempts)
                                        .await
                                    {
                                        Ok(redeem_tx_hash) => {
                                            println!(
                                                "{}",
                                                style(format!(
                                                    "✅ Order redeemed with transaction hash: {}",
                                                    redeem_tx_hash
                                                ))
                                                .green()
                                                .bold()
                                            );
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                style(format!("❌ Failed to redeem order: {}", e))
                                                    .red()
                                            );
                                        }
                                    }
                                } else {
                                    println!(
                                        "{}",
                                        style("⚠️ Order not ready for redemption after maximum attempts, continuing to next iteration")
                                            .yellow()
                                    );
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    style(format!("❌ Failed to initiate order: {}", e)).red()
                                );
                            }
                        }
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
