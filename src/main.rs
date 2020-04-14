use std::io;
use std::net::Ipv4Addr;
use tcp_proto::nic::Interface;
use tcp_proto::tcp::control_message;
use tcp_proto::tcp::tcp;
use tcp_proto::test::{test1, test2, test3};
fn main() -> io::Result<()> {
    env_logger::init();

    let mut tcp_instance = tcp::new(Ipv4Addr::new(192, 168, 0, 1))?.unwrap();

    tcp_instance.control(control_message::Bind(4000 as u16));
    tcp_instance.control(control_message::Bind(5000 as u16));
    tcp_instance.control(control_message::Connect(
        4000,
        Ipv4Addr::new(192, 168, 0, 2),
        5000 as u16,
    ));

    while true {
        let mut buf = [0u8; 1504];
        let mut nbytes = tcp_instance.nic.recv(&mut buf[..])?;
        tcp_instance.action(&buf, nbytes);
    }
    Ok(())
}
