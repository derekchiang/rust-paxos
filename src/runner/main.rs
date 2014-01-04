extern mod paxos;

use std::path::Path;
use std::io::fs::File;

use paxos::internal::replica::Replica;

fn main() {
    for i in range(1, 4) {
        let path: Path = Path::new(format!("config-{}.json", i));
        let on_error = || fail!("open of {:?} failed", path);
        let mut reader: File = File::open(&path).unwrap_or_else(on_error);
        let _ = Replica::new(&mut reader);
    }
}