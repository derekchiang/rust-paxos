use super::replica::ReplicaID;
use super::instance::{InstanceID, SequenceID};

#[deriving(Clone, Encodable, Decodable, ToStr)]
pub enum Message {
    NetworkM(NetworkMessage),
    PaxosM(PaxosMessage),
}

#[deriving(Clone, Encodable, Decodable, ToStr)]
pub struct NetworkMessage {
    replica_id: ReplicaID
}

#[deriving(Clone, Encodable, Decodable, ToStr)]
pub enum PaxosMessageContent {
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
    Acknowledge(SequenceID),
}

#[deriving(Clone, Encodable, Decodable, ToStr)]
pub struct PaxosMessage {
    instance_id: InstanceID,
    content: PaxosMessageContent,
}