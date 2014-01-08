#[crate_id = "paxos"];

#[comment = "An implementation of the Paxos protocol"];
#[license = "MIT"];
#[crate_type = "lib"];

#[feature(macro_rules)];
// #[feature(globs)];

extern mod extra;
extern mod object_stream;

pub mod internal;

// use internal::replica::Replica;
// use internal::instance::{InstanceID, Instance};


// pub struct Cluster {
//     replicas: ~[Replica]
// }

// pub fn new_cluster<T: Reader>(addrs: ~[&mut T]) {
//     addrs.move_iter().map(|addr| {
//         Replica::new(addr)
//     }).collect()
// }

// impl Cluster {
//     pub fn submit(value: ~[u8]) {

//     }
// }