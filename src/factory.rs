use std::{
    collections::{hash_map::RandomState, HashMap},
    sync::{mpsc, Arc, Mutex}, time::Instant,
};

use crate::{
    errors::Result,
    mine::{MineLine, Miner, Work},
    rpc::{RpcClient, RpcPool},
};
use ore::{state::Treasury, utils::AccountDeserialize, TREASURY_ADDRESS};
use solana_sdk::{keccak::Hash, pubkey::Pubkey, signature::Keypair};
use threadpool::ThreadPool;

pub struct Ore {
    pub owner: Keypair,
    pub rpc_pool: RpcPool,
    pub miners: Vec<Miner>,
}

impl Ore {
    pub fn fee_payer(&self) -> &Keypair {
        &self.owner
    }

    pub fn get_client(&self, key: Option<usize>) -> &RpcClient {
        self.rpc_pool.get_client(key)
    }

    pub async fn get_treasury(&self) -> Result<Treasury> {
        let data = self
            .get_client(None)
            .get_account_data(&TREASURY_ADDRESS)
            .await?;
        Ok(*Treasury::try_from_bytes(&data).expect("Failed to parse treasury account"))
    }

    pub async fn mine(self) -> Result<()> {
        // rayon::ThreadPoolBuilder::new().num_threads(8).build_global().unwrap();
        let treasury = self.get_treasury().await?;
        println!("Treasury: {:?}", treasury);

        let difficulty = Hash::new_from_array([
            0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ]);

        let mut mineline = MineLine::init(&self.miners, self.get_client(None), Some(self.fee_payer()))
            .await
            .unwrap();
        println!("Miners inited: {:?}", mineline);

        rayon::scope(|scope| {
            let receiver = mineline.mine_in_scope(scope, difficulty.into());
            loop {
                let work = receiver.recv().unwrap();
            }
        });

        Ok(())
    }
}
