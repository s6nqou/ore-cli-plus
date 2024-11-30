#!/bin/bash

source .env

paste -d \\n $KEYPAIR_ROOT/* > keypairs
