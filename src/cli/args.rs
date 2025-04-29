use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    
    Create {
        
        #[clap(short, long, default_value = "1")]
        count: usize,

        
        #[clap(short, long, default_value = "order_data.json")]
        output: String,
    },

    
    Verify {
        
        #[clap(short, long, default_value = "order_data.json")]
        input: String,

        
        #[clap(short, long, default_value = "100")]
        concurrency: usize,
    },

    
    Initiate {
        
        #[clap(short, long, default_value = "order_data.json")]
        input: String,

        
        #[clap(short, long)]
        private_key: String,

        
        #[clap(short, long, default_value = "5")]
        concurrency: usize,
    },

    
    Redeem {
        
        #[clap(short, long, default_value = "order_data.json")]
        input: String,

        
        #[clap(short, long, default_value = "5")]
        concurrency: usize,
    },

    
    Status {
        
        #[clap(short, long)]
        order_id: String,
    },

    
    GardenFlow {
        
        #[clap(short, long, default_value = "1")]
        count: usize,
    },
}
