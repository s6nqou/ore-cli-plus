#!/bin/bash

source .env

total_amount=0

for keypair_path in $KEYPAIR_ROOT/*; do
    pubkey=$(solana-keygen pubkey $keypair_path)
    rewards=$(ore --rpc $DEFAULT_RPC rewards $(solana-keygen pubkey $keypair_path))

    if [ $? -ne 0 ]; then
        continue
    fi

    amount=$(echo $(echo $rewards | tr -cd '[0-9\.]+') | bc)
    total_amount=$(echo "$total_amount+$amount" | bc)
    echo $keypair_path $rewards
done

printf "\nTotal rewards: $(printf "%.9f" $total_amount) ORE\n"
