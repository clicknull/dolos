extern crate rustc_serialize;
extern crate docopt;
extern crate core;

use docopt::Docopt;
use std::net::UdpSocket;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::thread;
use core::str::FromStr;
use std::collections::hash_map::HashMap;


static USAGE: &'static str =    "
Usage: dolos [options] [<srcip>] <srcport> <dstip> <dstport>
       dolos --help

Options:
  -h, --help       Show this message.
  -v, --verbose    Print more information.
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_srcport: u16,
    arg_dstport: u16,
    arg_srcip: String,
    arg_dstip: String
}

fn spawn_return_socket_thread(local_socket: UdpSocket, remote_socket: UdpSocket, proxy_src: SocketAddr) {
    thread::spawn(move || {
        let mut buf = [0; 2048];
        loop {
            match remote_socket.recv_from(&mut buf) {
                Ok((amt, _src)) => {
                    // Send a reply to the socket we received data from
                    let buf = &mut buf[.. amt];
                    local_socket.send_to(buf, proxy_src).ok();
                    print!(" <--< ");
                    print_u8(buf);
                },
                Err(e) => println!("couldn't receive a datagram: {}", e),
            }
        }
    });
}

fn print_u8(buf: &[u8]){
    for i in buf.iter() {
        print!("0x{:0>2X}, ", i)
    }
    println!("")
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());
    println!("Proxying {}:{} to {}:{}.", args.arg_srcip, args.arg_srcport, args.arg_dstip, args.arg_dstport);

    let local_addr = match args.arg_dstip.as_str() {
        "" => (args.arg_dstip.as_str(), args.arg_dstport),
        _  => ("0.0.0.0", args.arg_dstport),
    };
    let remote_addr = SocketAddrV4::new(Ipv4Addr::new(0,0,0,0), 0);
    let dest_addr = SocketAddr::new(IpAddr::from_str(args.arg_dstip.as_str()).ok().expect("couldn't parse dest ip"), args.arg_dstport);

    // Socket for incoming requests from proxy clients
    let local_socket = match UdpSocket::bind(local_addr) {
        Ok(s) => s,
        Err(e) => panic!("couldn't bind local socket: {}", e),
    };

    let mut buf = [0; 2048];
    let mut src_to_socket_list: HashMap<SocketAddr, UdpSocket> = HashMap::new();

    loop {
        match local_socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                let remote_socket = src_to_socket_list.entry(src).or_insert_with(|| {
                    println!("new client: {}", src);

                    // socket for server being proxied to
                    let remote_socket =  UdpSocket::bind(remote_addr).ok().expect("couldn't bind remote socket");

                    spawn_return_socket_thread(
                        local_socket.try_clone().ok().expect("couldn't clone lsock"),
                        remote_socket.try_clone().ok().expect("couldn't clone rsock"),
                        src);

                    remote_socket
                });


                // send message to dest server
                let buf = &mut buf[.. amt];
                remote_socket.send_to(buf, dest_addr).ok();
                print!(" >--> ");
                print_u8(buf);
            },
            Err(e) => println!("couldn't receive a datagram: {}", e)
        }
    }
}
