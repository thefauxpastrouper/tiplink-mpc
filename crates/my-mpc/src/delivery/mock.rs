use tokio::sync::mpsc;
use futures::{SinkExt, StreamExt};
use crate::{Delivery, Message, PartyId, Error};

/// A mock delivery channel for testing protocols locally
pub struct MockDelivery<M> {
    sender: mpsc::UnboundedSender<Message<M>>,
    receiver: mpsc::UnboundedReceiver<Message<M>>
}

impl<M> MockDelivery<M> {
    pub fn new(party_id: PartyId, num_parties: PartyCount) -> Self {
        // ... setup channels for each party ...
        unimplemented!()
    }
}

impl<M> Delivery for MockDelivery<M> {}