# Garden CLI

A command-line interface for interacting with the Garden Finance protocol, featuring parallel processing of orders.

## Quick Start

1. Clone the repository:

```bash
git clone <repository-url>
cd garden_cli
```

2. Run the application:

```bash
cargo run -q
```

## Interactive Mode Example

When you run `cargo run -q`, you'll see an interactive prompt like this:

```
🌼 Welcome to the Garden SDK CLI Application!
👥 How many clients do you want to create? 5
📦 How many orders per client? 10
🔗 Select source chain -> destination chain
  arbitrum_sepolia -> starknet_sepolia
⚙️ Do you want to initiate the created orders? [Y/n]
🔑 Enter your private key (hex format)
🎉 Do you want to redeem the orders? [Y/n]
```

## Features

- Parallel order processing
- Automatic secret generation
- Chain pair selection
- Order data persistence
- Interactive CLI interface

## Requirements

- Rust 1.70 or higher
- Garden Finance API key (contact support to get one)
- Private key for signing transactions (hex format)
- Network connectivity to Garden Finance API endpoints

## Future Plans

- Strategy Selection
  - View available strategies
  - Choose specific strategy for orders

- Amount Validation
  - Validate input amounts against strategy limits

- Additional Features
  - Order history

- Remove HardCoded Values
  - Remove hardcoded API links
  - HardCoded dummy_orders.json

## Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request
