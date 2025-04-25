pub mod args;
pub mod interactive;

use crate::services::order_service::OrderService;
use crate::utils::dummy_orders::{
    find_quote_by_chains, list_available_chain_pairs, load_dummy_orders,
};
use alloy::hex::ToHexExt;
use console::Term;
use console::style;
use dialoguer::{Confirm, Input, Select};
use eyre::Result;
use futures::{StreamExt, stream::FuturesUnordered};
use std::path::Path;
use std::process;
use std::sync::Arc;
use tracing::info;

pub async fn run() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
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

    // 🧑‍💼 Get number of clients (coroutines)
    let num_clients: u32 = Input::new()
        .with_prompt(
            &style("👥 How many clients do you want to create?")
                .cyan()
                .to_string(),
        )
        .interact_text()?;

    // 📦 Get number of orders per client
    let orders_per_client: u32 = Input::new()
        .with_prompt(&style("📦 How many orders per client?").cyan().to_string())
        .interact_text()?;

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

    // Find the quote for the selected chain pair
    let quote = find_quote_by_chains(&dummy_quotes, &selected_pair.0, &selected_pair.1)
        .expect("No quote found for selected chain pair");

    // 🛠️ Order creation confirmation
    let prompt = format!(
        "\n🌸 You are about to create {} orders ({} clients × {} orders per client).\nDo you want to proceed?",
        num_clients * orders_per_client,
        num_clients,
        orders_per_client
    );

    if !Confirm::new()
        .with_prompt(&style(prompt).magenta().bold().to_string())
        .default(true)
        .interact()?
    {
        println!("{}", style("🛑 Order creation skipped.").red());
        return Ok(());
    }

    // Initialize the order service
    let order_service = OrderService::new();
    let mut order_ids: Vec<(String, String)> = Vec::new(); // (order_id, secret)

    println!("{}", style("📦 Creating orders...").yellow());

    // Create a semaphore to limit concurrent requests
    let semaphore = Arc::new(tokio::sync::Semaphore::new(num_clients as usize));

    println!(
        "{}",
        style(format!(
            "🔍 Fetching quote for order pair: {}",
            quote.order_pair
        ))
        .blue()
    );

    // Get quote for the order
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

            // Create orders based on the quote
            let mut tasks = FuturesUnordered::new();
            let strategy_id = strategy_id.clone(); // Clone before the loop

            // Clone order_pair, amount, and destination_amount outside the loop
            let order_pair = quote.order_pair.clone();
            let amount = quote.amount.clone();
            let destination_amount = destination_amount.clone();

            // Process each client (coroutine)
            for client_id in 0..num_clients {
                let order_service_clone = order_service.clone();
                let semaphore_clone = semaphore.clone();
                let strategy_id = strategy_id.clone(); // Clone for each client
                let order_pair = order_pair.clone();
                let amount = amount.clone();
                let destination_amount = destination_amount.clone(); // Clone for each client

                tasks.push(tokio::spawn(async move {
                    let mut results = Vec::new();
                    let client_start = std::time::Instant::now();

                    info!(
                        "Client {} starting to process {} orders",
                        client_id + 1,
                        orders_per_client
                    );

                    // Process orders for this client
                    for order_num in 0..orders_per_client {
                        let permit = semaphore_clone.clone().acquire_owned().await.unwrap();
                        match order_service_clone
                            .create_order(
                                strategy_id.clone(),
                                input_price,
                                output_price,
                                &order_pair,
                                &amount,
                                quote.exact_out,
                                destination_amount.clone(),
                            )
                            .await
                        {
                            Ok((order_id, secret)) => {
                                println!(
                                    "{}",
                                    style(format!(
                                        "✅ Client {}: Created order {} of {} (ID: {})",
                                        client_id + 1,
                                        order_num + 1,
                                        orders_per_client,
                                        order_id
                                    ))
                                    .green()
                                );
                                results.push(Ok((order_id, secret)));
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    style(format!(
                                        "❌ Client {}: Failed to create order {} of {}: {}",
                                        client_id + 1,
                                        order_num + 1,
                                        orders_per_client,
                                        e
                                    ))
                                    .red()
                                );
                                results.push(Err(e));
                            }
                        }
                        drop(permit);
                    }

                    info!(
                        "Client {} completed {} orders in {:?}",
                        client_id + 1,
                        orders_per_client,
                        client_start.elapsed()
                    );

                    results
                }));
            }

            // Collect results from all clients
            while let Some(result) = tasks.next().await {
                match result {
                    Ok(client_results) => {
                        for result in client_results {
                            match result {
                                Ok((order_id, secret)) => {
                                    order_ids.push((order_id, secret));
                                }
                                Err(_) => continue,
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", style(format!("❌ Client task error: {}", e)).red());
                    }
                }
            }
        }
        Err(e) => {
            println!("{}", style(format!("❌ Failed to get quote: {}", e)).red());
            return Ok(());
        }
    }

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

        // Get private key for signing
        let private_key: String = Input::new()
            .with_prompt(
                &style("🔑 Enter your private key (hex format)")
                    .cyan()
                    .to_string(),
            )
            .interact_text()?;

        // Initialize the order service
        let order_service = OrderService::new();

        // Create a semaphore to limit concurrent requests
        let semaphore = Arc::new(tokio::sync::Semaphore::new(num_clients as usize));

        let mut tasks = FuturesUnordered::new();

        // Process each client's orders
        for (order_id, _) in &order_ids {
            let order_service_clone = order_service.clone();
            let order_id_clone = order_id.clone();
            let private_key_clone = private_key.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            tasks.push(tokio::spawn(async move {
                let result = order_service_clone
                    .initiate_order(&order_id_clone, &private_key_clone)
                    .await;
                drop(permit);
                (order_id_clone, result)
            }));
        }

        while let Some(result) = tasks.next().await {
            match result {
                Ok((order_id, Ok(tx_hash))) => {
                    println!(
                        "{}",
                        style(format!("✅ Initiated order {}: {}", order_id, tx_hash)).green()
                    );
                }
                Ok((order_id, Err(e))) => {
                    println!(
                        "{}",
                        style(format!("❌ Failed to initiate order {}: {}", order_id, e)).red()
                    );
                }
                Err(e) => {
                    println!("{}", style(format!("❌ Task error: {}", e)).red());
                }
            }
        }
    } else {
        println!("{}", style("⏭️ Skipping order initiation.").dim());
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

        // Initialize the order service
        let order_service = OrderService::new();

        // Create a semaphore to limit concurrent requests
        let semaphore = Arc::new(tokio::sync::Semaphore::new(num_clients as usize));

        let mut tasks = FuturesUnordered::new();

        for (order_id, secret) in &order_ids {
            let order_service_clone = order_service.clone();
            let order_id_clone = order_id.clone();
            let secret_clone = secret.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            tasks.push(tokio::spawn(async move {
                // Use retry_redeem_order with 5 retry attempts instead of direct redeem_order
                let result = order_service_clone
                    .retry_redeem_order(&order_id_clone, &secret_clone, 5)
                    .await;
                drop(permit);
                (order_id_clone, result)
            }));
        }

        while let Some(result) = tasks.next().await {
            match result {
                Ok((order_id, Ok(tx_hash))) => {
                    println!(
                        "{}",
                        style(format!("✅ Redeemed order {}: {}", order_id, tx_hash)).green()
                    );
                }
                Ok((order_id, Err(e))) => {
                    println!(
                        "{}",
                        style(format!("❌ Failed to redeem order {}: {}", order_id, e)).red()
                    );
                }
                Err(e) => {
                    println!("{}", style(format!("❌ Task error: {}", e)).red());
                }
            }
        }

        // ✅ Final Success Message
        println!(
            "{}",
            style(format!(
                "✅ Congratulations! You have successfully created, initiated, and redeemed {} orders ({} clients × {} orders per client)!",
                order_ids.len(),
                num_clients,
                orders_per_client
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
