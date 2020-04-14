use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::Ipv4Addr;

use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Quad {
    pub src: (Ipv4Addr, u16),
    pub dst: (Ipv4Addr, u16),
}

#[derive(Debug)]
pub enum State {
    //Listen,
    SynRcvd,
    Estab,
    FinWait1,
    FinWait2,
    TimeWait,
    CloseWait,
    LastAck,
    Closed,
}

/// State of the Send Sequence Space (RFC 793 S3.2 F4)
///
/// ```
///            1         2          3          4
///       ----------|----------|----------|----------
///              SND.UNA    SND.NXT    SND.UNA
///                                   +SND.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers of unacknowledged data
/// 3 - sequence numbers allowed for new data transmission
/// 4 - future sequence numbers which are not yet allowed
/// ```
pub struct SendSequenceSpace {
    /// send unacknowledged
    una: u32,
    /// send next
    nxt: u32,
    /// send window
    wnd: u16,
    /// send urgent pointer
    up: bool,
    /// segment sequence number used for last window update
    wl1: usize,
    /// segment acknowledgment number used for last window update
    wl2: usize,
    /// initial send sequence number
    iss: u32,
}

/// State of the Receive Sequence Space (RFC 793 S3.2 F5)
///
/// ```
///                1          2          3
///            ----------|----------|----------
///                   RCV.NXT    RCV.NXT
///                             +RCV.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers allowed for new reception
/// 3 - future sequence numbers which are not yet allowed
/// ```
pub struct RecvSequenceSpace {
    /// receive next
    nxt: u32,
    /// receive window
    wnd: u16,
    /// receive urgent pointer
    up: bool,
    /// initial receive sequence number
    irs: u32,
}


pub struct flow {
    pub quad: Quad,
    pub state: State,
    pub send: SendSequenceSpace,
    pub recv: RecvSequenceSpace,

    ip: etherparse::Ipv4Header,
    tcp: etherparse::TcpHeader,

    // pub stats: Statistics,
    pub(crate) incoming: VecDeque<u8>,
    pub(crate) unacked: VecDeque<u8>,
}

impl flow {
    pub fn three_way_handshake<'a>(
        nic: &mut tun_tap::Iface, // why mutable?
        iph: etherparse::Ipv4HeaderSlice<'a>,
        tcph: etherparse::TcpHeaderSlice<'a>,
    ) -> io::Result<Option<Self>> {
        let buf = [0u8; 1500];
        if !tcph.syn() {
            // only expected SYN packet
            return Ok(None);
        }

        let iss = 0;
        let wnd = 64240; // same as the window size of cat

        let mut f = flow {
            quad: Quad {
                src: (iph.source_addr(), tcph.source_port()),
                dst: (iph.destination_addr(), tcph.destination_port()),
            },

            state: State::SynRcvd,
            send: SendSequenceSpace {
                una: iss,
                nxt: iss,
                wnd: wnd,
                up: false,
                wl1: 0,
                wl2: 0,
                iss: iss,
            },
            recv: RecvSequenceSpace {
                irs: tcph.sequence_number(),
                nxt: tcph.sequence_number() + 1,
                wnd: tcph.window_size(),
                up: false,
            },
            incoming: Default::default(),
            unacked: Default::default(),
            tcp: etherparse::TcpHeader::new(tcph.destination_port(), tcph.source_port(), iss, wnd),
            ip: etherparse::Ipv4Header::new(
                0,
                64,
                etherparse::IpTrafficClass::Tcp,
                [
                    iph.destination()[0],
                    iph.destination()[1],
                    iph.destination()[2],
                    iph.destination()[3],
                ],
                [
                    iph.source()[0],
                    iph.source()[1],
                    iph.source()[2],
                    iph.source()[3],
                ],
            ),
        };
        // need to start establishing a connection
        f.tcp.syn = true;
        f.tcp.ack = true;
        f.write(nic, f.send.nxt, 0)?;
        Ok(Some(f))
    }

    pub fn write(
        &mut self,
        nic: &mut tun_tap::Iface,
        seq: u32,
        mut limit: usize,
    ) -> io::Result<usize> {
        let mut buf = [0u8; 1500];
        self.tcp.sequence_number = seq;
        self.tcp.acknowledgment_number = self.recv.nxt;
        let size = std::cmp::min(
            buf.len(),
            self.tcp.header_len() as usize + self.ip.header_len() as usize,
        );

        self.ip
            .set_payload_len(size - self.ip.header_len() as usize);

        // write out the headers and the payload
        use std::io::Write;
        let buf_len = buf.len();
        let mut unwritten = &mut buf[..];

        self.ip.write(&mut unwritten);
        let ip_header_ends_at = buf_len - unwritten.len();

        // postpone writing the tcp header because we need the payload as one contiguous slice to calculate the tcp checksum
        unwritten = &mut unwritten[self.tcp.header_len() as usize..];
        let tcp_header_ends_at = buf_len - unwritten.len();

        // TODO: write out the payload

        // calculate the checksum
        self.tcp.checksum = self
            .tcp
            .calc_checksum_ipv4(&self.ip, &[])
            .expect("failed to compute checksum");

        let mut tcp_header_buf = &mut buf[ip_header_ends_at..tcp_header_ends_at];

        self.tcp.write(&mut tcp_header_buf);

        if self.tcp.syn {
            self.send.nxt = self.send.nxt.wrapping_add(1);
            self.tcp.syn = false;
        }
        if self.tcp.fin {
            self.send.nxt = self.send.nxt.wrapping_add(1);
            self.tcp.fin = false;
        }

        //debug!("{:?}",&buf[..tcp_header_ends_at]);
        nic.send(&buf[..tcp_header_ends_at])?;

        Ok(0 as usize)
    }

    /// State::Estab | State::FinWait1 | State::FinWait2
    /// read data, put into buffer
    pub fn data_from_segment(
        &mut self,
        data: &[u8],
        tcph: &etherparse::TcpHeaderSlice,
    ) -> io::Result<u64> {
        let seqn = tcph.sequence_number();

        let mut unread_data_at = (self.recv.nxt - seqn) as usize;
        if unread_data_at > data.len() {
            // ?
            // we must have received a re-transmitted FIN that we have already seen
            // nxt points to beyond the fin, but the fin is not in data!
            assert_eq!(unread_data_at, data.len() + 1);
            unread_data_at = 0;
        }
        self.incoming.extend(&data[unread_data_at..]);
        //self.debug_print_buffer();
        self.recv.nxt = seqn
            .wrapping_add(data.len() as u32)
            .wrapping_add(if tcph.fin() { 1 } else { 0 });
        return Ok(0 as u64);
    }

    /// Segment Receive  Test: called by ESTABLISH
    /// slen: the virtual data len, counting syn or fin
    ///     Length  Window
    ///     ------- -------  -------------------------------------------
    ///
    ///        0       0     SEG.SEQ = RCV.NXT
    ///
    ///        0      >0     RCV.NXT =< SEG.SEQ < RCV.NXT+RCV.WND
    ///
    ///       >0       0     not acceptable
    ///
    ///       >0      >0     RCV.NXT =< SEG.SEQ < RCV.NXT+RCV.WND
    ///                   or RCV.NXT =< SEG.SEQ+SEG.LEN-1 < RCV.NXT+RCV.WND
    ///
    ///         let mut slen = data.len() as u32;
    ///         if tcph.fin() {
    ///             slen += 1;
    ///         };
    ///         if tcph.syn() {
    ///             slen += 1;
    ///         };
    pub fn segment_check(&mut self, slen: u32, seqn: u32) -> bool {
        let wend = self.recv.nxt.wrapping_add(self.recv.wnd as u32);
        let okay = if slen == 0 {
            // zero-length segment has separate rules for acceptance
            if self.recv.wnd == 0 {
                if seqn != self.recv.nxt {
                    false
                } else {
                    true
                }
            } else if !is_between_wrapped(self.recv.nxt.wrapping_sub(1), seqn, wend) {
                false
            } else {
                true
            }
        } else {
            if self.recv.wnd == 0 {
                false
            } else if !is_between_wrapped(self.recv.nxt.wrapping_sub(1), seqn, wend)
                && !is_between_wrapped(
                    self.recv.nxt.wrapping_sub(1),
                    seqn.wrapping_add(slen - 1),
                    wend,
                )
            {
                false
            } else {
                true
            }
        };
        return okay;
    }

    pub fn SynRcvd_handler(
        &mut self,
        nic: &mut tun_tap::Iface,
        tcph: etherparse::TcpHeaderSlice,
        data: &[u8],
    ) -> io::Result<u64> {
        debug!("SynRcvd_handler called");

        let seqn = tcph.sequence_number();
        let ackn = tcph.acknowledgment_number();

        // the segement length is data length plus 1 (SYN)
        let ok = self.segment_check((data.len() + 1) as u32, seqn);
        if ok == false {
            return Ok(0 as u64);
        }

        // whether ack our previous ack
        if is_between_wrapped(
            self.send.una.wrapping_sub(1),
            ackn,
            self.send.nxt.wrapping_add(1),
        ) {
            // must have ACKed our SYN, since we detected at least one acked byte,
            // and we have only sent one byte (the SYN).
            debug!("connection established!");
            self.state = State::Estab;
        } else {
            // TODO: <SEQ=SEG.ACK><CTL=RST>
            return Ok(0 as u64);
        }

        self.data_from_segment(data, &tcph);

        // no need to ack if there is no data
        if data.len() != 0 {
            self.write(nic, self.send.nxt, 0)?;
        }
        return Ok(0 as u64);
    }

    pub fn Estab_handler(
        &mut self,
        nic: &mut tun_tap::Iface,
        tcph: etherparse::TcpHeaderSlice,
        data: &[u8],
    ) -> io::Result<u64> {
        //debug!("Estab_handler called");

        let seqn = tcph.sequence_number();
        let ackn = tcph.acknowledgment_number();

        let mut unread_data_at = (self.recv.nxt - seqn) as usize;
        if unread_data_at > data.len() {
            // ?
            // we must have received a re-transmitted FIN that we have already seen
            // nxt points to beyond the fin, but the fin is not in data!
            assert_eq!(unread_data_at, data.len() + 1);
            unread_data_at = 0;
        }
        if let ok = self.segment_check((data.len() + 1) as u32, seqn) {
            match (ok) {
                true => {
                    self.data_from_segment(data, &tcph);

                    self.write(nic, self.send.nxt, 0)?;

                    if tcph.fin() {
                        self.state = State::CloseWait;
                        // sending an FIN immediately
                        self.tcp.fin = true;
                        self.write(nic, self.send.nxt, 0)?;
                        self.state = State::LastAck;
                    }
                    return Ok(0 as u64);
                }
                false => {
                    return Ok(0 as u64);
                }
            }
        }
        return Ok(0 as u64);
    }

    pub fn FinWait1_handler(&mut self) {
        debug!("FinWait1 called");
    }
    pub fn FinWait2_handler(&mut self) {
        debug!("FinWait2 called");
    }
    pub fn TimeWait_handler(&mut self) {
        debug!("TimeWait called");
    }

    pub fn CloseWait_handler(&mut self) {
        debug!("CloseWait called");
    }

    pub fn LastAck_handler(
        &mut self,
        nic: &mut tun_tap::Iface,
        tcph: etherparse::TcpHeaderSlice,
    ) -> io::Result<u64> {
        debug!("LastAck called");
        let seqn = tcph.sequence_number();
        let ackn = tcph.acknowledgment_number();

        // the segement length is data length plus 1 (FIN)
        let ok = self.segment_check(1 as u32, seqn);
        if ok == false {
            return Ok(0 as u64);
        }

        // whether ack our previous ack
        if is_between_wrapped(
            self.send.una.wrapping_sub(1),
            ackn,
            self.send.nxt.wrapping_add(1),
        ) {
            // must have ACKed our SYN, since we detected at least one acked byte,
            // and we have only sent one byte (the SYN).
            debug!("connection terminated!");
            self.debug_print_buffer();
            self.state = State::Closed;
        } else {
            // TODO: <SEQ=SEG.ACK><CTL=RST>
            return Ok(0 as u64);
        }

        // how to destroy the connection in memory
        return Ok(0 as u64);
    }

    pub fn Closed_handler(&mut self) {
        debug!("LastAck called");
    }


    pub fn debug_print_buffer(&mut self) {
        // print_data_in_the_buffer
        let temp_directory = env::temp_dir();
        let temp_file = temp_directory.join("test.mp4");
        let mut file = File::create(temp_file).unwrap();
        file.write(Vec::from(self.incoming.clone()).as_ref())
            .unwrap();
        //info!("data in the buffer (self.incoming) {:?}", test);
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
