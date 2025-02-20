# Ensure the script exits on any errors
set -e

# Check if the argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <identity_string>"
    exit 1
fi

IDENTITY_STRING=$1
NETWORK="testnet"

echo "Build and optimize the contracts..."

make build >/dev/null
cd target/wasm32-unknown-unknown/release

echo "Contracts compiled."
echo "Optimize contracts..."

stellar contract optimize --wasm soroban_token_contract.wasm

# stellar contract optimize --wasm normal_market.wasm
stellar contract optimize --wasm normal_market_factory.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(stellar keys address $IDENTITY_STRING)

echo "Deploy the soroban_token_contract and capture its contract ID hash..."

XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

# TOKEN_ADDR1=$XLM

# TOKEN_ADDR2=$(stellar contract deploy \
#     --wasm soroban_token_contract.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK)


FACTORY_ADDR=$(stellar contract deploy \
    --wasm normal_market_factory.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

echo "Tokens and factory deployed."

# Sort the token addresses alphabetically
# if [[ "$TOKEN_ADDR1" < "$TOKEN_ADDR2" ]]; then
#     TOKEN_ID1=$TOKEN_ADDR1
#     TOKEN_ID2=$TOKEN_ADDR2
# else
#     TOKEN_ID1=$TOKEN_ADDR2
#     TOKEN_ID2=$TOKEN_ADDR1
# fi

echo "Install the soroban_token, normal_market and normal_market_factory contracts..."

TOKEN_WASM_HASH=$(stellar contract install \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

# Continue with the rest of the deployments
# MARKET_WASM_HASH=$(stellar contract install \
#     --wasm normal_market.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK)

echo "Token contract deployed."

echo "Initialize factory..."

stellar contract invoke \
    --id $FACTORY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --insurance $ADMIN_ADDRESS \
    --market_wasm_hash $TOKEN_WASM_HASH \
    --token_wasm_hash $TOKEN_WASM_HASH

echo "Factory initialized: " $FACTORY_ADDR

echo "#############################"

echo "Initialization complete!"
echo "Factory Contract address: $FACTORY_ADDR"

# stellar contract bindings typescript \
#   --network testnet \
#   --source josh \
#   --contract-id CAPJ6T4ICUHQ5FQU2EUQJCCKGZYMAO7C3T2USW2HMQJUX4P2XG7FA23L \
#   --output-dir packages/factory