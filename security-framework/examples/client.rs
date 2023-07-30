use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

use security_framework::secure_transport::ClientBuilder;

fn main() {
    let stream = TcpStream::connect("google.com:443").unwrap();
    let mut stream = ClientBuilder::new()
        .handshake("google.com", stream)
        .unwrap();
    println!(
        "negotiated chipher: {:?}",
        stream.context().negotiated_cipher().unwrap()
    );
    println!(
        "negotiated version: {:?}",
        stream.context().negotiated_protocol_version().unwrap()
    );

    stream.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
    stream.flush().unwrap();
    let mut buf = vec![];
    stream.read_to_end(&mut buf).unwrap();
    println!("{}", String::from_utf8_lossy(&buf));
}
