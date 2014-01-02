use std::io::Reader;
use std::io::Writer;
use std::io::net::tcp::{TcpStream, TcpListener};
use std::io::net::ip::SocketAddr;
use std::io::io_error;
use std::io::{ConnectionRefused};
use std::io::{Listener, Acceptor};
use std::comm::SharedChan;
use std::hashmap::HashMap;
use std::io::mem::BufReader;
use extra::json;
use extra::json::{Object, List, Number, String};
use extra::comm::DuplexStream;
use extra::serialize::{Encodable, Decodable};

use std::{vec, ptr};
 
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

#[deriving(Encodable, Decodable)]
struct Message {
    from: ReplicaID,
    to: ReplicaID,
    content: MessageContent,
}

#[deriving(Encodable, Decodable)]
enum MessageContent {
    Propose,
}

pub type ReplicaID = uint;

pub struct Replica {
    N: uint,
    id: uint,
    address: SocketAddr,
    peer_addrs: ~[SocketAddr],
    peer_chans: ~[SharedChan<DuplexStream<Message, Message>>]
}

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
// struct Communicator {
//     my_id: ReplicaID,
//     peer_id: ReplicaID,
//     tcp_stream: DeplexStream<bool, TcpStream>,
//     message_stream_port: Port<DuplexStream<Message, Message>,
// }

// impl Communicator {
//     fn run(self) {
        
//     }
// }

fn communicator(tcp_chan: DuplexStream<> port: Port<DuplexStream<Message, Message>>) {
    let mut stream_map = HashMap::new();

    loop {
        let mut tcp = match io_error::cond.trap(|err| {
            match err.kind {
                ConnectionRefused => {},
                _ => fail!(err.desc)
            }
        }).inside(|| {
            TcpStream::connect(peer)
        }) {
            Some(tcp) => tcp,
            None => continue,
        };

        let mut bytes = ~[];

        loop {
            match port.try_recv() {
                Some((rid, stream)) => {
                    stream_map.insert(rid, stream);
                },
                None => {},
            }

            for (_, stream) in stream_map.iter() {
                match stream.try_recv() {
                    Some(msg) => {
                        let mut tcp_encoder = json::Encoder::new(&mut tcp as &mut Writer);
                        msg.encode(&mut tcp_encoder);
                    },
                    None => {},
                }
            }

            bytes = vec::append(bytes, tcp.read_to_end());
            let (msg, b) = parse_msg(bytes);
            bytes = b;
            match msg {
                Some(msg) => {
                    let stream = stream_map.get(&(msg.to));
                    stream.try_send(msg);
                },
                None => {},
            }
        }
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
}

impl Replica {
    pub fn new<T: Reader>(config: &mut T) -> Replica {
        macro_rules! take_or_fail(($val:expr, $ok:pat => $out:expr) => {
            match $val {
                $ok => $out,
                _ => fail!(~"malformed config")
            }
        })

        let content = take_or_fail!(json::from_reader(config as &mut Reader), Ok(c) => c);
        let mut obj = take_or_fail!(content, Object(obj) => obj);
        let id = take_or_fail!(take_or_fail!(obj.pop(&~"id"), Some(t) => t), Number(n) => n as uint);

        let addresses = take_or_fail!(take_or_fail!(obj.pop(&~"peers"), Some(t) => t), List(lst) => {
            assert!(id < lst.len());
            lst.move_iter().map(|p| {
                let s = take_or_fail!(p, String(s) => s);
                match from_str::<SocketAddr>(s) {
                    Some(a) => a,
                    None => fail!(~"malformed peer address")
                }
            })
        });

        let mut peers = ~[];
        let mut chans = ~[];
        let mut my_address = None;
        for (i, address) in addresses.enumerate() {
            if (i != id) {
                let (port, chan) = SharedChan::new();
                chans.push(chan);
                peers.push(address);
                let address = address.clone();
                do spawn {
                    communicator(address, port);
                }
            } else {
                my_address = Some(address);
            }
        }

        let my_address = my_address.unwrap();

        // Accept connections
        do spawn {
            let mut acceptor = match TcpListener::bind(my_address).listen() {
                Some(acceptor) => acceptor,
                None => {
                    error!("bind or listen failed :-(");
                    return;
                },
            };
            let stream = acceptor.accept().unwrap();
            let (p, c) = SharedChan::new();
            c.send(stream);
            loop {
                // let stream = acceptor.acc
            }
        }

        Replica{
            N: peers.len(),
            id: id,
            address: my_address,
            peer_addrs: peers,
            peer_chans: chans,
        }
    }
}