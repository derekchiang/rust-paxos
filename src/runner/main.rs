extern mod paxos;

use std::path::Path;
use std::io::fs::File;

use paxos::cluster::Replica;

fn main() {
    let path: Path = Path::new("config-example.json");
    let on_error = || fail!("open of {:?} failed", path);
    let mut reader: File = File::open(&path).unwrap_or_else(on_error);
    let r = Replica::new(&mut reader);
}