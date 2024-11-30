use std::ops::Deref;

use cached::{cached_result, proc_macro::cached, TimedCache};
use solana_client::{client_error::Result};
use solana_sdk::{commitment_config::CommitmentConfig, hash::Hash};

pub use solana_client::nonblocking::rpc_client::RpcClient;

pub struct RpcPool {
    clients: Vec<RpcClient>,
}

impl RpcPool {
    pub fn new(urls: Vec<String>) -> Self {
        let clients = urls.iter().map(|url| RpcClient::new(url.clone())).collect();
        RpcPool { clients }
    }
    pub fn get_client(&self, key: Option<usize>) -> &RpcClient {
        &self.clients[0]
    }
}

