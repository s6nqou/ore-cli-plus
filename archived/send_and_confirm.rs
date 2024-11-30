use std::time::Duration;

use solana_client::{
    client_error::{ClientError, ClientErrorKind, Result as ClientResult},
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig},
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    native_token::LAMPORTS_PER_SOL,
    signature::{Signature, Signer},
    transaction::Transaction,
};
use solana_transaction_status::{TransactionConfirmationStatus, UiTransactionEncoding};

use crate::{instruction::OreInstruction, Miner};

impl Miner {
    pub async fn send_and_confirm(
        &self,
        instruction: OreInstruction,
        dynamic_cus: bool,
        dynamic_fee: bool,
        skip_confirm: bool,
    ) -> ClientResult<Signature> {
        let signer = self.signer();
        let fee_payer = self.fee_payer();
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::confirmed());
        let _submit_client = self.submit_cluster.clone().map(|submit_cluster| {
            RpcClient::new_with_commitment(submit_cluster.clone(), CommitmentConfig::confirmed())
        });
        let submit_client = _submit_client.as_ref().unwrap_or(&client);

        // Return error if balance is zero
        let balance = client
            .get_balance_with_commitment(&fee_payer.pubkey(), CommitmentConfig::confirmed())
            .await
            .unwrap();
        if balance.value <= LAMPORTS_PER_SOL / 1000 {
            return Err(ClientError {
                request: None,
                kind: ClientErrorKind::Custom("Insufficient SOL balance".into()),
            });
        }

        // Build tx
        let mut instruction = instruction;
        let (mut hash, mut slot) = client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await
            .unwrap();
        let mut send_cfg = RpcSendTransactionConfig {
            skip_preflight: self.skip_preflight,
            preflight_commitment: Some(CommitmentLevel::Confirmed),
            encoding: Some(UiTransactionEncoding::Base64),
            max_retries: Some(self.rpc_retries),
            min_context_slot: Some(slot),
        };
        let mut tx = Transaction::new_with_payer(&instruction.to_vec(), Some(&fee_payer.pubkey()));

        // Simulate if necessary
        if dynamic_cus {
            let mut sim_attempts = 0;
            'simulate: loop {
                let sim_res = client
                    .simulate_transaction_with_config(
                        &tx,
                        RpcSimulateTransactionConfig {
                            sig_verify: false,
                            replace_recent_blockhash: true,
                            commitment: Some(CommitmentConfig::confirmed()),
                            encoding: Some(UiTransactionEncoding::Base64),
                            accounts: None,
                            min_context_slot: None,
                            inner_instructions: false,
                        },
                    )
                    .await;
                match sim_res {
                    Ok(sim_res) => {
                        if let Some(err) = sim_res.value.err {
                            println!("Simulaton error: {:?}", err);
                            sim_attempts += 1;
                            if sim_attempts.gt(&self.simulation_retries) {
                                return Err(ClientError {
                                    request: None,
                                    kind: ClientErrorKind::Custom("Simulation failed".into()),
                                });
                            }
                        } else if let Some(units_consumed) = sim_res.value.units_consumed {
                            println!("Dynamic CUs: {:?}", units_consumed);
                            instruction.compute_unit_limit = Some(units_consumed as u32 + 1000);
                            tx = Transaction::new_with_payer(
                                &instruction.to_vec(),
                                Some(&fee_payer.pubkey()),
                            );
                            break 'simulate;
                        }
                    }
                    Err(err) => {
                        println!("Simulaton error: {:?}", err);
                        sim_attempts += 1;
                        if sim_attempts.gt(&self.simulation_retries) {
                            return Err(ClientError {
                                request: None,
                                kind: ClientErrorKind::Custom("Simulation failed".into()),
                            });
                        }
                    }
                }
            }
        }

        if dynamic_fee {
            if let Some(dynamic_fee) = &self.dynamic_priority_fee {
                instruction.compute_unit_price = Some(dynamic_fee.get(&client).await?);
                tx = Transaction::new_with_payer(&instruction.to_vec(), Some(&fee_payer.pubkey()));
            }
        }

        // Submit tx
        tx.sign(&[&signer, &fee_payer], hash);
        let mut sigs = vec![];
        let mut attempts = 0;
        loop {
            println!(
                "Attempt {:?} with priority fee {:?}",
                attempts,
                instruction.compute_unit_price.unwrap_or(0)
            );
            match submit_client
                .send_transaction_with_config(&tx, send_cfg)
                .await
            {
                Ok(sig) => {
                    sigs.push(sig);
                    // println!("{:?}", sig);

                    // Confirm tx
                    if skip_confirm {
                        return Ok(sig);
                    }
                    for _ in 0..self.confirm_retries {
                        std::thread::sleep(Duration::from_millis(self.confirm_wait_ms));
                        match client.get_signature_statuses(&sigs).await {
                            Ok(signature_statuses) => {
                                for (sig_idx, signature_status) in
                                    signature_statuses.value.iter().enumerate()
                                {
                                    if let Some(signature_status) = signature_status.as_ref() {
                                        if signature_status.confirmation_status.is_some() {
                                            let current_commitment = signature_status
                                                .confirmation_status
                                                .as_ref()
                                                .unwrap();
                                            match current_commitment {
                                                TransactionConfirmationStatus::Processed => {}
                                                TransactionConfirmationStatus::Confirmed
                                                | TransactionConfirmationStatus::Finalized => {
                                                    println!("✅ Transaction landed on retries {} after total retries {}!", sig_idx, sigs.len() - 1);
                                                    return Ok(sig);
                                                }
                                            }
                                        } else {
                                            println!("No status");
                                        }
                                    }
                                }
                            }

                            // Handle confirmation errors
                            Err(err) => {
                                println!("Confirm Error: {:?}", err);
                            }
                        }
                    }
                    // println!("Transaction did not land");
                }

                // Handle submit errors
                Err(err) => {
                    println!("Submit Error: {:?}", err);

                    if err.get_transaction_error().is_some() {
                        return Err(err);
                    }
                }
            }

            if dynamic_fee {
                if let Some(dynamic_fee) = &self.dynamic_priority_fee {
                    instruction.compute_unit_price = Some(dynamic_fee.get(&client).await?);
                    tx = Transaction::new_with_payer(
                        &instruction.to_vec(),
                        Some(&fee_payer.pubkey()),
                    );
                }
            }

            (hash, slot) = client
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .await
                .unwrap();
            send_cfg = RpcSendTransactionConfig {
                skip_preflight: self.skip_preflight,
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                encoding: Some(UiTransactionEncoding::Base64),
                max_retries: Some(self.rpc_retries),
                min_context_slot: Some(slot),
            };

            tx.sign(&[&signer, &fee_payer], hash);

            attempts += 1;
            if attempts > self.submit_retries {
                return Err(ClientError {
                    request: None,
                    kind: ClientErrorKind::Custom("Max retries".into()),
                });
            }
        }
    }
}
