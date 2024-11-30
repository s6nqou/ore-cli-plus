
mod mine;
mod factory;
mod pipeline;
mod rpc;
mod transaction;
mod errors;
mod mine_gpu;

use std::sync::Arc;

use clap::{command, Parser, Subcommand};
use mine::Miner;
use solana_sdk::signature::{read_keypair_file, Keypair};

use crate::{factory::Ore, rpc::RpcPool};

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        help = "Network address of your RPC provider",
    )]
    rpc: String,

    #[arg(
        long,
        help = "Filepath to keypair to use"
    )]
    owner: String,

    #[arg(
        long,
        help = "Filepath to keypair to use"
    )]
    miners: Vec<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Mine Ore using local compute")]
    Mine,

}
#[tokio::main]
async fn main() {
    let args = Args::parse();

    let owner = read_keypair_file(args.owner.clone()).unwrap();
    let rpc_pool = RpcPool::new(vec![args.rpc.clone()]);
    let miners = args.miners.iter().map(|miner| Miner::new(read_keypair_file(miner.clone()).unwrap())).collect();

    let ore = Ore { owner, rpc_pool, miners };

    ore.mine().await.unwrap();

    println!("{:?}", args);
}
