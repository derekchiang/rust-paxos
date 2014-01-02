#[crate_id = "paxos"];

#[comment = "An implementation of the Paxos protocol"];
#[license = "MIT"];
#[crate_type = "lib"];

#[feature(macro_rules)];

extern mod extra;

pub mod cluster;