//! # nic
//!
//! A library for modeling nics
//!
use std::io;

pub struct Interface {
    pub nic: tun_tap::Iface,
}

pub struct recv_result {
    data: [u8; 1504],
    nbytes: usize,
}

impl Interface {
    pub fn new() -> io::Result<Self> {
        let nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;
        return Ok(Interface { nic });
    }

    /// Recieve one packet
    /// # Example
    /// ```
    /// use std::io;
    /// mod nic;
    /// fn main()-> io::Result<()> {
    ///     let mut interface = nic::Interface::new()?;
    ///     while true{
    ///         let mut buf,nbytes = interface.recieve()?;
    ///         println!("{} bytes:{:?}",nbytes,&buf[..]);
    ///     }
    ///     Ok(())
    /// }
    ///
    /// ```
    // pub fn recieve(&mut self) -> io::Result(recv_result){
    //     let mut buf = [0u8; 1504];
    //
    //     let nbytes = self.nic.recv(&mut buf[..])?;
    //
    //     Ok(a)
    // }

    pub fn send(&mut self) -> io::Result<()> {
        unimplemented!()
    }
}
