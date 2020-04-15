pub mod flow;

use std::collections::{HashMap, VecDeque};
use std::net::Ipv4Addr;

use crate::nic;
use crate::nic::Interface;
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::io;

pub struct tcp {
    flow_table: HashMap<flow::Quad, flow::flow>, // the mapping from the Quad to the flow
    listening: HashSet<u16>,                     // the mapping from the port
    pub nic: nic::Interface,
}

pub enum control_message {
    Bind(u16),
    Connect(u16, Ipv4Addr, u16),
    Read,
    Write,
}

impl tcp {
    pub fn new(ip: Ipv4Addr) -> io::Result<Option<Self>> {
        let mut tcp_instance = tcp {
            flow_table: Default::default(),
            listening: Default::default(),
            nic: Interface::new(ip)?,
        };
        Ok(Some(tcp_instance))
    }
    pub fn action(&mut self, buf: &[u8], nbytes: usize) {
        // is it a good choice to leave nic here?
        match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
            Ok(iph) => {
                // println!("An ip packet!");
                let src = iph.source_addr();
                let dst = iph.destination_addr();

                if iph.protocol() != 0x06 {
                    //debug!("Not TCP");
                    return;
                }
                match etherparse::TcpHeaderSlice::from_slice(&buf[iph.slice().len()..nbytes]) {
                    Ok(tcph) => {
                        let q = flow::Quad {
                            src: (src, tcph.source_port()),
                            dst: (dst, tcph.destination_port()),
                        };
                        let idata = (iph.slice().len() + tcph.slice().len());
                        match self.flow_table.entry(q) {
                            Entry::Occupied(mut f) => {
                                // debug!("got packet for known quad {:?}", q);
                                match f.get_mut().state {
                                    flow::State::SynRcvd => {
                                        f.get_mut().SynRcvd_handler(
                                            &mut self.nic,
                                            tcph,
                                            &buf[idata..nbytes],
                                        );
                                    }
                                    flow::State::Estab => {
                                        f.get_mut().Estab_handler(
                                            &mut self.nic,
                                            tcph,
                                            &buf[idata..nbytes],
                                        );
                                    }
                                    flow::State::CloseWait => {
                                        f.get_mut().Closed_handler();
                                    }
                                    flow::State::LastAck => {
                                        f.get_mut().LastAck_handler(&mut self.nic, tcph);
                                    }
                                    flow::State::TimeWait => {
                                        f.get_mut().TimeWait_handler();
                                    }

                                    flow::State::FinWait1 => {
                                        f.get_mut().FinWait1_handler();
                                    }
                                    flow::State::FinWait2 => {
                                        f.get_mut().FinWait2_handler();
                                    }
                                    flow::State::Closed => {
                                        f.get_mut().Closed_handler();
                                    }
                                    flow::State::SynSent => {
                                        f.get_mut().SynSent_handler(&mut self.nic, tcph);
                                    }
                                }
                            }
                            Entry::Vacant(e) => {
                                // debug!("got packet for unknown quad {:?}", q);
                                if self.listening.contains(&q.dst.1) {
                                    if let Some(new_f) = flow::flow::passive_three_way_handshake(
                                        &mut self.nic,
                                        iph,
                                        tcph,
                                    )
                                    .unwrap()
                                    {
                                        e.insert(new_f);
                                        return;
                                    } else {
                                    };
                                } else {
                                    //debug!("not listening, so dropping ...");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        //debug!("ignoring weird tcp packet {:?}", e);
                    }
                }
            }
            Err(e) => {
                //debug!("ignoring weird tcp packet {:?}", e);
            }
        }
    }
    pub fn control(&mut self, message: control_message) {
        match message {
            control_message::Bind(port) => {
                self.listening.insert(port);
                debug!("bind port number {}", port)
            }
            control_message::Connect(src_port, dst_ip, dst_port) => {
                let q = flow::Quad {
                    dst: (self.nic.ip, src_port),
                    src: (dst_ip, dst_port),
                };
                match self.flow_table.entry(q) {
                    Entry::Occupied(mut f) => {
                        debug!("already have the flow");
                    }
                    Entry::Vacant(e) => {
                        // create a flow
                        if let Some(new_f) =
                            flow::flow::active_three_way_handshake(&mut self.nic, &q).unwrap()
                        {
                            e.insert(new_f);
                            return;
                        } else {
                        };
                    }
                }
            }
            control_message::Read => unimplemented!(),
            control_message::Write => unimplemented!(),
        }
    }
}

fn wrapping_lt(lhs: u32, rhs: u32) -> bool {
    // From RFC1323:
    //     TCP determines if a data segment is "old" or "new" by testing
    //     whether its sequence number is within 2**31 bytes of the left edge
    //     of the window, and if it is not, discarding the data as "old".  To
    //     insure that new data is never mistakenly considered old and vice-
    //     versa, the left edge of the sender's window has to be at most
    //     2**31 away from the right edge of the receiver's window.
    lhs.wrapping_sub(rhs) > (1 << 31)
}

fn is_between_wrapped(start: u32, x: u32, end: u32) -> bool {
    wrapping_lt(start, x) && wrapping_lt(x, end)
}
