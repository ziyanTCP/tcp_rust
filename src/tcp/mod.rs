pub mod flow;
use flow::Quad;
use flow::State;

mod flow_table;
use flow_table::f_t;
use std::collections::{HashMap, VecDeque};
use std::net::Ipv4Addr;

use std::collections::hash_map::Entry;
use std::collections::HashSet;

#[derive(Default)]
pub struct tcp {
    flow_table: HashMap<Quad, flow::flow>, // the mapping from the Quad to the flow
    listening: HashSet<u16>,               // the mapping from the port
}

pub enum control_message {
    Bind(u16),
    Read,
    Write,
}

impl tcp {
    pub fn action(&mut self, buf: &[u8], nbytes: usize, nic: &mut tun_tap::Iface) {
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
                        let q = Quad {
                            src: (src, tcph.source_port()),
                            dst: (dst, tcph.destination_port()),
                        };
                        let idata = (iph.slice().len() + tcph.slice().len());
                        match self.flow_table.entry(q) {
                            Entry::Occupied(mut f) => {
                                // debug!("got packet for known quad {:?}", q);

                                match f.get_mut().state {
                                    State::SynRcvd => {
                                        f.get_mut().SynRcvd_handler(
                                            nic,
                                            iph,
                                            tcph,
                                            &buf[idata..nbytes],
                                        );
                                    }
                                    State::Estab => {
                                        f.get_mut().Estab_handler(
                                            nic,
                                            iph,
                                            tcph,
                                            &buf[idata..nbytes],
                                        );
                                    }
                                    State::CloseWait => {
                                        f.get_mut().Closed_handler();
                                    }
                                    State::LastAck => {
                                        f.get_mut().LastAck_handler();
                                    }
                                    State::TimeWait => {
                                        f.get_mut().TimeWait_handler();
                                    }

                                    State::FinWait1 => {
                                        f.get_mut().FinWait1_handler();
                                    }
                                    State::FinWait2 => {
                                        f.get_mut().FinWait2_handler();
                                    }
                                    State::Closed => {
                                        f.get_mut().Closed_handler();
                                    }
                                }
                                // let a = f.get_mut().on_packet(
                                //      nic,
                                //     iph,
                                //     tcph,
                                //     &buf[idata..nbytes],
                                // ).unwrap();
                            }
                            Entry::Vacant(e) => {
                                // debug!("got packet for unknown quad {:?}", q);
                                if self.listening.contains(&q.dst.1) {
                                    if let Some(new_f) =
                                        flow::flow::three_way_handshake(nic, iph, tcph).unwrap()
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
