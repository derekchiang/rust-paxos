use extra::comm::DuplexStream;

use super::replica::ReplicaID;
use super::message::{Propose, Promise, Request, Accept, Commit, MessageContent, Message};

#[deriving(TotalOrd, Encodable, Decodable)]
pub type SequenceID = (uint, ReplicaID);
pub type InstanceID = (ReplicaID, uint);

fn increment_seq(sid: SequenceID) -> SequenceID {
    match sid {
        (seq, rid) => (seq + 1, rid)
    }
}

pub enum InstanceIdentity {
    Proposer,
    Acceptor,
}

pub enum InstanceState {
    // Initial state
    Null,

    // (#sequence, value, #promised)
    Proposed(SequenceID, uint),
    // (#sequence)
    Promised(SequenceID),

    // (#sequence, value, #accepted)
    Requested(SequenceID, ~[u8], uint),
    // (#sequence, value)
    Accepted(SequenceID, ~[u8]),

    // (#sequence, value)
    Committed(SequenceID, ~[u8]),
}

pub struct Instance {
    identity: InstanceIdentity,
    replica_id: ReplicaID,
    id: InstanceID,
    value: ~[u8],
    state: InstanceState,
    peers: ~[DuplexStream<MessageContent, MessageContent>],
}

impl Instance {
    pub fn new_as_proposer(rid: ReplicaID, iid: InstanceID, value: ~[u8],
        peer_chans: ~[SharedChan<(InstanceID, DuplexStream<MessageContent, MessageContent>)>]) -> Instance {
        let peers = peer_chans.move_iter().map(|chan| {
            let (from, to) = DuplexStream::new();
            chan.send((iid, to));
            from
        }).collect();

        Instance{
            identity: Proposer,
            replica_id: rid,
            id: iid,
            value: value,
            state: Null,
            peers: peers,
        }
    }

    pub fn new_as_acceptor(rid: ReplicaID, iid: InstanceID, seq: SequenceID,
        peer_chans: ~[SharedChan<(InstanceID, DuplexStream<MessageContent, MessageContent>)>]) -> Instance {
        let peers = peer_chans.move_iter().map(|chan| {
            let (from, to) = DuplexStream::new();
            chan.send((iid, to));
            from
        }).collect();

        Instance{
            identity: Acceptor,
            replica_id: rid,
            id: iid,
            value: ~[],
            state: Promised(seq),
            peers: peers,
        }
    }

    pub fn run(self) {
        match self.identity {
            Proposer => self.run_as_proposer(),
            Acceptor => self.run_as_acceptor(),
        }
    }

    fn run_as_proposer(mut self) {
        let seq_id = (0, self.replica_id);
        self.state = Proposed(seq_id, 0);

        for peer in self.peers.iter() {
            peer.send(Propose(seq_id))
        }
    }

    fn run_as_acceptor(self) {

    }
}