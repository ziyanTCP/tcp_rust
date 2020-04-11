use std::io;

/// basic receiving function for the NIC
/// # Purpose
/// # Expected
/// hahhaha
pub fn test1()->io::Result<()>{
    let nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;

    while true{
    let mut buf = [0u8; 1504];
    let mut nbytes = nic.recv(&mut buf[..])?;
    println!("{} bytes:{:?}",nbytes,&buf[..nbytes]);
    }
    Ok(())
}

pub fn test2()->io::Result<()>{
    let nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;

    while true{
        let mut buf = [0u8; 1504];
        let mut nbytes = nic.recv(&mut buf[..])?;
        //println!("{} bytes:{:?}",nbytes,&buf[..nbytes]);

        match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]){
            Ok(iph)=>{
                println!("An ip packet!");

            }
            Err(e) => {
                eprintln!("ignoring weird tcp packet {:?}", e);
            }
        }

    }
    Ok(())
}

pub fn test3()->io::Result<()>{
    let nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;

    while true{
        let mut buf = [0u8; 1504];
        let mut nbytes = nic.recv(&mut buf[..])?;
        //println!("{} bytes:{:?}",nbytes,&buf[..nbytes]);

        match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]){
            Ok(iph)=>{
                // println!("An ip packet!");
                let src = iph.source_addr();
                let dst = iph.destination_addr();
                if iph.protocol() != 0x06 {
                    eprintln!("Not TCP");
                    continue;
                }
                else{
                    eprintln!("TCP");
                }
            }
            Err(e) => {
                eprintln!("ignoring weird tcp packet {:?}", e);
            }
        }
    }
    Ok(())
}