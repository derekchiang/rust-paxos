use std::vec;
use std::ptr;
use std::io::net::tcp::TcpStream;
use std::io::io_error;
use std::io::mem::BufReader;
use std::io::buffered::BufferedStream;
use std::hashmap::HashMap;
use std::comm::Chan;
use std::sync::arc::UnsafeArc;

use extra::json;
use extra::comm::DuplexStream;
use extra::serialize::{Encodable, Decodable};

use object_stream::ObjectStream;

use super::message::{Message, PaxosMessageContent, Propose, NetworkM, PaxosM, PaxosMessage, NetworkMessage};
use super::replica::ReplicaID;
use super::instance::{Instance, InstanceID};

// Each communicator is responsible for communicating with a specific peer.
// When an instance wants to send a message to a peer, it sends the message
// to the corresponding communicator, which will takes care of all serialization.
// Effectively, an instance views communicators as peers and exchanges messages
// with them.
// A communicator receives TcpStream from a channel.  Under the current scheme,
// a replica is responsible for initiating TCP connections with replicas with
// a lower ID.  Thus, if a communicator detects that it's talking to a lower-ID
// replica and it doesn't have an active connection, it sends a signal through
// stream_chan, thus telling the ConnectionHandler to initiate a new connection.
// Otherwise, if the communicator is responsible for talking with higher-ID replica,
// it simply waits for a connection because the replica will initiate it.
pub struct Communicator {
    my_id: ReplicaID,
    peer_id: ReplicaID,
    tcp_stream: DuplexStream<bool, ObjectStream<BufferedStream<TcpStream>>>,
    message_stream_port: Port<(InstanceID, DuplexStream<PaxosMessageContent, PaxosMessageContent>)>,
    message_stream_chans: ~[SharedChan<(InstanceID, DuplexStream<PaxosMessageContent, PaxosMessageContent>)>],
}

impl Communicator {
    pub fn run(self) {
        debug!("Replica {} is running a communicator for {}",
            self.my_id, self.peer_id);

        let mut stream_map = HashMap::new();

        loop {
            // Get a TCP stream
            let stream_opt = self.tcp_stream.try_recv();
            let mut tcp = if stream_opt.is_none() {
                self.tcp_stream.send(true);
                self.tcp_stream.recv()
            } else {
                stream_opt.unwrap()
            };

            debug!("Replica {}'s communicator for {} received a TCP stream",
                self.my_id, self.peer_id);

            // Send my ID
            // TODO: according to the current design, it's only necessary
            // for replicas with higher IDs to identify themselves to
            // replicas with lower IDs.  However, it our communication is
            // such that extra noise bytes are overlooked, then this should
            // not be a problem.
            let init_msg = NetworkM(NetworkMessage{
                replica_id: self.my_id 
            });
            tcp.send(init_msg);
            
            let (tcp_send_arc, tcp_recv_arc) = UnsafeArc::new2(tcp);
            let (msg_port, msg_chan) = Chan::new();

            do spawn {
                unsafe {
                    let tcp_recv_ptr = tcp_recv_arc.get();
                    loop {
                        match (*tcp_recv_ptr).recv::<Message>() {
                            Ok(msg) => msg_chan.send(msg),
                            Err(_) => {},
                        };
                    }
                }
            }

            let tcp_send_ptr = tcp_send_arc.get();
            loop {
                // Accept new instances
                match self.message_stream_port.try_recv() {
                    Some((iid, stream)) => {
                        stream_map.insert(iid, stream);
                    },
                    None => {},
                }

                // Send new messages
                for (iid, stream) in stream_map.iter() {
                    match stream.try_recv() {
                        Some(content) => {
                            match content {
                                _ => {
                                    let msg = PaxosMessage{
                                        content: content,
                                        instance_id: *iid,
                                    };
                                    unsafe {
                                        (*tcp_send_ptr).send(PaxosM(msg));
                                    }
                                }
                            };
                        },
                        None => {},
                    };
                }

                // Receive new messages
                match msg_port.try_recv() {
                    Some(PaxosM(msg)) => {
                        let stream = stream_map.find(&msg.instance_id);
                        if stream.is_none() {
                            match msg.content {
                                // Only Propose is legal
                                Propose(seq) => {
                                    let mut msg_streams = vec::with_capacity(self.message_stream_chans.len());
                                    let (_, rid) = seq;
                                    for (idx, chan) in self.message_stream_chans.iter().enumerate() {
                                        let (from, to) = DuplexStream::new();
                                        if (idx == rid) {
                                            to.send(msg.content.clone());
                                            chan.send((msg.instance_id.clone(), to));
                                        }
                                        msg_streams.push(from);
                                    }
                                    let instance = Instance::new_as_acceptor(self.my_id, msg.instance_id);
                                    let msg_streams = msg_streams;
                                    do spawn { instance.run(msg_streams); }
                                },
                                _ => {}, // overlook the wrong message
                            }
                        } else {
                            stream.unwrap().try_send(msg.content);
                        }
                    },
                    _ => {},
                };
            };
        }
    }
}