#!/bin/bash

source .env

keypair=$1
rpc=${2:-$DEFAULT_RPC}
submit_rpc=${3:-$DEFAULT_RPC}

wallet_pubkey=$(solana-keygen pubkey $WALLET_KEYPAIR)
token_mint_address=oreoN2tQbHXVaZsr3pf66A48miqcBXCDJozganhEJgz
beneficiary_token_address=$(spl-token address --url $DEFAULT_RPC --owner $wallet_pubkey --token $token_mint_address --verbose | awk -F ': ' '{print $2}' | awk 'NR==2 {print}')
pubkey=$(solana-keygen pubkey $keypair)

while true; do
    balance=$(solana balance --keypair $WALLET_KEYPAIR --url $DEFAULT_RPC)
    balance_amount=$(echo $(echo $balance | tr -cd '[0-9\.]+') | bc)

    if [ $(echo "$balance_amount < 0.001" | bc) -eq 1 ]; then
        exit 0
    fi

    rewards=$(ore --rpc $DEFAULT_RPC rewards $pubkey)
    rewards_amount=$(echo $(echo $rewards | tr -cd '[0-9\.]+') | bc)

    if [ $(echo "$rewards_amount == 0" | bc) -eq 1 ]; then
        exit 0
    fi

    ore \
        --rpc $rpc \
        --submit-rpc $submit_rpc \
        --keypair $keypair \
        --fee-payer $WALLET_KEYPAIR \
        --default-priority-fee $CLAIM_PRIORITY_FEE \
        --submit-retries $SUBMIT_RETRIES \
        --confirm-retries $CONFIRM_RETRIES \
        --confirm-wait-ms $CONFIRM_WAIT_MS \
        --skip-preflight \
        claim \
        --beneficiary $beneficiary_token_address

    if [ $? -eq 0 ]; then
        break
    fi
done
