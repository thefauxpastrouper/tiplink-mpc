use futures::{Sink, Stream};
use crate::{Message, Error, PartyId};
use std::pin::Pin;

/// A channel for sending and receiving messages in an MPC protocol
/// 
/// Users implement this trait for their chosen transport layer
pub trait Delivery<M>: Stream<Item = Result<Message<M>, Error>> + Sink<Message<M>, Error = Error> + Unpin {}

// A blanket implementation for any type that implements the bound
impl<T, M> Delivery<M> for T 
where T: Stream<Item = Result<Message<M>, Error>> + Sink<Message<M>, Error = Error> + Unpin {}