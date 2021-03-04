use std::net::{Ipv4Addr, SocketAddrV4};
use std::thread::sleep;
use std::time::Duration;

use socket2::{Domain, SockAddr, Socket, Type};

mod error;
mod report;

use crate::report::Report;

fn main() -> anyhow::Result<()> {
    let socket = Socket::new(Domain::ipv4(), Type::dgram(), None).expect("socket failed");
    socket.set_broadcast(true).expect("set_broadcast failed");
    let report = Report::new()?;
    let broadcast = SockAddr::from(SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), 58379));
    loop {
        socket
            .send_to(&report, &broadcast)
            .expect("broadcast failed");
        sleep(Duration::from_secs(1))
    }
}
