use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Args {
    /// Number of clients to spawn
    #[clap(short, long, default_value = "1")]
    pub client_count: usize,

    /// Default number of orders per client
    #[clap(short, long, default_value = "1")]
    pub orders_per_client: usize,
}
