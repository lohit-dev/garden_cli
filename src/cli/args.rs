use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create orders with quotes and attestation
    Create {
        /// Number of orders to create
        #[clap(short, long, default_value = "1")]
        count: usize,
        
        /// Path to save order IDs and secrets
        #[clap(short, long, default_value = "order_data.json")]
        output: String,
    },
    
    /// Verify created orders
    Verify {
        /// Path to order data file
        #[clap(short, long, default_value = "order_data.json")]
        input: String,
        
        /// Maximum concurrent verification requests
        #[clap(short, long, default_value = "100")]
        concurrency: usize,
    },
    
    /// Initiate orders with EIP-712 signing
    Initiate {
        /// Path to order data file
        #[clap(short, long, default_value = "order_data.json")]
        input: String,
        
        /// Private key for signing (hex format)
        #[clap(short, long)]
        private_key: String,
        
        /// Maximum concurrent initiation requests
        #[clap(short, long, default_value = "5")]
        concurrency: usize,
    },
    
    /// Redeem orders
    Redeem {
        /// Path to order data file
        #[clap(short, long, default_value = "order_data.json")]
        input: String,
        
        /// Maximum concurrent redemption requests
        #[clap(short, long, default_value = "5")]
        concurrency: usize,
    },
    
    /// Check order status
    Status {
        /// Order ID to check
        #[clap(short, long)]
        order_id: String,
    },
    
    /// Execute complete Garden Finance flow
    GardenFlow {
        /// Number of orders to process
        #[clap(short, long, default_value = "1")]
        count: usize,
    },
}
