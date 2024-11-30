use crate::rpc::RpcClient;
use solana_client::{
    client_error::{ClientError, Result},
    rpc_config::RpcSendTransactionConfig,
};
use solana_sdk::{
    clock::{Slot, MAX_HASH_AGE_IN_SECONDS, MAX_PROCESSING_AGE},
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    signature::Signature,
    signer::Signer,
    transaction::Transaction as RawTransaction,
};
use solana_transaction_status::{TransactionConfirmationStatus, TransactionStatus};
use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};
use tokio::time::sleep;

pub struct Transactor {}

#[derive(Clone)]
pub struct Transaction {
    pub cu_limit: Option<u32>,
    pub cu_price: Option<u64>,
    pub instructions: Vec<Instruction>,
}

impl Transaction {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Transaction {
            cu_limit: None,
            cu_price: None,
            instructions,
        }
    }

    pub fn set_cu_limit(&mut self, units: u32) {
        self.cu_limit = Some(units);
    }

    pub fn set_cu_price(&mut self, micro_lamports: u64) {
        self.cu_price = Some(micro_lamports);
    }

    pub async fn send(
        &self,
        client: &RpcClient,
        signer: &dyn Signer,
        fee_payer: Option<&dyn Signer>,
        skip_preflight: bool,
    ) -> Result<SentTransaction> {
        let (blockhash, slot) = client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await?;

        let instructions = self.get_combined_instructions();

        let mut signing_keypairs = vec![signer];
        if let Some(fee_payer) = fee_payer {
            signing_keypairs.push(fee_payer)
        }

        let tx = RawTransaction::new_signed_with_payer(
            &instructions,
            fee_payer.map(|fee_payer| fee_payer.pubkey()).as_ref(),
            &signing_keypairs,
            blockhash,
        );

        let config = RpcSendTransactionConfig {
            skip_preflight,
            min_context_slot: Some(slot),
            ..Default::default()
        };

        let signature = client.send_transaction_with_config(&tx, config).await?;

        Ok(SentTransaction {
            transaction: self.clone(),
            blockhash,
            slot,
            signature,
        })
    }

    fn get_combined_instructions(&self) -> Vec<Instruction> {
        let mut instructions = vec![];

        if let Some(cu_limit) = self.cu_limit {
            instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(cu_limit));
        }
        if let Some(cu_price) = self.cu_price {
            instructions.push(ComputeBudgetInstruction::set_compute_unit_price(cu_price));
        }
        instructions.extend_from_slice(&self.instructions);

        instructions
    }
}

pub struct SentTransaction {
    transaction: Transaction,
    blockhash: Hash,
    slot: Slot,
    signature: Signature,
}

impl std::hash::Hash for SentTransaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.signature.hash(state);
    }
}

impl SentTransaction {
    pub async fn confirm(
        &self,
        client: &RpcClient,
        commitment: CommitmentConfig,
        interval: Duration,
    ) -> Result<bool> {
        loop {
            if let Some(status) = &client
                .get_signature_statuses(&[self.signature])
                .await?
                .value[0]
            {
                if status.satisfies_commitment(commitment) {
                    return Ok(true);
                }
            } else {
                let current_slot = client.get_slot().await?;
                if self.is_expired(current_slot) {
                    return Ok(false);
                }
            }

            sleep(interval).await;
        }
    }

    pub fn is_expired(&self, current_slot: Slot) -> bool {
        return current_slot > self.slot + MAX_PROCESSING_AGE as u64 + 1;
    }
}

impl Deref for SentTransaction {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

pub struct SentTransactions(HashSet<SentTransaction>);

impl SentTransactions {
    pub async fn confirm_any(
        &self,
        client: &RpcClient,
        commitment: CommitmentConfig,
        interval: Duration,
    ) -> Result<Vec<&SentTransaction>> {
        loop {
            let statuses = self.get_statuses(client).await?;

            let confirmed = statuses
                .iter()
                .filter_map(|(tx, status)| {
                    status
                        .as_ref()
                        .and_then(|status| status.satisfies_commitment(commitment).then_some(*tx))
                })
                .collect::<Vec<&SentTransaction>>();

            if !confirmed.is_empty() {
                return Ok(confirmed);
            }

            sleep(interval).await;
        }
    }

    async fn get_statuses(
        &self,
        client: &RpcClient,
    ) -> Result<Vec<(&SentTransaction, Option<TransactionStatus>)>> {
        let signatures = self
            .iter()
            .map(|tx| tx.signature)
            .collect::<Vec<Signature>>();
        let statuses = client.get_signature_statuses(&signatures).await?.value;
        Ok(self.iter().zip(statuses).collect())
    }
}

impl Deref for SentTransactions {
    type Target = HashSet<SentTransaction>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SentTransactions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
