use ore::BUS_ADDRESSES;
use solana_client::{client_error::Result, nonblocking::rpc_client::RpcClient};

pub struct DynamicFee {
    pub min_fee: Option<u64>,
    pub max_fee: Option<u64>,
    pub uplift_fee: u64,
    pub baseline_pct: u8,
}

impl DynamicFee {
    pub async fn get(&self, client: &RpcClient) -> Result<u64> {
        let mut fees = client
            .get_recent_prioritization_fees(&BUS_ADDRESSES)
            .await?
            .iter()
            .filter_map(|fee| {
                if fee.prioritization_fee > 0 {
                    Some(fee.prioritization_fee)
                } else {
                    None
                }
            })
            .collect::<Vec<u64>>();
        fees.sort();

        let baseline = fees[(self.baseline_pct as f32 / 100f32 * fees.len() as f32) as usize];
        let mut final_fee = baseline + self.uplift_fee;

        if let Some(min_fee) = self.min_fee {
            final_fee = final_fee.max(min_fee);
        }
        if let Some(max_fee) = self.max_fee {
            final_fee = final_fee.min(max_fee);
        }

        Ok(final_fee)
    }
}
