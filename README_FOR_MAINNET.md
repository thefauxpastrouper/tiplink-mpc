k# TipLink MPC Wallet

A secure, interactive 2-Party Multi-Party Computation (MPC) web3 wallet built for the Solana blockchain. This project allows users to generate TipLink wallets using distributed key generation (DKG) securely within the browser and transfer funds via a backend co-signer using interactive signing.

## Architecture
- **Backend:** Rust, Actix-Web, SQLx, Redis
- **Frontend:** React, Vite, Glassmorphism UI
- **Client Cryptography:** WebAssembly (WASM) via `tiplink-client-wasm`
- **Database:** PostgreSQL (for TipLink configurations and partial signatures)
- **Message Broker:** Redis (for indexer workers)
- **Blockchain:** Solana (Devnet)

---

## Complete Start Guide

Follow these steps in separate terminal windows to restart the entire TipLink infrastructure from scratch.

### 1. Start PostgreSQL and Redis (Docker)
The backend uses Postgres for database storage and Redis for message queueing. Start both using Docker:
```bash
# Start Postgres
docker run -d --name pg-tiplink -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres

# Start Redis
docker run -d --name redis-tiplink -p 6379:6379 redis
```

### 2. Build the WebAssembly (WASM) Module
If you haven't built the WASM module yet, or if you've made changes to `tiplink-client-wasm`, compile it so the frontend can use it:
```bash
cd tiplink-client-wasm
wasm-pack build --target web
cd ..
```

### 3. Start the Rust Backend
The Actix-Web backend handles 2-party secret sharing, co-signing, and Solana indexing.
Open a new terminal window:
```bash
cd backend

# Export required environment variables
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/tiplink"
export REDIS_URL="redis://127.0.0.1:6379"
export RPC_URL="https://api.devnet.solana.com"
export RUST_LOG="info"

# Ensure the database exists and migrations are run
sqlx database create
sqlx migrate run

# Start the backend server
cargo run
```
The backend should now be running on `http://127.0.0.1:8080`.

### 4. Start the React Frontend
Finally, start the Vite development server to launch the frontend interface.
Open a new terminal window:
```bash
cd frontend

# Install dependencies (only required on first run)
npm install

# Start the Vite dev server
npm run dev
```
The frontend should now be running on `http://localhost:5173`.

---

## Usage Guide
1. Go to `http://localhost:5173` in your browser.
2. Click **Create New TipLink Wallet**. The WASM client and Rust backend will engage in a distributed key generation round to securely generate a shared Solana wallet address.
3. Once the wallet is generated, **copy the TipLink Solana Address**.
4. Open your terminal and **airdrop funds** to your new wallet address so it can pay for transaction fees:
   ```bash
   solana airdrop 2 <YOUR_TIPLINK_ADDRESS> --url https://api.devnet.solana.com
   ```
5. Enter a destination Solana address and an amount of SOL in the UI.
6. Click **Send Funds**. The frontend and backend will complete an interactive MPC handshake to securely sign and broadcast the Solana transfer without either party revealing their private keys!
