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

soroban contract optimize --wasm soroban_token_contract.wasm
soroban contract optimize --wasm normal_index_token.wasm
soroban contract optimize --wasm normal_index_token_factory.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

# ...

echo "#############################"

echo "Initialization complete!"
echo "Index Token Contract address: $MULTIHOP"
echo "Index Token Factory Contract address: $FACTORY_ADDR"
