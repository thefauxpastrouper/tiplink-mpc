use std::sync::Arc;

use solana_client::rpc_response::{OptionSerializer, UiInnerInstructions, UiInstruction, UiParsedInstruction, UiTransactionTokenBalance};
use solana_client::pubkey::PubKey;
use solana_sdk::bs58;
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_proto::prelude::InnerInstruction;
use yellowstone_vixen_core::instruction::{InstructionShared, InstructionUpdate};

pub struct VixenUtils;

impl VixenUtils {
    // Convert gRPC inner instructions to Vixen InstructionUpdates
    pub fn convert_protobuf_inner_instruction(
        inner_ixs: &Vec<InnerInstruction>,
        all_accounts: &Vec<PubKey>,
        shared: Arc<InstructionShared,
    ) -> Option<Vec<InstructionUpdate>> {
        let data = inner_ixs
            .iter()
            .filter_map(|ix| {
                if ix.program_id_index > all_accounts.len() as u32 {
                    return None;
                }

                let pgm_id = all_accounts[ix.program_id_index as usize];
                let accs = all_accounts
                    .iter()
                    .map(|acc| yellowstone_vixen_parser::PubKey::from(acc.to_bytes()))
                    .collect();

                Some(InstructionUpdate {
                    data: ix.data.clone(),
                    shared: shared.clone(),
                    inner: vec![],
                    accounts: accs,
                    program: yellowstone_vixen_parser::pubkey::PubKey::from(pgm_id.to_bytes())
                })
            })
            .collect::<Vec<InstructionUpdate>>();

        Some(data)
    }

    pub fn convert_rpc_inner_instruction(
        ui_ix: &UiInstruction,
        all_accounts: &[PubKey],
        shared: Arc<InstructionShared>
    ) -> Option<InstructionUpdate> {
        match ui_ix {
            UiInstructionUpdate::Compiled(compiled) => {
                let prog_id = all_accounts.get(compiled.prog_id_index as usize)?;
                let accs: Vec<yellowstone_vixen_parser::Pubkey> = compiled
                .accounts
                .iter()
                .filter_map(|&idx| all_accounts.get(idx as usize))
                .map(|a| yellowstone_vixen_parser::PubKey::from(a.to_bytes()))
                .collect();

                let data = bs58::decode(&compiled.data).into_vec().ok()?;
                Some(InstructionUpdate {
                    program: yellowstone_vixen_parser::PubKey::from(prog_id.to_bytes()),
                    accounts: accs,
                    data,
                    shared,
                    inner: vec![]
                })
            }
            UiInstruction::Parsed(parsed) => match parsed {
                UiParsedInstruction::PartiallyDecoded(pd) => {
                    let prog_id = pd.program_id.parse<PubKey>().ok()?;
                    let accs = Vec<yellowstone_vixen_parser::PubKey> = pd
                        .accounts
                        .iter()
                        .filter_map(|a| a.parse::<PubKey>().ok())
                        .map(|a| yellowstone_vixen_parser::PubKey::from(a.to_bytes()))
                        .collect();
                    let data = bs58::decode(&pd.data).into_vec().ok()?;

                    Some(InstructionUpdate{
                        program: yellowstone_vixen_parser::PubKey::from(prog_id.to_bytes()),
                        accounts: accs,
                        data,
                        shared,
                        inner: vec![]
                    })
                }
                UiParsedInstruction::Parsed(_json) => {
                    tracing::warn!("Skipping JSON-parsed inner instruction (Vixen needs raw bytes)");
                    None
                }
            }
        }
    }

    // Build VixenInstructionUpdate for RPC path
    pub fn to_vixen_update_rpc(
        program_id: &PubKey,
        data: &[u8],
        accounts: &[Pubkey],
        signature: &str,
        slot: u64,
        inner_ixs: Option<&UiInnerInstructions>
    ) -> InstructionUpdate {
        let shared = Arc::new(InstructionShared {
            signature: signature.as_bytes().to_vec(),
            slot,
            ..Default::default()
        });

        let inner_updates = if let Some(inner_container) = inner_ixs {
            innner_container
                .instructions
                .iter()
                .filter_map(|ui_ix| Self::convert_rpc_inner_instruction(
                    ui_ix, 
                    all_accounts, 
                    shared.clone()))
                .collect()
        } else {
            Vec::new()
        };

        InstructionUpdate {
            program: yellowstone_vixen_parser::PubKey::from(program_id.to_bytes()),
            accounts: accounts
                    .iter()
                    .map(|a| yellowstone_vixen_parser::PubKey::from(a.to_bytes()))
                    .collect(),
            data: data.to_vec(),
            shared,
            inner: inner_updates
        }
    }

    // Convert gRPC token balances to RPC-style OptionSerializer format
    pub fn convert_token_balances_grpc(
        pre_token_balances: &[yellowstone_grpc_proto::prelude::TokenBalance],
    ) -> OptionSerializer<Vec<UiTransactionTokenBalance>> {
        OptionSerializer::Some(
            pre_token_balances
                .iter()
                .map(|b| UiTransactionTokenBalance {
                    account_index: b.account_index as u8,
                    mint: b.mint.clone(),
                    ui_token_amount: solana_client::rpc_response::UiTokenAmount {
                        ui_amount: b.ui_token_amount.clone().and_then(|a| Some(a.ui_amount)),
                        decimals: b.ui_token_amount
                                    .as_ref()
                                    .map(|a| a.decimals as u8)
                                    .unwrap_or(0),
                        amount: b.ui_token_amount
                                    .as_ref()
                                    .map(|a| a.amount.clone())
                                    .unwrap_or_default(),
                        ui_amount_string: b.ui_token_amount
                                    .as_ref()
                                    .map(|a| a.ui_amount_string.clone())
                                    .unwrap_or_default()
                    },
                    owner: OptionSerializer::Some(b.owner.clone()),
                    program_id: OptionSerializer::Some(b.program_id.clone())
                }).collect()
        )
    }

    // Extract mint address from token account using pre_token_balances
    pub fn get_mint(
        account_pubkey: &yellowstone_vixen_parser::PubKey,
        all_accounts: &[Pubkey],
        pre_token_balances: &OptionSerializer<Vec<UiTransactionTokenBalance>>
    ) -> String {
        let token_pubkey = Pubkey::new_from_array(account_pubkey.0);

        let token_account_index = all_accounts.iter().position(|k| *k == token_pubkey);

        if let Some(token_idx) = token_account_index {
            if let OptionSerializer::Some(token_balances) = pre_token_balances {
                let data = token_balances 
                    .iter()
                    .find(|token_bal| token_bal.account_index == token_idx as u8);

                if let Some(token_balance) = data {
                    return token_balance.mint.clone();
                }
            }
        }
        // If not found, assumed wrapped SOL
        crate::domain::WSOL_MINT.to_string()
    }

    // Reconstruct all messages from gRPC message and loaded addresses
    pub fn extract_accounts_from_grpc(
        message_account_keys: &[Vec<u8>],
        loaded_writable: &[Vec<u8>],
        loaded_readonly: &[Vec<u8>]
    ) -> Vec<PubKey> {
        let mut all_accounts = Vec::with_capacity(
            message_account_keys.len() + loaded_writable.len() + loaded_readonly.len()
        );

        for key_bytes in message_account_keys {
            if let Ok(key_array) = key_bytes.clone().try_into() {
                all_accounts.push(Pubkey::new_from_array(key_array));
            }
        }

        for key_bytes in loaded_writable {
            if let Ok(key_array) = key_bytes.clone().try_into() {
                all_accounts.push(Pubkey::new_from_array(key_array));
            }
        }

        for key_array in loaded_readonly {
            if let Ok(key_array) = key_bytes.clone().try_into() {
                all_accounts.push(Pubkey::new_from_array(key_array));
            }
        }

        all_accounts
    }
}