import { useState, useEffect } from 'react';
import axios from 'axios';
import bs58 from 'bs58';
import { ClientMpcContext } from 'tiplink-client-wasm';
import './App.css';

const API_BASE = import.meta.env.VITE_API_URL || '';

function App() {
  const [context, setContext] = useState(null);
  const [walletInfo, setWalletInfo] = useState(null);
  
  const [recipient, setRecipient] = useState('');
  const [amount, setAmount] = useState('');
  
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState('');
  const [error, setError] = useState('');
  const [txSignature, setTxSignature] = useState('');

  // Initialize the MPC Context on load
  useEffect(() => {
    try {
      const ctx = new ClientMpcContext();
      setContext(ctx);
    } catch (e) {
      console.error("Failed to initialize WASM MPC context", e);
      setError("Failed to initialize cryptographic context.");
    }
  }, []);

  const handleInitWallet = async () => {
    if (!context) return;
    setLoading(true);
    setStatus('Initializing interactive TipLink MPC wallet...');
    setError('');
    
    try {
      const pubkeyBytes = context.get_public_point();
      const pubkeyBase58 = bs58.encode(pubkeyBytes);
      
      const res = await axios.post(`${API_BASE}/api/wallet/init`, {
        client_pubkey: pubkeyBase58
      });
      
      setWalletInfo(res.data);
      setStatus('Wallet initialized successfully.');
    } catch (err) {
      console.error(err);
      setError(err.response?.data?.error || err.message);
      setStatus('');
    } finally {
      setLoading(false);
    }
  };

  const handleTransfer = async (e) => {
    e.preventDefault();
    if (!walletInfo || !context) return;
    setLoading(true);
    setError('');
    setTxSignature('');
    
    try {
      // 1. Generate Nonce Commitment
      setStatus('Generating nonce commitment (Round 1)...');
      const nonceCtx = context.compute_nonce_commitment();
      const rClientBytes = nonceCtx.get_r();
      const rClientB58 = bs58.encode(rClientBytes);
      
      const lamports = Math.floor(parseFloat(amount) * 1e9);

      // 2. Request Server Sign
      setStatus('Requesting server signature (Round 2)...');
      const signReq = await axios.post(`${API_BASE}/api/transfer/sign`, {
        tiplink_id: walletInfo.tiplink_id,
        to_address: recipient,
        lamports,
        r_client: rClientB58
      });
      
      const { r_server, s_server, message_data, recent_blockhash } = signReq.data;
      
      // 3. Compute Client Partial Signature
      setStatus('Computing client partial signature...');
      const sClientBytes = context.compute_partial_signature(
        nonceCtx.get_k(),
        bs58.decode(r_server),
        rClientBytes,
        bs58.decode(walletInfo.combined_pubkey),
        bs58.decode(message_data)
      );
      
      // 4. Combine Signatures
      setStatus('Combining signatures...');
      const combinedSignatureBytes = context.combine_signatures(
        sClientBytes,
        bs58.decode(s_server),
        rClientBytes,
        bs58.decode(r_server)
      );
      
      const combinedSignatureB58 = bs58.encode(combinedSignatureBytes);
      
      // 5. Submit Transaction
      setStatus('Submitting transaction to network...');
      const submitReq = await axios.post(`${API_BASE}/api/transfer/submit`, {
        tiplink_id: walletInfo.tiplink_id,
        to_address: recipient,
        lamports,
        recent_blockhash,
        signature: combinedSignatureB58
      });
      
      setStatus('Transfer complete!');
      setTxSignature(submitReq.data.signature);
    } catch (err) {
      console.error(err);
      setError(err.response?.data?.error || err.message);
      setStatus('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app-container">
      <div className="glass-panel">
        <h1 className="title">TipLink MPC</h1>
        <p className="subtitle">Secure Interactive 2-Party Web3 Wallet</p>
        
        {!walletInfo ? (
          <div className="init-section">
            <button 
              className="primary-btn" 
              onClick={handleInitWallet} 
              disabled={loading || !context}
            >
              {loading ? 'Initializing...' : 'Create New TipLink Wallet'}
            </button>
          </div>
        ) : (
          <div className="wallet-section">
            <div className="wallet-card">
              <div className="info-row">
                <span className="label">TipLink Address:</span>
                <span className="value">{walletInfo.combined_pubkey}</span>
              </div>
              <div className="info-row">
                <span className="label">Wallet ID:</span>
                <span className="value">{walletInfo.tiplink_id}</span>
              </div>
            </div>
            
            <form className="transfer-form" onSubmit={handleTransfer}>
              <h2>Transfer SOL</h2>
              
              <div className="input-group">
                <label>Recipient Address</label>
                <input 
                  type="text" 
                  value={recipient} 
                  onChange={(e) => setRecipient(e.target.value)} 
                  placeholder="Solana address"
                  required
                />
              </div>
              
              <div className="input-group">
                <label>Amount (SOL)</label>
                <input 
                  type="number" 
                  step="0.0001" 
                  min="0.0001"
                  value={amount} 
                  onChange={(e) => setAmount(e.target.value)} 
                  placeholder="0.1"
                  required
                />
              </div>
              
              <button 
                type="submit" 
                className="primary-btn submit-btn" 
                disabled={loading}
              >
                {loading ? 'Processing...' : 'Send Funds'}
              </button>
            </form>
          </div>
        )}
        
        {status && <div className="status-message">{status}</div>}
        {error && <div className="error-message">{error}</div>}
        
        {txSignature && (
          <div className="success-message">
            <p><strong>Success!</strong> Transaction Signature:</p>
            <a 
              href={`https://explorer.solana.com/tx/${txSignature}?cluster=devnet`} 
              target="_blank" 
              rel="noreferrer"
              className="tx-link"
            >
              {txSignature}
            </a>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
