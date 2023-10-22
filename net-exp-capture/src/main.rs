use std::env::args;
use std::process::exit;
use pcap::{Capture, Device};

fn main() {
    let devices = Device::list()
        .expect("Error listing devices");

    let device = if let Some(val) = args().nth(1) { val } else {
        println!("Available devices:");
        for device in &devices {
            let flags = &device.flags;
            print!("{}", if flags.is_up() { "UP " } else { "   " });
            print!("{}", if flags.is_running() { "R " } else { "  " });
            print!("{}", if flags.is_loopback() { "LB " } else { "   " });
            print!("{}", if flags.is_wireless() { "WL " } else { "   " });
            print!("{}", device.name);
            println!();
        }
        return;
    };

    ctrlc::set_handler(|| {
        exit(0);
    }).unwrap();

    let device = devices.into_iter().find(|x| x.name == device)
        .expect("Can't find specified device!");
    let cap = Capture::from_device(device).unwrap()
        .immediate_mode(true);
    let mut cap = cap.open()
        .expect("Can't open packet capture!");

    loop {
        let packet = cap.next_packet()
            .expect("Error receiving next packet");
        let pk_len = packet.header.len;
        let pk_cap_len = packet.header.caplen;
        let pk_cap_time = packet.header.ts;
        let cap_time_sec = pk_cap_time.tv_sec;
        let cap_time_usec = pk_cap_time.tv_usec;
        println!("PACKET AT {cap_time_sec} S, {cap_time_usec} U, \
        LENGTH {pk_len} CAPTURED {pk_cap_len}.");

        let data = packet.data.to_vec();
    }
}
