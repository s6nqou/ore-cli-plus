#!/bin/bash

source .env

wallet_pubkey=$(solana-keygen pubkey $WALLET_KEYPAIR)

for keypair_path in $KEYPAIR_ROOT/*; do
    pubkey=$(solana-keygen pubkey $keypair_path)
    balance=$(solana balance $pubkey --url $DEFAULT_RPC)
    balance_amount=$(echo $(echo $balance | tr -cd '[0-9\.]+') | bc)

    if [ $(echo "$balance_amount == 0" | bc) -eq 1 ]; then
        continue
    fi

    solana transfer $wallet_pubkey ALL \
        --keypair $keypair_path \
        --fee-payer $WALLET_KEYPAIR \
        --url $DEFAULT_RPC \
        --with-compute-unit-price $DEFAULT_PRIORITY_FEE \
        --allow-unfunded-recipient \
        --no-wait
done
