use std::io::Listener;
use std::io::Acceptor;
use std::io::buffered::BufferedStream;
use std::io::net::ip::SocketAddr;
use std::io::net::tcp::{TcpStream, TcpListener};

use extra::comm::DuplexStream;
use object_stream::ObjectStream;

use super::replica::ReplicaID;
use super::message::{Message, NetworkM};

pub struct ConnectionHandler {
    id: ReplicaID,
    address: SocketAddr,
    tcp_request_streams: ~[DuplexStream<ObjectStream<BufferedStream<TcpStream>>, bool>],
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
                                    stream.send(ObjectStream::new(BufferedStream::new(tcp)));
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
                let stream = acceptor.accept().unwrap();
                let mut stream = ObjectStream::new(BufferedStream::new(stream));
                match stream.recv::<Message>() {
                    Ok(NetworkM(msg)) => {
                        // Read the replica's ID
                        let id = msg.replica_id;
                        debug!("Got id from replica {}", id);
                        self.tcp_request_streams[id - self.id].send(stream);
                    },
                    Ok(msg) => {
                        debug!("Got wrong initial message {}", msg.to_str());
                    },
                    Err(err) => {
                        debug!("{}", err);
                    }
                };
            }
        }
    }
}