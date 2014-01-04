use std::io::net::ip::SocketAddr;

use extra::json;
use extra::json::{Object, List, Number, String};
use extra::comm::DuplexStream;

use super::connection_handler::ConnectionHandler;
use super::communicator::Communicator;
use super::instance::{Instance, InstanceID};
use super::message::MessageContent;

pub type ReplicaID = uint;

pub struct Replica {
    N: uint,
    id: ReplicaID,
    instance_id: InstanceID,
    address: SocketAddr,
    peer_addrs: ~[SocketAddr],
    peer_chans: ~[SharedChan<(InstanceID, DuplexStream<MessageContent, MessageContent>)>]
}

impl Replica {
    pub fn new<T: Reader>(config: &mut T) -> Replica {
        debug!("Creating replica");

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
        let mut tcp_request_streams = ~[];
        let mut communicators = ~[];
        for (i, address) in addresses.enumerate() {
            if (i != id) {
                let (port, chan) = SharedChan::new();
                let (from_child, to_child) = DuplexStream::new();
                tcp_request_streams.push(from_child);
                chans.push(chan);
                peers.push(address);
                let communicator = Communicator {
                    my_id: id,
                    peer_id: i,
                    tcp_stream: to_child,
                    message_stream_port: port,
                    message_stream_chans: ~[],
                };
                communicators.push(communicator);
            } else {
                my_address = Some(address);
            }
        }

        for mut communicator in communicators.move_iter() {
            communicator.message_stream_chans = chans.clone();
            do spawn { communicator.run() };
        }

        let my_address = my_address.unwrap();

        let conn_handler = ConnectionHandler{ 
            id: id,
            address: my_address.clone(),
            tcp_request_streams: tcp_request_streams,
            peer_addrs: peers.clone()
        };
        do spawn {
            let conn_handler = conn_handler;
            conn_handler.run()
        };

        Replica{
            N: peers.len(),
            id: id,
            instance_id: (id, 0),
            address: my_address,
            peer_addrs: peers,
            peer_chans: chans,
        }
    }

    pub fn submit(&self, value: ~[u8]) {
        let instance = Instance::new_as_proposer(self.id, self.instance_id,
            value, self.peer_chans.clone());
        instance.run();
    }
}