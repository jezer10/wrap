
use std::{io::{Read,Write}, net::TcpListener, thread};
fn main() {
 if let Some(_proxy)=std::env::args().nth(1) {
  reqwest::blocking::Client::new().post("http://127.0.0.1:1/v1/control").header("Authorization", "Bearer audit-scoped-token").body("audit-input").send().unwrap();
  return;
 }
 let proxy=TcpListener::bind("127.0.0.1:0").unwrap();
 let address=proxy.local_addr().unwrap();
 let handle=thread::spawn(move || {
  let (mut sock,_)=proxy.accept().unwrap();let mut buf=[0;8192];let n=sock.read(&mut buf).unwrap();
  let request=String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
  sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
  (request.contains("authorization: bearer audit-scoped-token"),request.starts_with("post http://127.0.0.1:1/v1/control"))
 });
 let status=std::process::Command::new(std::env::current_exe().unwrap()).arg("child").env("HTTP_PROXY",format!("http://{address}")).env("http_proxy",format!("http://{address}")).env_remove("NO_PROXY").env_remove("no_proxy").env_remove("ALL_PROXY").env_remove("all_proxy").status().unwrap();
 assert!(status.success());let (credential,url)=handle.join().unwrap();
 println!("loopback_request_sent_to_proxy={url}; scoped_credential_received_by_proxy={credential}");
}
