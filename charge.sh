#/bin/bash

source .env

target_amount=$1

for keypair_path in $KEYPAIR_ROOT/*; do

    pubkey=$(solana-keygen pubkey $keypair_path)
    balance=$(solana balance $pubkey)
    amount=$(echo $(echo $balance | tr -cd '[0-9\.]+') | bc)

    echo $pubkey $balance

    if [ $(echo "$amount < $target_amount" | bc) -eq 1 ]; then
        increment=$(printf "%.9f" $(echo "$target_amount - $amount" | bc))
        solana transfer $pubkey $increment \
            --keypair $WALLET_KEYPAIR
            --url $DEFAULT_RPC \
            --with-compute-unit-price $DEFAULT_PRIORITY_FEE \
            --allow-unfunded-recipient \
            --no-wait
    fi

done
