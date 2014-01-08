extern mod paxos;

use std::path::Path;
use std::io::fs::File;
use std::io::timer::sleep;

use paxos::internal::replica::Replica;

fn main() {
    let mut replicas = ~[];
    for i in range(1, 4) {
        let path: Path = Path::new(format!("config-{}.json", i));
        let on_error = || fail!("open of {:?} failed", path);
        let mut reader: File = File::open(&path).unwrap_or_else(on_error);
        replicas.push(Replica::new(&mut reader));
    }

    let mut r = replicas.pop();
    r.submit(~[0u8, 1u8, 2u8]);

    sleep(10000);
}