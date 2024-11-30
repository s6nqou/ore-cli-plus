#!/bin/bash

source .env

create_number=$1

keypair_root=$KEYPAIR_ROOT
temp_keypair="./temp_keypair.json"

if [ ! -d $keypair_root ]; then
    mkdir $keypair_root
fi

for i in $(seq 1 $create_number); do

    solana-keygen new --no-bip39-passphrase --outfile $temp_keypair

    pubkey=$(solana-keygen pubkey $temp_keypair)

    while true; do
        solana transfer $pubkey 0.00155904 \
            --keypair $WALLET_KEYPAIR \
            --url $DEFAULT_RPC \
            --with-compute-unit-price $DEFAULT_PRIORITY_FEE \
            --allow-unfunded-recipient
        
        if [ $? -eq 0 ]; then
            break
        fi
    done

    mv $temp_keypair $keypair_root/$pubkey.json

done
