use std::io::Cursor;

use pnet::datalink::interfaces;
use pnet::ipnetwork::IpNetwork;
use pnet::util::MacAddr;
use socket2::{Domain, SockAddr, Socket, Type};

use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::thread::sleep;
use std::time::Duration;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("unsupported report format")]
    Unsupported,
    #[error("bad/unrecognized format")]
    BadFormat,
    #[error("bad/unrecognized deserialize format")]
    Deserialize(#[from] serde_cbor::Error),
    #[cfg(feature = "compress")]
    #[error("bad lzma compressed format")]
    BadLzma,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Report {
    hostname: Option<String>,
    interfaces: Vec<Interface>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Interface {
    name: String,
    mac: Option<MacAddr>,
    addrs: Vec<IpNetwork>,
}

impl Report {
    fn new() -> Vec<u8> {
        let hostname = match hostname::get() {
            Ok(oss) => oss.into_string().ok(),
            Err(_) => None,
        };
        let interfaces: Vec<Interface> = interfaces()
            .into_iter()
            .filter(|iface| !iface.is_loopback())
            .map(|iface| Interface {
                name: iface.name,
                mac: iface.mac,
                addrs: iface.ips,
            })
            .filter(|iface| !iface.addrs.is_empty())
            .collect();
        let report = Report {
            hostname,
            interfaces,
        };
        let mut serialized = Cursor::new(serde_cbor::to_vec(&report).expect("cannot serialize"));
        #[cfg(not(feature = "compress"))]
        {
            let serialized = serialized.into_inner();
            return serialized;
        }
        #[cfg(feature = "compress")]
        {
            let mut compressed = Cursor::new(Vec::new());
            lzma_rs::lzma_compress(&mut serialized, &mut compressed).unwrap();
            let compressed = compressed.into_inner();
            return compressed;
        }
    }

    fn decode<B: AsRef<[u8]>>(buf: B) -> Result<Self, DecodeError> {
        let buf = buf.as_ref();
        if buf.len() < 2 {
            return Err(DecodeError::BadFormat);
        }
        let compressed = buf[0] & 0x01 == 0;
        if compressed {
            #[cfg(feature = "compress")]
            {
                let mut decompressed = Cursor::new(Vec::new());
                lzma_rs::lzma_decompress(&mut data, &mut decompressed).unwrap();
                let serialized = decompressed.into_inner();
                let report = serde_cbor::from_slice(&serialized)?;
                Ok(report)
            }
            #[cfg(not(feature = "compress"))]
            Err(DecodeError::Unsupported)
        } else {
            let mut data = Cursor::new(&buf[1..]);
            let report = serde_cbor::from_slice(data.into_inner())?;
            Ok(report)
        }
    }
}

fn main() {
    let socket = Socket::new(Domain::ipv4(), Type::dgram(), None).expect("socket failed");
    socket.set_broadcast(true).expect("set_broadcast failed");
    let report = Report::new();
    let broadcast = SockAddr::from(SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), 58379));
    loop {
        socket
            .send_to(&report, &broadcast)
            .expect("broadcast failed");
        sleep(Duration::from_secs(1))
    }
}
