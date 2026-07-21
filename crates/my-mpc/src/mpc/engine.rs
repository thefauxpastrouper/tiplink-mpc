use crate::{Delivery, Mpc, Message, PartyId, PartyCount, Error};
use std::collections::VecDeque;

/// A concrete implementation of the `Mpc` trait backed by a `Delivery` channel
pub struct MpcEngine<M, D> {
    party_id: PartyId,
    party_count: PartyCount,
    delivery: D,
    // a buffer for received messages
    message_buffer: VecDeque<Message<M>>
}

impl<M, D> MpcEngine<M, D> 
where 
    D: Delivery<M>,
    M: Clone
{
    pub fn new(party_id: PartyId, party_count: PartyCount, delivery: D) -> Self {
        Self {
            party_id,
            party_count,
            delivery,
            message_buffer: VecDeque::new()
        }
    }
}

impl<M, D> Mpc<M> for MpcEngine<M, D> 
where 
    D: Delivery<M> + Unpin + Send,
    M: Clone + Send + 'static
{
    fn party_id(&self) -> PartyId { self.party_id }
    fn party_count(&self) -> PartyCount { self.party_count }

    async fn send_to(&mut self, recipient: PartyId, payload: M) -> Result<(), Error> {
        let msg = Message {
            sender: self.party_id,
            recipient: Some(recipient),
            round: 0,
            payload
        };

        use futures::SinkExt;
        self.delivery.send(msg).await?;
        Ok(())
    }

    async fn broadcast(&mut self, payload: M) -> Result<(), Error> {
        let msg = Message {
            sender: self.party_id,
            recipient: None,
            round: 0,
            payload
        };

        use futures::SinkExt;
        self.delivery.send(msg).await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<(), Error> {
        use futures::StreamExt;
        if let Some(msg_result) = self.delivery.next().await {
            let msg = msg_result?;
            self.message_buffer.push_back(msg);
            Ok(())
        } else {
            // Wait, we need an Error type but it's not well defined in the crate yet.
            // Let's assume Error has a generic error conversion or we can panic.
            return Err(Error::ChannelClosed);
        }
    }

    async fn receive_from(&mut self, sender: PartyId, round: u64) -> Result<M, Error> {
        // Check buffer first
        if let Some(idx) = self.message_buffer.iter().position(|msg| msg.sender == sender && msg.round == round) {
            let msg = self.message_buffer.remove(idx).unwrap();
            return Ok(msg.payload);
        }

        use futures::StreamExt;
        // Read from delivery until we get the message
        while let Some(msg_result) = self.delivery.next().await {
            let msg = msg_result?;
            if msg.sender == sender && msg.round == round {
                return Ok(msg.payload);
            } else {
                self.message_buffer.push_back(msg);
            }
        }

        Err(Error::ChannelClosed)
    }
}