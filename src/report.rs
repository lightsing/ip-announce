use std::io::Cursor;

use pnet::datalink::interfaces;
use pnet::ipnetwork::IpNetwork;
use pnet::util::MacAddr;

use serde::{Deserialize, Serialize};

use crate::error::{EncodeError, DecodeError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub hostname: Option<String>,
    pub interfaces: Vec<Interface>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interface {
    pub name: String,
    pub mac: Option<MacAddr>,
    pub addrs: Vec<IpNetwork>,
}

impl Report {
    fn new_inner() -> Result<Vec<u8>, EncodeError> {
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
        return Ok(serde_cbor::to_vec(&report)?)
    }

    pub fn new() -> Result<Vec<u8>, EncodeError> {
        let mut serialized = Self::new_inner()?;
        serialized.insert(0, 0);
        return Ok(serialized)
    }

    #[cfg(feature = "compress")]
    pub fn new_compressed() -> Result<Vec<u8>, EncodeError> {
        let mut serialized = Cursor::new(Self::new_inner()?);
        let mut compressed = Cursor::new(Vec::new());
        lzma_rs::lzma_compress(&mut serialized, &mut compressed)?;
        let mut compressed = compressed.into_inner();
        compressed.insert(0, 1);
        return Ok(compressed)
    }

    pub fn decode<B: AsRef<[u8]>>(buf: B) -> Result<Self, DecodeError> {
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
            let data = Cursor::new(&buf[1..]);
            let report = serde_cbor::from_slice(data.into_inner())?;
            Ok(report)
        }
    }
}
