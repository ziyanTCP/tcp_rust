use std::io;
use tcp_proto::tcp::{control_message, tcp};
use tcp_proto::test::{test1, test2, test3};

fn main() -> io::Result<()> {
    env_logger::init();

    let mut nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;
    let mut tcp_instance: tcp = Default::default();
    tcp_instance.control(control_message::Bind(4000 as u16));
    while true {
        let mut buf = [0u8; 1504];
        let mut nbytes = nic.recv(&mut buf[..])?;
        tcp_instance.action(&buf, nbytes, &mut nic);
    }
    Ok(())
}
