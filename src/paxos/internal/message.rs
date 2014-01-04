use super::replica::ReplicaID;
use super::instance::{InstanceID, SequenceID};

#[deriving(Clone, Encodable, Decodable)]
pub enum MessageContent {
    Propose(SequenceID),
    Promise(SequenceID),
    // The first id is the sequence id being rejected
    // The second is the sequence id based on which the first is rejected
    RejectPropose(SequenceID, SequenceID),

    Request(SequenceID, ~[u8]),
    Accept(SequenceID),
    // Similar to RejectPropose
    RejectRequest(SequenceID, SequenceID),

    Commit(SequenceID),
}

#[deriving(Clone, Encodable, Decodable)]
pub struct Message {
    instance_id: InstanceID,
    content: MessageContent,
}