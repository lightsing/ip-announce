use std::io::Cursor;

use pnet::datalink::interfaces;
use pnet::ipnetwork::IpNetwork;
use pnet::util::MacAddr;
use socket2::{Socket, Type, Domain, SockAddr};

use serde::{Deserialize, Serialize};
use std::net::{SocketAddrV4, Ipv4Addr};
use std::thread::sleep;
use std::time::Duration;

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
        #[cfg(feature = "compress")]
        let mut compressed = Cursor::new(Vec::new());
        #[cfg(all(feature = "compress", not(any(feature = "lzma2", feature = "xz"))))]
        lzma_rs::lzma_compress(&mut serialized, &mut compressed).unwrap();
        #[cfg(all(feature = "compress", not(any(feature = "lzma", feature = "xz"))))]
        lzma_rs::lzma2_compress(&mut serialized, &mut compressed).unwrap();
        #[cfg(all(feature = "compress", not(any(feature = "lzma", feature = "lzma2"))))]
        lzma_rs::lzma2_compress(&mut serialized, &mut compressed).unwrap();

        #[cfg(not(feature = "compress"))]
        {
            let serialized = serialized.into_inner();
            return serialized;
        }
        #[cfg(feature = "compress")]
        {
            let compressed = compressed.into_inner();
            return compressed;
        }
    }

    fn decode<B: AsRef<[u8]>>(buf: B) -> Result<Self, serde_cbor::Error> {
        let mut data = Cursor::new(buf.as_ref());
        #[cfg(feature = "compress")]
        {
            let mut decompressed = Cursor::new(Vec::new());
            #[cfg(all(feature = "compress", not(any(feature = "lzma2", feature = "xz"))))]
            lzma_rs::lzma_decompress(&mut data, &mut decompressed).unwrap();
            #[cfg(all(feature = "compress", not(any(feature = "lzma", feature = "xz"))))]
            lzma_rs::lzma2_decompress(&mut data, &mut decompressed).unwrap();
            #[cfg(all(feature = "compress", not(any(feature = "lzma", feature = "lzma2"))))]
            lzma_rs::xz_decompress(&mut data, &mut decompressed).unwrap();
            let serialized = decompressed.into_inner();
            let report = serde_cbor::from_slice(&serialized)?;
            return Ok(report);
        }
        #[cfg(not(feature = "compress"))]
        {
            let report = serde_cbor::from_slice(data.into_inner)?;
            return Ok(report);
        }
    }
}

fn main() {
    let socket = Socket::new(Domain::ipv4(), Type::dgram(), None).expect("socket failed");
    socket.set_broadcast(true).expect("set_broadcast failed");
    let report = Report::new();
    let broadcast = SockAddr::from(SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), 58379));
    loop {
        socket.send_to(&report, &broadcast).expect("broadcast failed");
        sleep(Duration::from_secs(1))
    }
}
