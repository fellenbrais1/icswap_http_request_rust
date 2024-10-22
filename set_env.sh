#!/bin/bash

export PRE_MINTED_TOKENS=10_000_000_000
dfx identity use default
export DEFAULT=$(dfx identity get-principal)
export TRANSFER_FEE=0
dfx identity use archive_controller
export ARCHIVE_CONTROLLER=$(dfx identity get-principal)
export TRIGGER_THRESHOLD=2000
export CYCLE_FOR_ARCHIVE_CREATION=10000000000000
export NUM_OF_BLOCK_TO_ARCHIVE=1000
export TOKEN_NAME="Lift Cash"
export TOKEN_SYMBOL="LIFT"
# dfx identity use minter
# export MINTER=$(dfx identity get-principal)
export MINTER=bd3sg-teaaa-aaaaa-qaaba-cai
export FEATURE_FLAGS=false
# Write variables to .env file
echo "PRE_MINTED_TOKENS=$PRE_MINTED_TOKENS" >> .env
echo "DEFAULT=$DEFAULT" >> .env
echo "TRANSFER_FEE=$TRANSFER_FEE" >> .env
echo "ARCHIVE_CONTROLLER=$ARCHIVE_CONTROLLER" >> .env
echo "TRIGGER_THRESHOLD=$TRIGGER_THRESHOLD" >> .env
echo "CYCLE_FOR_ARCHIVE_CREATION=$CYCLE_FOR_ARCHIVE_CREATION" >> .env
echo "NUM_OF_BLOCK_TO_ARCHIVE=$NUM_OF_BLOCK_TO_ARCHIVE" >> .env
echo "TOKEN_NAME=$TOKEN_NAME" >> .env
echo "TOKEN_SYMBOL=$TOKEN_SYMBOL" >> .env
echo "MINTER=$MINTER" >> .env
echo "FEATURE_FLAGS=$FEATURE_FLAGS" >> .env
dfx identity use Jesper