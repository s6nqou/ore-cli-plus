use std::str::FromStr;

use ore::{self, state::Proof, utils::AccountDeserialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signer};

use crate::{cu_limits::CU_LIMIT_CLAIM, instruction::OreInstruction, utils::proof_pubkey, Miner};

impl Miner {
    pub async fn claim(
        &self,
        cluster: String,
        beneficiary: Option<String>,
        amount: Option<f64>,
    ) -> Result<(), String> {
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());
        let beneficiary = match beneficiary {
            Some(beneficiary) => {
                Pubkey::from_str(&beneficiary).expect("Failed to parse beneficiary address")
            }
            None => self.initialize_ata().await,
        };
        let amount = if let Some(amount) = amount {
            (amount * 10f64.powf(ore::TOKEN_DECIMALS as f64)) as u64
        } else {
            match client.get_account(&proof_pubkey(pubkey)).await {
                Ok(proof_account) => {
                    let proof = Proof::try_from_bytes(&proof_account.data).unwrap();
                    proof.claimable_rewards
                }
                Err(err) => {
                    println!("Error looking up claimable rewards: {:?}", err);
                    return Err(err.to_string());
                }
            }
        };
        let amountf = (amount as f64) / (10f64.powf(ore::TOKEN_DECIMALS as f64));
        let instruction = OreInstruction::new(
            Some(CU_LIMIT_CLAIM),
            Some(self.default_priority_fee),
            ore::instruction::claim(pubkey, beneficiary, amount),
        );
        println!("Submitting claim transaction...");
        match self.send_and_confirm(instruction, false, true, false).await {
            Ok(sig) => {
                println!("Claimed {:} ORE to account {:}", amountf, beneficiary);
                println!("{:?}", sig);
                Ok(())
            }
            Err(err) => {
                println!("Error: {:?}", err);
                Err(err.to_string())
            }
        }
    }

    async fn initialize_ata(&self) -> Pubkey {
        // Initialize client.
        let signer = self.signer();
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::confirmed());

        // Build instructions.
        let token_account_pubkey = spl_associated_token_account::get_associated_token_address(
            &signer.pubkey(),
            &ore::MINT_ADDRESS,
        );

        // Check if ata already exists
        if let Ok(Some(_ata)) = client.get_token_account(&token_account_pubkey).await {
            return token_account_pubkey;
        }

        // Sign and send transaction.
        let instruction = OreInstruction::new(
            None,
            None,
            spl_associated_token_account::instruction::create_associated_token_account(
                &signer.pubkey(),
                &signer.pubkey(),
                &ore::MINT_ADDRESS,
                &spl_token::id(),
            ),
        );
        println!("Creating token account {}...", token_account_pubkey);
        match self.send_and_confirm(instruction, true, false, false).await {
            Ok(_sig) => println!("Created token account {:?}", token_account_pubkey),
            Err(e) => println!("Transaction failed: {:?}", e),
        }

        // Return token account address
        token_account_pubkey
    }
}
