#!/bin/bash

source .env

keypair=$1
rpc=${2:-$DEFAULT_RPC}
submit_rpc=${3:-$DEFAULT_RPC}

while true; do

    balance=$(solana balance --keypair $WALLET_KEYPAIR --url $DEFAULT_RPC)
    balance_amount=$(echo $(echo $balance | tr -cd '[0-9\.]+') | bc)

    if [ $(echo "$balance_amount < 0.001" | bc) -eq 1 ]; then
        exit 0
    fi

    ore \
        --rpc $rpc \
        --submit-rpc $submit_rpc \
        --keypair $keypair \
        --fee-payer $WALLET_KEYPAIR \
        --default-priority-fee $DEFAULT_PRIORITY_FEE \
        --dynamic-priority-fee \
        --dynamic-priority-fee-min $DYNAMIC_FEE_MIN \
        --dynamic-priority-fee-max $DYNAMIC_FEE_MAX \
        --dynamic-priority-fee-percentile $DYNAMIC_FEE_PERCENTILE \
        --dynamic-priority-fee-uplift $DYNAMIC_FEE_UPLIFT \
        --submit-retries $SUBMIT_RETRIES \
        --confirm-retries $CONFIRM_RETRIES \
        --confirm-wait-ms $CONFIRM_WAIT_MS \
        --skip-preflight \
        mine \
        --threads $MINE_THREADS

    if [ $? -eq 0 ]; then
        break
    fi

done
