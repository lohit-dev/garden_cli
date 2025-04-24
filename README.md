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
# Interactive mode
cargo run -q

# Non-interactive mode
cargo run -- create --count 5 --output orders.json
cargo run -- initiate --input orders.json --private-key <your-key>
cargo run -- redeem --input orders.json
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

## Commands

### Create Orders
```bash
cargo run -- create --count <number> --output <file>
```
- `--count`: Number of orders to create
- `--output`: Path to save order data (default: order_data.json)

### Initiate Orders
```bash
cargo run -- initiate --input <file> --private-key <key> --concurrency <number>
```
- `--input`: Path to order data file
- `--private-key`: Private key for signing (hex format)
- `--concurrency`: Maximum concurrent requests (default: 5)

### Redeem Orders
```bash
cargo run -- redeem --input <file> --concurrency <number>
```
- `--input`: Path to order data file
- `--concurrency`: Maximum concurrent requests (default: 5)

### Check Order Status
```bash
cargo run -- status --order-id <id>
```
- `--order-id`: Order ID to check

## Features

- Parallel order processing
- Automatic secret generation
- Configurable concurrency
- Chain pair selection
- Order data persistence

## Requirements

- Rust 1.70 or higher
- Garden Finance API key (contact support to get one)
- Private key for signing transactions (hex format)
- Network connectivity to Garden Finance API endpoints

## Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request 