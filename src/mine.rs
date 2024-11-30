use crate::{
    errors::{CliError, Error, Result},
    rpc::RpcClient,
    transaction::Transaction,
};
use cached::proc_macro::cached;
use futures::{
    future::{self, try_join_all},
    Future, SinkExt,
};
use ore::{instruction, state::Proof, utils::AccountDeserialize, PROOF};
use rayon::{prelude::*, Scope};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    keccak::{hashv, Hash as KeccakHash},
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};
use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    fmt::Debug,
    hash::{Hash, Hasher},
    io::{stdout, Write},
    num,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{channel, Receiver},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};

#[derive(PartialEq)]
pub struct Miner {
    keypair: Keypair,
}

impl Miner {
    pub fn new(keypair: Keypair) -> Self {
        Miner { keypair }
    }

    pub fn mine(&self, last_work: &Work, difficulty: &KeccakHash) -> Work {
        let mut next_hash: KeccakHash;

        let last_hash = last_work.hash().to_bytes();
        let pubkey = self.keypair.pubkey().to_bytes();
        let mut nonce = 0u64;
        loop {
            next_hash = hashv(&[&last_hash, &pubkey, &nonce.to_le_bytes()]);
            if next_hash.le(difficulty) {
                break;
            }
            nonce += 1;
        }

        Work::ToBeProved(next_hash, nonce)
    }

    pub fn mine_par(&self, last_work: &Work, difficulty: &KeccakHash) -> Work {
        let pubkey = self.pubkey().to_bytes();
        let last_hash = last_work.hash().to_bytes();

        let (hash, nonce) = (0..usize::MAX)
            .into_par_iter()
            .by_exponential_blocks()
            .find_map_any(|nonce| {
                let hash = hashv(&[&last_hash, &pubkey, &nonce.to_le_bytes()]);
                if hash.le(&difficulty) {
                    return Some((hash, nonce));
                } else {
                    return None;
                }
            })
            .unwrap();

        Work::ToBeProved(hash, nonce as u64)
    }

    pub async fn get_proof(&self, client: &RpcClient) -> Result<Proof> {
        let proof_address = proof_pubkey(self.keypair.pubkey());
        let data = client.get_account_data(&proof_address).await?;
        Ok(*Proof::try_from_bytes(&data).expect("Failed to parse miner's proof account"))
    }

    pub async fn get_proof_or_register(
        &self,
        client: &RpcClient,
        fee_payer: Option<&dyn Signer>,
    ) -> Result<Proof> {
        let proof_result = self.get_proof(client).await;

        if proof_result
            .as_ref()
            .is_err_and(|err| err.to_string().contains("AccountNotFound"))
        {
            let instruction = instruction::register(self.keypair.pubkey());
            let transaction = Transaction::new(vec![instruction]);
            let sent_tx = transaction
                .send(client, &self.keypair, fee_payer, false)
                .await?;
            let confirmed = sent_tx
                .confirm(
                    client,
                    CommitmentConfig::confirmed(),
                    Duration::from_millis(1000),
                )
                .await?;

            if confirmed {
                return self.get_proof(client).await;
            } else {
                return Err(Error::CliError(CliError::TransactionNotLanded));
            }
        }

        proof_result
    }
}

#[cached]
pub fn proof_pubkey(miner: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[PROOF, miner.as_ref()], &ore::ID).0
}

impl Eq for Miner {}

impl Hash for Miner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.keypair.pubkey().hash(state);
    }
}

impl Deref for Miner {
    type Target = Keypair;
    fn deref(&self) -> &Self::Target {
        &self.keypair
    }
}

impl Debug for Miner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Miner").field("keypair", &self.keypair.pubkey()).finish()
    }
}

#[derive(Clone, Debug)]
pub enum Work {
    Proved(KeccakHash),
    ToBeProved(KeccakHash, u64),
}

impl Work {
    pub fn hash(&self) -> &KeccakHash {
        match self {
            Work::Proved(hash) => hash,
            Work::ToBeProved(hash, _) => hash,
        }
    }

    pub fn to_signed(self, miner: &Miner) -> SignedWork {
        SignedWork {
            signer: miner,
            work: self,
        }
    }
}

#[derive(Debug)]
pub struct SignedWork<'a> {
    signer: &'a Miner,
    work: Work,
}

#[derive(Debug)]
pub struct MinerLog {
    last_work: Work,
}

#[derive(Debug)]
pub struct MineLine<'a> {
    miner_logs: HashMap<&'a Miner, MinerLog>,
}

impl<'a> MineLine<'a> {
    pub async fn init(
        miners: &'a [Miner],
        client: &RpcClient,
        fee_payer: Option<&dyn Signer>,
    ) -> Result<Self> {
        let set_last_work_futures = miners.iter().map(|miner| async move {
            let proof = miner.get_proof_or_register(client, fee_payer).await?;
            let log = MinerLog {
                last_work: Work::Proved(proof.hash.into()),
            };
            Result::Ok((miner, log))
        });

        let miner_logs = try_join_all(set_last_work_futures).await?;
        let miner_logs = HashMap::from_iter(miner_logs);

        return Ok(MineLine { miner_logs });
    }

    pub fn mine_in_scope(
        &'a mut self,
        scope: &Scope<'a>,
        difficulty: KeccakHash,
    ) -> Receiver<SignedWork<'a>> {
        let (sender, receiver) = channel::<SignedWork<'a>>();

        for (miner, miner_log) in self.miner_logs.iter_mut() {
            let sender = sender.clone();
            scope.spawn(move |_| loop {
                println!("Mining new...");
                let now = Instant::now();
                let last_work = miner_log.last_work.clone();
                let new_work = miner.mine_par(&last_work, &difficulty.clone());
                miner_log.last_work = new_work.clone();
                let signed_work = new_work.to_signed(miner);
                println!("Mined: {:?}", signed_work);
                println!("Duration: {:?}", now.elapsed());
                sender.send(signed_work).unwrap();
            });
        }

        receiver
    }
}
