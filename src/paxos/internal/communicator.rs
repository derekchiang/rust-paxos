use std::vec;
use std::ptr;
use std::io::net::tcp::TcpStream;
use std::io::io_error;
use std::io::buffered::BufferedStream;
use std::io::mem::BufReader;
use std::hashmap::HashMap;

use extra::json;
use extra::comm::DuplexStream;
use extra::serialize::{Encodable, Decodable};

use super::message::{Message, MessageContent, Propose};
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
    tcp_stream: DuplexStream<bool, BufferedStream<TcpStream>>,
    message_stream_port: Port<(InstanceID, DuplexStream<MessageContent, MessageContent>)>,
    message_stream_chans: ~[SharedChan<(InstanceID, DuplexStream<MessageContent, MessageContent>)>],
}

fn split_owned_vec<T>(mut v: ~[T], index: uint) -> (~[T], ~[T]) {
    assert!(index <= v.len());

    let new_len = v.len() - index;
    let mut new_v = vec::with_capacity(v.len() - index);
    unsafe {
        ptr::copy_nonoverlapping_memory(new_v.as_mut_ptr(), v.as_ptr().offset(index as int), new_len);
        v.set_len(index);
        new_v.set_len(new_len);
    }

    (v, new_v)
}

fn parse_msg(bytes: ~[u8]) -> (Option<Message>, ~[u8]) {
    let mut msg_end = -1;
    for (i, _) in bytes.iter().enumerate() {
        // '\r' is 13 and '\n' is 10
        if bytes[i] == 13 && bytes[i + 1] == 10 {
            msg_end = i;
        }
    }

    if msg_end > -1 {
        let (msg_bytes, bytes) = split_owned_vec(bytes, msg_end);

        let mut reader = BufReader::new(msg_bytes);
        let j = match json::from_reader(&mut reader as &mut Reader) {
            Ok(j) => j,
            Err(_) => return (None, bytes),
        };

        let mut decoder = json::Decoder::new(j);
        let msg = Decodable::decode(&mut decoder);

        return (Some(msg), ~[]);
    } else {
        return (None, bytes);
    }
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
            tcp.write_le_uint(self.my_id);

            let mut bytes = ~[];
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
                                // END_INSTANCE => {
                                //     // How do we remove the instances?
                                // },
                                _ => {
                                    let msg = Message{
                                        content: content,
                                        instance_id: *iid,
                                    };
                                    let mut tcp_encoder = json::Encoder::new(&mut tcp as &mut Writer);
                                    msg.encode(&mut tcp_encoder);
                                }
                            };
                        },
                        None => {},
                    };
                }

                // Receive new messages
                bytes = vec::append(bytes, tcp.read_to_end());
                let (msg, b) = parse_msg(bytes);
                bytes = b;
                match msg {
                    Some(msg) => {
                        let stream = stream_map.find(&msg.instance_id);
                        if stream.is_none() {
                            match msg.content {
                                // Only Propose is legal
                                Propose(seq) => {
                                    let mut msg_streams = vec::with_capacity(self.message_stream_chans.len());
                                    for chan in self.message_stream_chans.iter() {
                                        let (from, to) = DuplexStream::new();
                                        to.send(msg.content.clone());
                                        chan.send((msg.instance_id.clone(), to));
                                        msg_streams.push(from);
                                    }
                                    let instance = Instance::new_as_acceptor(self.my_id, msg.instance_id, msg_streams);
                                    do spawn { instance.run(); }
                                },
                                _ => {}, // overlook the wrong message
                            }
                        } else {
                            stream.unwrap().try_send(msg.content);
                        }
                    },
                    None => {},
                };
            };
        }
    }
}