use serde::{Serialize, Deserialize};
use crate::{PartyId};
// A message can be sent between parties in MPC protocol.
pub struct Message<M> {
    /// The sender of the message
    pub sender: PartyId,
    /// The recceiver of the message
    pub recipient: Option<PartyId>,
    /// The round number this message belongs to
    pub round: u64,
    /// The actual payload
    pub payload: M
}