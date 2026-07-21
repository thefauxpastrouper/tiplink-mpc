use std::process::Output;

use crate::{Mpc, PartyId, party::PartyCount, Result};
use std::future::Future;

/// A trait for defining an MPC protocol
/// 
/// Implementors define the protocol's logic in the `run` method
pub trait Protocol<M, Output> {
    /// Execute the protocol, returning the final output.
    fn run(self, mpc: impl Mpc<M>) -> impl Future<Output = Result<Output>> + Send;
}

use futures::future::BoxFuture;

/// A protocol defined as an async function.
pub type AsyncProtocol<MpcImpl, Output> = fn(
    mpc: MpcImpl, 
    party_id: PartyId, 
    party_count: PartyCount
) -> BoxFuture<'static, Result<Output>>;
