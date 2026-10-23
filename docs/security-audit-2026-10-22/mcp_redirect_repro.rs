
use std::{io::{Read,Write}, net::TcpListener, thread};
fn main() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    let destination_addr = destination.local_addr().unwrap();
    let origin = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin_addr = origin.local_addr().unwrap();
    let receiver = thread::spawn(move || {
        let (mut socket, _) = destination.accept().unwrap();
        let mut buf = [0;8192];
        let n = socket.read(&mut buf).unwrap();
        let text = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        (text.contains("x-api-key: audit-dummy-key"), text.contains("authorization: bearer audit-dummy-token"))
    });
    let redirector = thread::spawn(move || {
        let (mut socket, _) = origin.accept().unwrap();
        let mut buf = [0;8192];
        socket.read(&mut buf).unwrap();
        write!(socket, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{destination_addr}/capture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    let rt=tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-api-key", "audit-dummy-key".parse().unwrap());
        headers.insert("authorization", "Bearer audit-dummy-token".parse().unwrap());
        let client=reqwest::Client::builder().no_proxy().default_headers(headers).build().unwrap();
        client.post(format!("http://{origin_addr}/mcp")).body("{}").send().await.unwrap();
    });
    redirector.join().unwrap();
    let (custom, standard)=receiver.join().unwrap();
    println!("cross_origin_x_api_key_received={custom}; cross_origin_authorization_received={standard}");
}
