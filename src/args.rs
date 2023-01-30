use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about=None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[clap(short, long, help = "Id of the account|tx to be parsed")]
    pub id: String,

    #[clap(short, long, help = "Cluster (mainnet|devnet|testnet)")]
    pub cluster: String,

    pub output: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    Account {
        #[arg(short, long, help = "Number of recent transactions to retrieve")]
        tx_limit: u64,
    },
    Transaction,
}

