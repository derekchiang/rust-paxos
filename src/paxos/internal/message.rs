use super::replica::ReplicaID;
use super::instance::{InstanceID, SequenceID};

#[deriving(Encodable, Decodable)]
pub enum MessageContent {
    Propose(SequenceID),
    Promise(SequenceID),
    Request(SequenceID, ~[u8]),
    Accept(SequenceID),
    Commit(SequenceID),
}

#[deriving(Encodable, Decodable)]
pub struct Message {
    instance_id: InstanceID,
    content: MessageContent,
}