use std::{
    env, fs,
    io::{Read as _, Write as _},
    net::TcpStream,
};

use security_framework::{certificate::SecCertificate, secure_transport::ClientBuilder};

fn main() {
    let cert_path = env::args().next().unwrap();
    let cert_data = fs::read(cert_path).unwrap();
    let root_cert = SecCertificate::from_der(&cert_data).unwrap();

    let stream = TcpStream::connect("localhost:443").unwrap();
    let mut stream = ClientBuilder::new()
        .anchor_certificates(&[root_cert])
        .handshake("localhost", stream)
        .unwrap();

    stream
        .write_all(b"GET / HTTP/1.1\r\nhost: localhost\r\nconnection: close\r\n\r\n")
        .unwrap();
    let mut page = vec![];
    stream.read_to_end(&mut page).unwrap();
    println!("{}", String::from_utf8_lossy(&page));
}
