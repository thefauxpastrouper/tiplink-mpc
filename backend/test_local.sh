#!/bin/bash
set -e

# Cleanup trap
cleanup() {
    echo ""
    echo "Cleaning up processes..."
    [ -n "$BACKEND_PID" ] && kill $BACKEND_PID 2>/dev/null || true
    [ -n "$VALIDATOR_PID" ] && kill $VALIDATOR_PID 2>/dev/null || true
    echo "Done."
}
trap cleanup EXIT

echo "Starting local test environment..."

# Start Postgres if not running (using Docker for convenience if local pg isn't running)
if ! pg_isready -h localhost -U postgres 2>/dev/null; then
    echo "Starting PostgreSQL via Docker..."
    if docker ps -a | grep -q pg-tiplink; then
        docker start pg-tiplink
    else
        docker run -d --name pg-tiplink -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:14
    fi
    sleep 3
fi

# Create DB if it doesn't exist, drop if it does for a clean slate
export PGPASSWORD=postgres
dropdb -U postgres -h localhost tiplink 2>/dev/null || true
createdb -U postgres -h localhost tiplink || true

# Start Redis
if ! nc -z localhost 6379 2>/dev/null; then
    echo "Starting Redis via Docker..."
    if docker ps -a | grep -q redis-tiplink; then
        docker start redis-tiplink
    else
        docker run -d --name redis-tiplink -p 6379:6379 redis:latest
    fi
    sleep 2
fi

# Start solana-test-validator in background
echo "Starting solana-test-validator..."
killall solana-test-validator 2>/dev/null || true
solana-test-validator --reset --quiet &
VALIDATOR_PID=$!

echo "Waiting for validator RPC to be ready..."
while ! curl -s http://127.0.0.1:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1, "method":"getVersion"}' > /dev/null; do
    sleep 2
done
echo "Validator RPC is up!"

# Set env vars and start backend in background
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/tiplink"
export REDIS_URL="redis://127.0.0.1:6379"
export RPC_URL="http://127.0.0.1:8899"
export RUST_LOG="info"

echo "Starting TipLink backend..."
cargo run > backend.log 2>&1 &
BACKEND_PID=$!
sleep 5

# Test Flow
echo "========================================="
echo "Testing End-to-End Flow"
echo "========================================="

# 1. Generate a mock client key
RANDOM_SUFFIX=$(date +%s)
CLIENT_KEY="ClientMockPubkey111111111111${RANDOM_SUFFIX}"

# 2. Init Wallet (2-party DKG)
echo "1. Initializing TipLink Wallet (DKG)..."
INIT_RES=$(curl -s -X POST http://localhost:8080/api/wallet/init \
    -H "Content-Type: application/json" \
    -d "{\"client_pubkey\":\"$CLIENT_KEY\"}")

echo "Response: $INIT_RES"
TIPLINK_ID=$(echo $INIT_RES | grep -oP '"tiplink_id":"\K[^"]+')
SERVER_PUBKEY=$(echo $INIT_RES | grep -oP '"combined_pubkey":"\K[^"]+')

echo "TipLink ID: $TIPLINK_ID"
echo "Combined Pubkey: $SERVER_PUBKEY"

# 3. Airdrop SOL to the combined pubkey
echo "2. Airdropping 10 SOL to the wallet..."
solana airdrop 10 $SERVER_PUBKEY --url http://127.0.0.1:8899

# Wait a few seconds for the indexer to pick up the deposit
echo "Waiting for indexer to detect deposit..."
sleep 10

# Check TipLink status (should be 'funded')
echo "3. Checking TipLink status..."
STATUS_JSON=$(curl -s http://localhost:8080/api/tiplink/$TIPLINK_ID)
echo "Raw status: $STATUS_JSON"
echo $STATUS_JSON | grep -o '"state":"[^"]*"' || true

# 4. Transfer SOL (2-party signing)
echo "4. Transferring 1 SOL to random address..."
RANDOM_ADDR="EYRBe1NbUZQSykunpBoMUaHvrwNaW1hMpZPdQJ6zz1hg"
TRANSFER_RES=$(curl -s -X POST http://localhost:8080/api/transfer \
    -H "Content-Type: application/json" \
    -d "{\"tiplink_id\":\"$TIPLINK_ID\",\"to_address\":\"$RANDOM_ADDR\",\"lamports\":1000000000}")

echo "Response: $TRANSFER_RES"
SIG=$(echo $TRANSFER_RES | grep -oP '"signature":"\K[^"]+')

# 5. Check Transaction Status
echo "5. Checking transaction status..."
curl -s http://localhost:8080/api/transactions/status/$SIG
