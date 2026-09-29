use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rustuo_server::account_repository::LegacyXmlAccountRepository;
use rustuo_server::renaissance_tcp_runtime::RenaissanceTcpRuntime;
use rustuo_server::AuthIdIssuer;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Issuer(u32);
impl AuthIdIssuer for Issuer {
    type Error = ();
    fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0)
    }
}

fn runtime() -> (RenaissanceTcpRuntime, Issuer) {
    let path = std::env::temp_dir().join(format!(
        "rustuo-tcp-{}-{}.xml",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, "<accounts><account><username>alice</username><password>secret</password></account></accounts>").unwrap();
    let repo = LegacyXmlAccountRepository::open(&path).unwrap();
    fs::remove_file(path).unwrap();
    (
        RenaissanceTcpRuntime::bind(
            "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            repo,
            b"Shard".to_vec(),
            0x7f00_0001,
        )
        .unwrap(),
        Issuer(0x1122_3344),
    )
}

fn client(addr: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
}

fn login() -> [u8; 62] {
    let mut b = [0; 62];
    b[0] = 0x80;
    b[1..6].copy_from_slice(b"alice");
    b[31..37].copy_from_slice(b"secret");
    b
}
fn game(auth: u32) -> [u8; 65] {
    let mut b = [0; 65];
    b[0] = 0x91;
    b[1..5].copy_from_slice(&auth.to_be_bytes());
    b[5..10].copy_from_slice(b"alice");
    b[35..41].copy_from_slice(b"secret");
    b
}
fn exact(stream: &mut TcpStream, n: usize) -> Vec<u8> {
    let mut b = vec![0; n];
    stream.read_exact(&mut b).unwrap();
    b
}

#[test]
fn fragmented_login_redirect_and_fresh_socket_admission_keep_game_stream_open() {
    let (runtime, mut issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        assert!(runtime.serve_next(&mut issuer).unwrap().is_none());
        runtime.serve_next(&mut issuer).unwrap().unwrap()
    });
    let mut first = client(addr);
    first.write_all(&[0x01, 0x02]).unwrap();
    first.write_all(&[0x03, 0x04]).unwrap();
    let account_login = login();
    first.write_all(&account_login[..17]).unwrap();
    first.write_all(&account_login[17..]).unwrap();
    let mut expected = vec![0xA8, 0, 46, 0x5D, 0, 1, 0, 0];
    expected.extend_from_slice(b"Shard");
    expected.extend_from_slice(&[0; 27]);
    expected.extend_from_slice(&[0, 0, 0x7f, 0, 0, 1]);
    assert_eq!(exact(&mut first, 46), expected);
    first.write_all(&[0xA0, 0, 0]).unwrap();
    assert_eq!(
        exact(&mut first, 11),
        [
            0x8C,
            1,
            0,
            0,
            0x7f,
            (addr.port() >> 8) as u8,
            addr.port() as u8,
            0x11,
            0x22,
            0x33,
            0x44
        ]
    );
    drop(first);
    let mut game_socket = client(addr);
    game_socket
        .write_all(&0x1122_3344u32.to_be_bytes())
        .unwrap();
    game_socket.write_all(&game(0x1122_3344)).unwrap();
    let (mut handed_off, admission) = server.join().unwrap();
    assert_eq!(admission.account.username(), "alice");
    handed_off.write_all(b"still-open").unwrap();
    assert_eq!(exact(&mut game_socket, 10), b"still-open");
}

#[test]
fn mismatch_seed_does_not_consume_grant_but_replay_is_rejected() {
    let (runtime, mut issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        runtime.serve_next(&mut issuer).unwrap();
        assert!(runtime.serve_next(&mut issuer).is_err());
        assert!(runtime.serve_next(&mut issuer).unwrap().is_some());
        assert!(runtime.serve_next(&mut issuer).is_err());
    });
    let mut first = client(addr);
    first.write_all(&[1, 2, 3, 4]).unwrap();
    first.write_all(&login()).unwrap();
    exact(&mut first, 46);
    first.write_all(&[0xA0, 0, 0]).unwrap();
    exact(&mut first, 11);
    drop(first);
    let mut mismatch = client(addr);
    mismatch.write_all(&9u32.to_be_bytes()).unwrap();
    mismatch.write_all(&game(0x1122_3344)).unwrap();
    drop(mismatch);
    let mut correct = client(addr);
    correct.write_all(&0x1122_3344u32.to_be_bytes()).unwrap();
    correct.write_all(&game(0x1122_3344)).unwrap();
    drop(correct);
    let mut replay = client(addr);
    replay.write_all(&0x1122_3344u32.to_be_bytes()).unwrap();
    replay.write_all(&game(0x1122_3344)).unwrap();
    drop(replay);
    server.join().unwrap();
}

#[test]
fn invalid_seed_and_unsupported_first_packet_fail_closed() {
    let (runtime, mut issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            assert!(runtime.serve_next(&mut issuer).is_err());
        }
        assert!(runtime.serve_next(&mut issuer).unwrap().is_none());
    });
    let mut zero = client(addr);
    zero.write_all(&[0; 4]).unwrap();
    drop(zero);
    let mut bad = client(addr);
    bad.write_all(&[1, 2, 3, 4]).unwrap();
    bad.write_all(&[0x82]).unwrap();
    drop(bad);
    let mut credentials = login();
    credentials[31..37].copy_from_slice(b"wrong!");
    let mut rejected = client(addr);
    rejected.write_all(&[1, 2, 3, 4]).unwrap();
    rejected.write_all(&credentials).unwrap();
    assert_eq!(exact(&mut rejected, 2), [0x82, 0x03]);
    drop(rejected);
    server.join().unwrap();
}
