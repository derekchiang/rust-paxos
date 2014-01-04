use std::io::Listener;
use std::io::Acceptor;
use std::io::net::ip::SocketAddr;
use std::io::net::tcp::{TcpStream, TcpListener};
use std::io::buffered::BufferedStream;

use extra::comm::DuplexStream;

use super::replica::ReplicaID;

pub struct ConnectionHandler {
    id: ReplicaID,
    address: SocketAddr,
    tcp_request_streams: ~[DuplexStream<BufferedStream<TcpStream>, bool>],
    peer_addrs: ~[SocketAddr],
}

impl ConnectionHandler {
    pub fn run(mut self) {
        debug!("Running a connection handler for replica {}", self.id)
        loop {
            // Initiate connections
            for i in range(0, self.id) {
                let stream = self.tcp_request_streams.remove(0);
                let peer_addrs = self.peer_addrs.clone();
                do spawn {
                    loop {
                        stream.recv();
                        loop {
                            match TcpStream::connect(peer_addrs[i]) {
                                None => continue,
                                Some(tcp) => {
                                    stream.send(BufferedStream::new(tcp));
                                    return;
                                },
                            };
                        }
                    }
                }
            }

            // Accept connections
            let mut acceptor = match TcpListener::bind(self.address).listen() {
                Some(acceptor) => acceptor,
                None => {
                    error!("bind or listen failed :-(");
                    return;
                },
            };

            loop {
                let mut stream = acceptor.accept().unwrap();
                // Read the replica's ID
                let id = stream.read_le_uint();
                debug!("Got id from replica {}", id);
                self.tcp_request_streams[id - self.id].send(BufferedStream::new(stream));
            }
        }
    }
}