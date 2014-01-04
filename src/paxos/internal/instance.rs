use extra::comm::DuplexStream;

use super::replica::ReplicaID;
use super::message::{Propose, Promise, RejectPropose, Request,
    Accept, RejectRequest, Commit, MessageContent, Message};

#[deriving(Clone, TotalOrd, Encodable, Decodable)]
pub type SequenceID = (uint, ReplicaID);

#[deriving(Clone, TotalOrd, Encodable, Decodable)]
pub type InstanceID = (ReplicaID, uint);

pub fn increment_seq(sid: SequenceID) -> SequenceID {
    match sid {
        (seq, rid) => (seq + 1, rid)
    }
}

pub fn increment_iid(iid: InstanceID) -> InstanceID {
    match iid {
        (rid, iid) => (rid, iid + 1)
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
    majority: uint,
}

impl Instance {
    pub fn new_as_proposer(rid: ReplicaID, iid: InstanceID, value: ~[u8],
        peers: ~[DuplexStream<MessageContent, MessageContent>]) -> Instance {

        Instance{
            identity: Proposer,
            replica_id: rid,
            id: iid,
            value: value,
            state: Null,
            majority: peers.len() / 2 + 1,
            peers: peers,
        }
    }

    pub fn new_as_acceptor(rid: ReplicaID, iid: InstanceID,
        peers: ~[DuplexStream<MessageContent, MessageContent>]) -> Instance {



        Instance{
            identity: Acceptor,
            replica_id: rid,
            id: iid,
            value: ~[],
            state: Null,
            majority: peers.len() / 2 + 1,
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
        let mut seq_id = (0, self.replica_id);
        self.state = Proposed(seq_id, 0);

        loop {
            // Propose
            for peer in self.peers.iter() {
                peer.send(Propose(seq_id))
            }

            // Wait for promises
            'main_loop: loop {
                for peer in self.peers.iter() {
                    match peer.try_recv() {
                        Some(Promise(_seq_id)) if _seq_id == seq_id => {
                            self.state = match self.state {
                                Proposed(seq_id, n) => Proposed(seq_id, n + 1),
                                _ => fail!("malformed proposer state"),
                            }
                        },
                        Some(RejectPropose(_seq_id, higher_id)) if _seq_id == seq_id => {
                            assert!(higher_id > seq_id);
                            seq_id = increment_seq(higher_id);
                            continue 'main_loop;
                        },
                        _ => continue,
                    }
                }
                match self.state {
                    Proposed(seq_id, n) => if n > self.majority { break },
                    _ => fail!("malformed proposer state"),
                }
            }

            for peer in self.peers.iter() {
                // peer.send(Request())
            }
        }
    }

    fn run_as_acceptor(self) {
        loop {
            match self.state {
                Promised(seq_id) => {
                    let (seq, rid) = seq_id;
                    self.peers[rid].send(Promise(seq_id));
                },
                _ => { fail!("malformed acceptor initial state.") }
            };
        }
    }
}