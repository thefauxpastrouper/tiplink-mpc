-- Add MPC key share columns to tiplinks table
-- Both shares are stored server-side during testing.
-- In production, client_share would NEVER be stored on the server.
ALTER TABLE tiplinks ADD COLUMN IF NOT EXISTS server_pubkey VARCHAR(255);
ALTER TABLE tiplinks ADD COLUMN IF NOT EXISTS server_share BYTEA;
ALTER TABLE tiplinks ADD COLUMN IF NOT EXISTS client_share BYTEA;
