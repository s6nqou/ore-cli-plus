use rand::Rng;
use reqwest::{Client, Result, Url};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

pub struct JitoClient {
    server_url: Url,
    pub tip_accounts: Vec<Pubkey>,
}

#[derive(Deserialize)]
struct GetTipAccountsResponse {
    data: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SendJitoBundleRequestTransaction {
    message_data: String,
    signatures: Vec<String>,
}

#[derive(Serialize)]
struct SendJitoBundleRequest {
    transactions: Vec<SendJitoBundleRequestTransaction>,
}

#[derive(Deserialize)]
struct SendJitoBundleResponse {
    data: String,
}

impl JitoClient {
    pub async fn new(server_url: &str) -> Result<Self> {
        let server_url = Url::parse(server_url).unwrap();

        let get_tip_accounts_url = server_url.join("/getTipAccounts").unwrap();

        let response = reqwest::get(get_tip_accounts_url)
            .await?
            .json::<GetTipAccountsResponse>()
            .await?;

        let tip_accounts = response
            .data
            .iter()
            .map(|key| Pubkey::try_from(bs58::decode(key).into_vec().unwrap()).unwrap())
            .collect();

        Ok(JitoClient {
            server_url,
            tip_accounts,
        })
    }

    pub fn get_random_tip_account(&self) -> Pubkey {
        let index = rand::thread_rng().gen_range(0..self.tip_accounts.len());
        self.tip_accounts[index]
    }

    pub async fn send_jito_bundle(&self, signed_transactions: Vec<&Transaction>) -> Result<String> {
        let send_jito_bundle_url = self.server_url.join("/sendJitoBundle").unwrap();

        let request_transactions = signed_transactions
            .iter()
            .map(|tran| {
                let message_data = bs58::encode(tran.message_data()).into_string();
                let signatures = tran
                    .signatures
                    .iter()
                    .map(|sig| bs58::encode(sig).into_string())
                    .collect();

                SendJitoBundleRequestTransaction {
                    message_data,
                    signatures,
                }
            })
            .collect();

        let request = SendJitoBundleRequest {
            transactions: request_transactions,
        };

        let client = Client::new();
        let response = client
            .post(send_jito_bundle_url)
            .json(&request)
            .send()
            .await?
            .json::<SendJitoBundleResponse>()
            .await?;

        Ok(response.data)
    }
}
