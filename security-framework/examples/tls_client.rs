use std::{
    io::{Read as _, Write as _},
    net::TcpStream,
};

use security_framework::secure_transport::ClientBuilder;

fn main() {
    let stream = TcpStream::connect("example.com:443").unwrap();
    let mut stream = ClientBuilder::new()
        .handshake("example.com", stream)
        .unwrap();
    println!(
        "negotiated cipher: {:?}",
        stream.context().negotiated_cipher().unwrap()
    );
    println!(
        "negotiated version: {:?}",
        stream.context().negotiated_protocol_version().unwrap()
    );

    stream
        .write_all(b"GET / HTTP/1.1\r\nhost: example.com\r\nconnection: close\r\n\r\n")
        .unwrap();
    stream.flush().unwrap();

    let mut buf = vec![];
    stream.read_to_end(&mut buf).unwrap();
    println!("{}", String::from_utf8_lossy(&buf));
}
