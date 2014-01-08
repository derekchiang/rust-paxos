use extra::comm::DuplexStream;

use super::replica::ReplicaID;
use super::message::{Propose, Promise, RejectPropose, Request, Accept,
    RejectRequest, Commit, Acknowledge, PaxosMessageContent, Message};

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

#[deriving(Clone)]
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
}

impl Instance {
    pub fn new_as_proposer(rid: ReplicaID, iid: InstanceID, value: ~[u8]) -> Instance {
        Instance{
            identity: Proposer,
            replica_id: rid,
            id: iid,
            value: value,
            state: Null,
        }
    }

    pub fn new_as_acceptor(rid: ReplicaID, iid: InstanceID) -> Instance {

        Instance{
            identity: Acceptor,
            replica_id: rid,
            id: iid,
            value: ~[],
            state: Null,
        }
    }

    pub fn run(self, peers: ~[DuplexStream<PaxosMessageContent, PaxosMessageContent>]) {
        match self.identity {
            Proposer => self.run_as_proposer(peers),
            Acceptor => self.run_as_acceptor(peers),
        }
    }

    fn run_as_proposer(mut self, peers: ~[DuplexStream<PaxosMessageContent, PaxosMessageContent>]) {
        let mut seq_id = (0, self.replica_id);
        self.state = Proposed(seq_id, 0);

        let majority = peers.len() / 2 + 1;

        loop {
            // Propose
            for peer in peers.iter() {
                println("BP3");
                peer.send(Propose(seq_id));
                println("BP4");
            }

            // Wait for promises
            'main_loop: loop {
                for peer in peers.iter() {
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
                    Proposed(seq_id, n) => if n > majority { break },
                    _ => fail!("malformed proposer state"),
                }
            }

            for peer in peers.iter() {
                // peer.send(Request())
            }
        }
    }

    fn run_as_acceptor(mut self, mut peers: ~[DuplexStream<PaxosMessageContent, PaxosMessageContent>]) {
        loop {
            for peer in peers.mut_iter() {
                let reply = match peer.try_recv() {
                    Some(Propose(seq)) => self.handle_propose(seq),
                    Some(Request(seq, value)) => self.handle_request(seq, value),
                    Some(Commit(seq)) => self.handle_commit(seq),
                    _ => None,
                };
                match reply {
                    Some(msg) => peer.send(msg),
                    None => (),
                };
            }
        }
    }

    fn handle_propose(&mut self, seq: SequenceID) -> Option<PaxosMessageContent> {
        return match self.state {
            Null => {
                self.state = Promised(seq);
                Some(Promise(seq))
            },
            Promised(old_seq) => {
                if seq >= old_seq {
                    self.state = Promised(seq);
                    Some(Promise(seq))
                } else {
                    Some(RejectPropose(seq, old_seq))
                }
            },
            Accepted(old_seq, _) => {
                if seq >= old_seq {
                    self.state = Promised(seq);
                    Some(Promise(seq))
                } else {
                    Some(RejectPropose(seq, old_seq))
                }
            },
            Committed(..) => None,
            _ => fail!("Illegal state"),
        }
    }

    fn handle_request(&mut self, seq: SequenceID, value: ~[u8]) -> Option<PaxosMessageContent> {
        return match self.state {
            Null => None,
            Promised(old_seq) => {
                if seq == old_seq {
                    self.state = Accepted(seq, value);
                    Some(Accept(seq))
                } else {
                    Some(RejectRequest(seq, old_seq))
                }
            },
            Accepted(old_seq, _) => {
                if seq == old_seq {
                    None
                } else {
                    Some(RejectRequest(seq, old_seq))
                }
            },
            Committed(..) => None,
            _ => fail!("Illegal state"),
        }
    }

    fn handle_commit(&mut self, seq: SequenceID) -> Option<PaxosMessageContent> {
        return match self.state.clone() {
            Null => None,
            Promised(..) => None,
            Accepted(old_seq, value) => {
                if old_seq == seq {
                    self.state = Committed(seq, value.clone());
                    self.commit(seq, value);
                    Some(Acknowledge(seq))
                } else {
                    None
                }
            },
            Committed(..) => None,
            _ => fail!("Illegal state"),
        }
    }

    fn commit(&self, seq: SequenceID, value: ~[u8]) {
        // TODO
    }
}