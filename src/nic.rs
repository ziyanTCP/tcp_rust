//! # abstraction for our nic. It can send, recieve, support different settings
//!
//! A library for modeling nics
//! extend it to support high performance data plane: layer 2 function, dpdk, netmap, drivers for smart NICs
//! right now we just support tun/tap
use std::fmt::Error;
use std::io;
use std::net::Ipv4Addr;

pub struct Interface {
    pub nic: tun_tap::Iface,
    pub ip: Ipv4Addr,
}

pub struct recv_result {
    data: [u8; 1504],
    nbytes: usize,
}

impl Interface {
    pub fn new(ip: Ipv4Addr) -> io::Result<Self> {
        let nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;
        return Ok(Interface { nic, ip: ip });
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

    /// a wrapper for tun_tap::Iface::send
    pub fn send(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.nic.send(buf)
    }

    /// a wrapper for tun_tap::Iface::recv
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.nic.recv(buf)
    }
}
