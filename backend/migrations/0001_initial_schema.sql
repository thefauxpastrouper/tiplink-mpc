-- Create TipLinks table
CREATE TABLE IF NOT EXISTS tiplinks (
    id UUID PRIMARY KEY,
    public_key VARCHAR(255) NOT NULL UNIQUE,
    state VARCHAR(50) NOT NULL DEFAULT 'waiting', -- waiting, claimed, emptied
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create Transactions table (to store parsed indexer data)
CREATE TABLE IF NOT EXISTS transactions (
    signature VARCHAR(255) PRIMARY KEY,
    tiplink_id UUID REFERENCES tiplinks(id),
    sender VARCHAR(255) NOT NULL,
    receiver VARCHAR(255) NOT NULL,
    amount BIGINT NOT NULL,
    token_mint VARCHAR(255), -- NULL if SOL
    is_buy BOOLEAN,
    block_time BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_transactions_tiplink_id ON transactions(tiplink_id);
CREATE INDEX idx_transactions_sender ON transactions(sender);
CREATE INDEX idx_transactions_receiver ON transactions(receiver);

-- Create Transactions trace table (for MPC signing tracking)
CREATE TABLE IF NOT EXISTS transactions_trace (
    id UUID PRIMARY KEY,
    tiplink_id UUID REFERENCES tiplinks(id),
    signature VARCHAR(255),
    status VARCHAR(50) NOT NULL, -- pending, success, failed
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
