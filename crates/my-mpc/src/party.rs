// src/party.rs
/// A unique identifier for a party in the protocol
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PartyId(pub u16);

impl PartyId {
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

// Total number of parties in the protocol.
pub type PartyCount = u16;
