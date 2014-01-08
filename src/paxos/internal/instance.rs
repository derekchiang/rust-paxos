use extra::comm::DuplexStream;

use super::replica::ReplicaID;
use super::message::{Propose, Promise, RejectPropose, Request, Accept,
    RejectRequest, Commit, Acknowledge, PaxosMessageContent, Message};

#[deriving(Clone, TotalOrd, Encodable, Decodable)]
pub type SequenceID = (uint, ReplicaID);

#[deriving(Clone, TotalOrd, Encodable, Decodable)]
pub type InstanceID = (ReplicaID, uint);

type Peers = ~[DuplexStream<PaxosMessageContent, PaxosMessageContent>];
type BorrowedPeers<'a> = &'a [DuplexStream<PaxosMessageContent, PaxosMessageContent>];

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
    Committed(SequenceID, ~[u8], uint),
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

    pub fn run(self, peers: Peers) {
        match self.identity {
            Proposer => self.run_as_proposer(peers),
            Acceptor => self.run_as_acceptor(peers),
        }
    }

    fn run_as_proposer(mut self, peers: Peers) {
        let mut seq = (0, self.replica_id);
        self.propose(seq, peers);

        let get_messages = |peers: BorrowedPeers| {
            peers.iter().fold(~[], |mut messages, peer| {
                match peer.try_recv() {
                    Some(msg) => messages.push(msg),
                    None => (),
                }
                messages
            })
        };

        loop {
            let messages = get_messages(peers);
            for msg in messages.move_iter() {
                match msg {
                    Promise(seq) => self.handle_promise(seq, peers),
                    RejectPropose(s1, s2) => self.handle_reject_propose(s1, s2, peers),
                    Accept(seq) => self.handle_accept(seq, peers),
                    RejectRequest(s1, s2) => self.handle_reject_request(s1, s2, peers),
                    Acknowledge(seq) => self.handle_acknowledge(seq, peers),
                    _ => (),
                };
            }
        }
    }

    fn propose(&mut self, seq: SequenceID, peers: BorrowedPeers) {
        self.state = Proposed(seq, 0);
        for peer in peers.iter() {
            peer.send(Propose(seq));
        }
    }

    fn handle_promise(&mut self, seq: SequenceID, peers: BorrowedPeers) {
        let majority: uint = peers.len() / 2 + 1;
        match self.state {
            Proposed(old_seq, count) => {
                if seq == old_seq {
                    count += 1;
                    if count >= majority {
                        for peer in peers.iter() {
                            peer.send(Request(seq, self.value.clone()));
                        }
                        self.state = Requested(seq, self.value.clone(), 0);
                        return;
                    } else {
                        self.state = Proposed(seq, count);
                    }
                } else if seq > old_seq {
                    self.propose(increment_seq(seq), peers);
                }
            },
            _ => (),
        }
    }

    fn handle_reject_propose(&mut self, s1: SequenceID, s2: SequenceID, peers: BorrowedPeers) {
        match self.state {
            Proposed(old_seq, count) => {
                if s1 == old_seq && s2 > s1 {
                    self.propose(increment_seq(s2), peers);
                }
            },
            _ => (),
        }
    }

    fn handle_accept(&mut self, seq: SequenceID, peers: BorrowedPeers) {
        let majority: uint = peers.len() / 2 + 1;
        match self.state.clone() {
            Requested(old_seq, value, count) => {
                if seq == old_seq {
                    count += 1;
                    if count >= majority {
                        for peer in peers.iter() {
                            peer.send(Commit(seq));
                        }
                        self.state = Committed(seq, self.value.clone(), 0);
                        return;
                    } else {
                        self.state = Requested(old_seq, value, count + 1);
                    }
                } else if seq > old_seq {
                    self.propose(increment_seq(seq), peers);
                }
            },
            _ => (),
        }
    }

    fn handle_reject_request(&mut self, s1: SequenceID, s2: SequenceID, peers: BorrowedPeers) {
        match self.state.clone() {
            Requested(old_seq, _, _) => {
                if s1 == old_seq && s2 > s1 {
                    self.propose(increment_seq(s2), peers);
                }
            },
            _ => (),
        }
    }

    fn handle_acknowledge(&mut self, seq: SequenceID, peers: BorrowedPeers) {
        match self.state.clone() {
            Committed(old_seq, value, count) => {
                if seq == old_seq {
                    self.state = Committed(seq, value, count + 1);
                }
            },
            _ => (),
        }
    }

    fn run_as_acceptor(mut self, mut peers: Peers) {
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
                    self.state = Committed(seq, value.clone(), 0);
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