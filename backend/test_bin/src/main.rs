use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Signature, system_instruction,
    transaction::Transaction, hash::Hash
};
use std::str::FromStr;

fn main() {
    let from_pubkey = Pubkey::new_unique();
    let to_pubkey = Pubkey::new_unique();
    let blockhash = Hash::default();
    
    let transfer_ix = system_instruction::transfer(&from_pubkey, &to_pubkey, 1000);
    let message = Message::new_with_blockhash(&[transfer_ix], Some(&from_pubkey), &blockhash);
    
    let mut tx = Transaction::new_unsigned(message);
    println!("Signatures len: {}", tx.signatures.len());
    
    let signature_bytes = vec![1; 64];
    tx.signatures[0] = Signature::try_from(signature_bytes.as_slice()).unwrap();
    println!("Assigned signature");
}
