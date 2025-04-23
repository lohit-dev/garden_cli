use clap::Parser;
use console::{Term, style};
use dialoguer::{Confirm, Input, Select};

pub mod cli;
pub mod config;
pub mod models;
pub mod services;
pub mod utils;

fn main() {
    let term = Term::stdout();

    // Welcome message
    term.write_line(
        &style("Welcome to the Order Creator CLI Application!")
            .green()
            .bold()
            .to_string(),
    )
    .unwrap();
    term.write_line(
        &style("This is a garden of features, ready to bloom!")
            .yellow()
            .dim()
            .to_string(),
    )
    .unwrap();
    term.write_line("Let's get started...\n").unwrap();

    if Confirm::new()
        .with_prompt(
            &style("Do you want to start the order creation process?")
                .green()
                .to_string(),
        )
        .default(true)
        .interact()
        .unwrap()
    {
        let _args = cli::args::Args::parse();

        let num_clients: u32 = Input::new()
            .with_prompt("How many clients do you want to create?")
            .interact_text()
            .unwrap();

        let orders_per_client: u32 = Input::new()
            .with_prompt("How many orders should each client make?")
            .interact_text()
            .unwrap();

        let available_orders = vec![
            "Order 1: Buy Apple",
            "Order 2: Buy Banana",
            "Order 3: Buy Carrot",
            "Order 4: Buy Date",
        ];

        let selection = Select::new()
            .with_prompt("Choose an order")
            .items(&available_orders)
            .default(0)
            .interact()
            .unwrap();

        println!("You selected: {}", available_orders[selection]);

        println!(
            "Creating {} clients, each with {} orders.",
            num_clients, orders_per_client
        );

        // Load configuration (uncomment this when implemented)
        // let config = load_config();

        // Run the interactive mode (uncomment when implemented)
        // if let Err(e) = run_interactive_mode(&args, &config) {
        //     eprintln!("Error: {}", e);
        //     std::process::exit(1);
        // }
    } else {
        term.write_line("Exiting application...").unwrap();
    }
}
