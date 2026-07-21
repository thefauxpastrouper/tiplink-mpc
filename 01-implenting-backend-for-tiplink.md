# Implementation of Backend for My Project TipLink
- Tiplink is a web based HD(Hierarchical Deterministic) wallet that follows two party MPC for signing the transactions on Solana. This file explains how to create the backend for it.
- Create a folder named backend and then create everything inside it. Consider this is production code and produce code that is done in real scenarios.
- I want to use actix-web as the framework for building the backend. 
- I want to use postgress as my database and sqlx for quering the database. 
- I want to use this `https://github.com/coinbase/cb-mpc` MPC library to implemenent 2-party MPC between my backend and the frontend client.
- I want to use this `https://github.com/thefauxpastrouper/solana-indexer` as the indexer to index the solana blockchain. 
- This indexer should parse the transactions then push it to redis queue and then ultimately to postgress db which stores those particular transactions. 
- Next, create all necessary endpoints for signing the transactions using the MPC library and then sending it to the blockchain.
- You should trace the success and failure of the transactions. 
- Here are the details about what to be done with the indexer data
- Tracking Link/Wallet States: When someone creates a TipLink, funds (such as SOL, SPL tokens, or NFTs) are deposited into it. An indexer monitors the blockchain to track whether a specific TipLink has been claimed, emptied, or if funds are still waiting to be claimed.

- Fast Data Retrieval: Instead of scanning millions of blocks to find out what assets are associated with a specific link or user account, the indexer organizes blockchain data into an optimized, searchable database. This allows the website to instantly load wallet balances and contents the moment a user opens a link.

- Decoding Events and Transactions: Indexers parse raw transaction logs and smart contract events (such as transfers or claims executed via Solana Pay or custom programs) and translate them into a human-readable format that the TipLink frontend can display.

- Metadata Enrichment: Indexers help pull token and NFT metadata (like names, images, and descriptions) so that when a user views a TipLink containing a collectible, the UI renders it smoothly without delay. 