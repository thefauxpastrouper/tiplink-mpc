use std::process::Output;

use crate::{Delivery, Message, PartyId, party::PartyCount, Error};
use futures::future::BoxFuture;

/// The primary interface for participating in the protocol
/// 
/// An `MPC` instance represents a single party's ability to send and receive 
/// messages during protocol execution.
pub trait Mpc<M> {
    /// The party's own ID
    fn party_id(&self) -> PartyId;
    /// The total number of parties in the protocol
    fn party_count(&self) -> PartyCount;
    /// Send a message to a specific recipient
    fn send_to(&mut self, recipient: PartyId, payload: M) -> impl Future<Output = Result<(), Error>> + Send;
    /// Send a message to all parties (broadcast)
    fn broadcast(&mut self, payload: M) -> impl Future<Output = Result<(), Error>> + Send;
    /// Receive the next message
    fn receive(&mut self) -> impl Future<Output = Result<(), Error>> + Send;
    /// Receive a message from a specific sender in a specific round
    fn receive_from(&mut self, sender: PartyId, round: u64) -> impl Future<Output = Result<M, Error>> + Send;
}