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

soroban contract optimize --wasm normal_votes.wasm
soroban contract optimize --wasm normal_vesting.wasm
soroban contract optimize --wasm normal_governor.wasm

soroban contract optimize --wasm normal_market.wasm
soroban contract optimize --wasm normal_market_factory.wasm

# soroban contract optimize --wasm normal_index_token.wasm
# soroban contract optimize --wasm normal_index_token_factory.wasm

soroban contract optimize --wasm normal_insurance.wasm
soroban contract optimize --wasm normal_scheduler.wasm

echo "Contracts optimized."

# Fetch the admin's address
ADMIN_ADDRESS=$(soroban keys address $IDENTITY_STRING)

echo "Deploy the soroban_token_contract and capture its contract ID hash..."

XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

TOKEN_ADDR1=$XLM

TOKEN_ADDR2=$(soroban contract deploy \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

soroban contract invoke \
    --id $TOKEN_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --decimal 7 \
    --name NORMAL \
    --symbol NORM

echo "NORM Token initialized."

FACTORY_ADDR=$(soroban contract deploy \
    --wasm normal_synth_market_factory.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

echo "Tokens and factory deployed."

# Sort the token addresses alphabetically
if [[ "$TOKEN_ADDR1" < "$TOKEN_ADDR2" ]]; then
    TOKEN_ID1=$TOKEN_ADDR1
    TOKEN_ID2=$TOKEN_ADDR2
else
    TOKEN_ID1=$TOKEN_ADDR2
    TOKEN_ID2=$TOKEN_ADDR1
fi

echo "Install the soroban_token, normal_amm and normal_synth_market contracts..."

TOKEN_WASM_HASH=$(soroban contract install \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

# Continue with the rest of the deployments
SYNTH_MARKET_WASM_HASH=$(soroban contract install \
    --wasm normal_synth_market.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

# ...

echo "Token, pair and stake contracts deployed."

echo "Initialize factory..."

# MULTIHOP=$(soroban contract install \
#     --wasm phoenix_multihop.optimized.wasm \
#     --source $IDENTITY_STRING \
#     --network $NETWORK)

soroban contract invoke \
    --id $FACTORY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --synth_market_wasm_hash $SYNTH_MARKET_WASM_HASH \
    --token_wasm_hash $TOKEN_WASM_HASH \
    # --whitelisted_accounts "[ \"${ADMIN_ADDRESS}\" ]" \
    # --lp_token_decimals 7

echo "Factory initialized: " $FACTORY_ADDR

echo "Initialize pair using the previously fetched hashes through factory..."

soroban contract invoke \
    --id $FACTORY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    create_synth_market \
    --sender $ADMIN_ADDRESS \
    --lp_init_info "{ \"admin\": \"${ADMIN_ADDRESS}\", \"swap_fee_bps\": 1000, \"fee_recipient\": \"${ADMIN_ADDRESS}\", \"max_allowed_slippage_bps\": 10000, \"default_slippage_bps\": 3000, \"max_allowed_spread_bps\": 10000, \"max_referral_bps\": 5000, \"token_init_info\": { \"token_a\": \"${TOKEN_ID1}\", \"token_b\": \"${TOKEN_ID2}\" }, \"stake_init_info\": { \"min_bond\": \"100\", \"min_reward\": \"100\", \"max_distributions\": 3, \"manager\": \"${ADMIN_ADDRESS}\", \"max_complexity\": 7 } }" \
    --default_slippage_bps 3000 \
    --max_allowed_fee_bps 10000 \
    --share_token_name "XLMPHOST" \
    --share_token_symbol "XPST" \
    --pool_type 0

echo "Query XLM/NORM pair address..."

MARKET_ADDR=$(soroban contract invoke \
    --id $FACTORY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 100 \
    -- \
    query_markets | jq -r '.[0]')

echo "Market contract initialized."

echo "Mint NORM token to the admin and provide liquidity..."
soroban contract invoke \
    --id $TOKEN_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    mint --to $ADMIN_ADDRESS --amount 10000000000 # 7 decimals, 10k tokens

# Provide liquidity in 2:1 ratio to the pool
soroban contract invoke \
    --id $MARKET_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 10000000 \
    -- \
    provide_liquidity --sender $ADMIN_ADDRESS --desired_a 2000000000 --desired_b 2000000000

echo "Liquidity provided."

# Continue with the rest of the commands
echo "Query stake contract address..."

STAKE_ADDR=$(soroban contract invoke \
    --id $MARKET_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 10000000 \
    -- \
    query_stake_contract_address | jq -r '.')

echo "Bond tokens to stake contract..."
# Bond token in stake contract
soroban contract invoke \
    --id $STAKE_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    bond --sender $ADMIN_ADDRESS --tokens 70000000

echo "Tokens bonded."

echo "#############################"

# TOKEN_ADDR2 stays the same - $PHO
TOKEN_ADDR1=$(soroban contract deploy \
    --wasm soroban_token_contract.optimized.wasm \
    --source $IDENTITY_STRING \
    --network $NETWORK)

soroban contract invoke \
    --id $TOKEN_ADDR1 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    initialize \
    --admin $ADMIN_ADDRESS \
    --decimal 7 \
    --name USDC \
    --symbol USDC

echo "USDC Token initialized."

# Sort the token addresses alphabetically
if [[ "$TOKEN_ADDR1" < "$TOKEN_ADDR2" ]]; then
    TOKEN_ID1=$TOKEN_ADDR1
    TOKEN_ID2=$TOKEN_ADDR2
else
    TOKEN_ID1=$TOKEN_ADDR2
    TOKEN_ID2=$TOKEN_ADDR1
fi

echo "Initialize pair through factory..."

soroban contract invoke \
    --id $FACTORY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    create_liquidity_pool \
    --sender $ADMIN_ADDRESS \
    --lp_init_info "{ \"admin\": \"${ADMIN_ADDRESS}\", \"swap_fee_bps\": 1000, \"fee_recipient\": \"${ADMIN_ADDRESS}\", \"max_allowed_slippage_bps\": 10000, \"default_slippage_bps\": 3000, \"max_allowed_spread_bps\": 10000, \"max_referral_bps\": 5000, \"token_init_info\": { \"token_a\": \"${TOKEN_ID1}\", \"token_b\": \"${TOKEN_ID2}\" }, \"stake_init_info\": { \"min_bond\": \"100\", \"min_reward\": \"100\", \"max_distributions\": 3, \"manager\": \"${ADMIN_ADDRESS}\", \"max_complexity\": 7 } }" \
    --default_slippage_bps 3000 \
    --max_allowed_fee_bps 10000 \
    --share_token_name "XLMPHOST" \
    --share_token_symbol "XPST" \
    --pool_type 0

echo "Query PHO/USDC pair address..."

MARKET_ADDR2=$(soroban contract invoke \
    --id $FACTORY_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 100 \
    -- \
    query_pools | jq -r '.[1]')

echo "Pair contract initialized."

echo "Mint PHO & USDC token to the admin and provide liquidity..."
soroban contract invoke \
    --id $TOKEN_ADDR1 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    mint --to $ADMIN_ADDRESS --amount 10000000000 # 7 decimals, 10k tokens

soroban contract invoke \
    --id $TOKEN_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    mint --to $ADMIN_ADDRESS --amount 10000000000 # 7 decimals, 10k tokens

# Provide liquidity in 2:1 ratio to the pool
soroban contract invoke \
    --id $MARKET_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 10000000 \
    -- \
    provide_liquidity --sender $ADMIN_ADDRESS --desired_a 6000000000 --desired_b 2000000000

echo "Liquidity provided."

# Continue with the rest of the commands
echo "Query stake contract address..."

STAKE_ADDR2=$(soroban contract invoke \
    --id $MARKET_ADDR \
    --source $IDENTITY_STRING \
    --network $NETWORK --fee 10000000 \
    -- \
    query_stake_contract_address | jq -r '.')

echo "Bond tokens to stake contract..."
# Bond token in stake contract
soroban contract invoke \
    --id $STAKE_ADDR2 \
    --source $IDENTITY_STRING \
    --network $NETWORK \
    -- \
    bond --sender $ADMIN_ADDRESS --tokens 70000000

echo "Tokens bonded."

echo "#############################"

echo "Initialization complete!"
echo "XLM address: $XLM"
echo "NORM address: $TOKEN_ADDR2"
echo "USDC address: $TOKEN_ADDR1"
echo "XLM/NORM Pair Contract address: $PAIR_ADDR"
echo "XLM/NORM Stake Contract address: $STAKE_ADDR"
echo "NORM/USDC Pair Contract address: $PAIR_ADDR2"
echo "NORM/USDC Stake Contract address: $STAKE_ADDR2"
echo "Factory Contract address: $FACTORY_ADDR"
echo "Multihop Contract address: $MULTIHOP"
