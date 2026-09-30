use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rustuo_server::account_repository::LegacyXmlAccountRepository;
use rustuo_server::renaissance_tcp_runtime::{RenaissanceTcpRuntime, TcpRuntimeError};
use rustuo_server::renaissance_world_entry_session::{
    AvatarPresentation, RenaissanceLoginTailFixture, RenaissanceWorldEntrySession, WorldEntryError,
};
use rustuo_server::AuthIdIssuer;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct TempAccounts(std::path::PathBuf);

impl TempAccounts {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rustuo-server-{}-{}.xml",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(
            &path,
            "<accounts><account><username>alice</username><password>secret</password></account></accounts>",
        )
        .unwrap();
        Self(path)
    }
}

impl Drop for TempAccounts {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn wait_for_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "server did not finish its one-shot exchange"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn startup_reader(stdout: ChildStdout) -> (String, BufReader<ChildStdout>) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut output = BufReader::new(stdout);
        let mut line = String::new();
        let result = output.read_line(&mut line).map(|_| line);
        let _ = sender.send((result, output));
    });
    let (line, output) = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("server did not report readiness within five seconds");
    (line.expect("could not read server readiness"), output)
}

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

#[test]
fn executable_serves_one_loopback_login_reconnect_and_movement_exchange() {
    let accounts = TempAccounts::new();
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_rustuo-server"))
            .args([
                "--accounts",
                accounts.0.to_str().unwrap(),
                "--listen",
                "127.0.0.1:0",
                "--once",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (ready, _stdout_reader) = startup_reader(stdout);
    let address = ready
        .strip_prefix("LISTENING ")
        .expect("server should print its loopback address")
        .trim()
        .parse::<SocketAddr>()
        .unwrap();

    drop(client(address));

    let mut account_socket = client(address);
    account_socket.write_all(&[1, 2, 3, 4]).unwrap();
    account_socket.write_all(&login()).unwrap();
    let account_ack = exact(&mut account_socket, 46);
    assert_eq!(account_ack[0], 0xA8);
    assert_eq!(account_ack[42..46], [1, 0, 0, 127]);
    account_socket.write_all(&[0xA0, 0, 0]).unwrap();
    let redirect = exact(&mut account_socket, 11);
    assert_eq!(redirect[0], 0x8C);
    assert_eq!(redirect[1..5], [127, 0, 0, 1]);
    assert_eq!(
        u16::from_be_bytes([redirect[5], redirect[6]]),
        address.port()
    );
    let redirected_address = SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(redirect[1], redirect[2], redirect[3], redirect[4]),
        u16::from_be_bytes([redirect[5], redirect[6]]),
    ));
    assert_eq!(redirected_address, address);
    let auth_id = u32::from_be_bytes(redirect[7..11].try_into().unwrap());
    assert_ne!(auth_id, 0);
    drop(account_socket);

    let mut game_socket = client(redirected_address);
    game_socket.write_all(&auth_id.to_be_bytes()).unwrap();
    game_socket.write_all(&game(auth_id)).unwrap();
    assert_eq!(exact(&mut game_socket, 3), [0xB3, 0x06, 0x9A]);
    assert_eq!(exact(&mut game_socket, 85).first(), Some(&0x81));
    let mut play = [0; 73];
    play[0] = 0x5D;
    game_socket.write_all(&play).unwrap();
    game_socket.write_all(&movement_request(0x02)).unwrap();

    let status = wait_for_exit(&mut child.0);
    if !status.success() {
        let mut stderr = String::new();
        child
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        panic!("server exited with {status}: {stderr}");
    }
    let mut server_output = Vec::new();
    game_socket.read_to_end(&mut server_output).unwrap();
    let expected_movement_ack = rustuo_protocol::compress_legacy_packet(&[0x22, 0, 1]).unwrap();
    assert!(server_output.ends_with(&expected_movement_ack));
}

#[test]
fn executable_refuses_non_loopback_listeners() {
    let accounts = TempAccounts::new();
    let output = Command::new(env!("CARGO_BIN_EXE_rustuo-server"))
        .args([
            "--accounts",
            accounts.0.to_str().unwrap(),
            "--listen",
            "0.0.0.0:0",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("IPv4 loopback"));
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

fn avatar() -> AvatarPresentation {
    AvatarPresentation {
        name: b"Alice".to_vec(),
        body: 0x0190,
        direction: 2,
        hue: 0x0456,
        old_flags: 0x40,
        notoriety: 1,
    }
}

fn login_tail_fixture() -> RenaissanceLoginTailFixture {
    RenaissanceLoginTailFixture {
        global_light: 7,
        personal_light: 9,
        status: rustuo_protocol::RenaissanceAosMobileStatus {
            hits: (0x0102, 0x0304),
            can_rename: false,
            female: true,
            attributes: [5, 6, 7],
            stamina: (8, 9),
            mana: (10, 11),
            gold: 12,
            physical_resistance: 13,
            weight: 14,
            stat_cap: 15,
            followers: (2, 5),
            elemental_resistances: [16, 17, 18, 19],
            luck: 20,
            damage: (21, 22),
            tithing_points: 23,
        },
        war_mode: true,
        season: 2,
        current_time: (12, 34, 56),
    }
}

fn expected_login_tail_packets() -> Vec<Vec<u8>> {
    let incoming = vec![
        0xC9, 0x1F, 0x40, 0xFF, 0xD7, 0x4B, 0xCA, 0x35, 0xAA, 0xAA, 0x2E, 0xB2, 0xB5, 0x7C, 0x03,
        0x40,
    ];
    let mut status = vec![0x11, 0, 88, 0, 0, 0, 1];
    status.extend_from_slice(b"Alice");
    status.extend_from_slice(&[0; 25]);
    status.extend_from_slice(&[
        1, 2, 3, 4, 0, 4, 1, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0, 0, 0, 12, 0, 13, 0, 14,
        0, 15, 2, 5, 0, 16, 0, 17, 0, 18, 0, 19, 0, 20, 0, 21, 0, 22, 0, 0, 0, 23,
    ]);
    let compress = |frame: &[u8]| rustuo_protocol::compress_legacy_packet(frame).unwrap();
    let raw_incoming = vec![
        0x78, 0, 23, 0, 0, 0, 1, 1, 0x90, 0x0D, 0xAF, 0x0A, 0x0E, 14, 2, 4, 0x56, 0x40, 1, 0, 0, 0,
        0,
    ];
    let raw_tail = [
        vec![0x4F, 7],
        vec![0x4E, 0, 0, 0, 1, 9],
        vec![0x55],
        raw_incoming.clone(),
        status,
        vec![0x72, 1, 0, 0x32, 0],
        vec![0xBC, 2, 1],
        vec![0x5B, 12, 34, 56],
        vec![0xBF, 0, 6, 0, 8, 1],
        raw_incoming,
    ];
    let mut packets = vec![
        vec![0x80, 0xCE, 0xCE, 0x0F, 0xE8],
        vec![
            0x80, 0xCA, 0x51, 0x91, 0x03, 0xA8, 0, 0, 0, 0, 0, 0, 0, 0x06, 0x80,
        ],
        vec![0xB3, 0x06, 0x9A],
        incoming.clone(),
        vec![
            0xB4, 0x0F, 0xFD, 0x71, 0xD6, 0x56, 0xA9, 0x79, 0x46, 0xB5, 0x41, 0x15, 0x5A,
        ],
        incoming,
    ];
    packets.extend(raw_tail.iter().map(|frame| compress(frame)));
    packets
}

fn movement_request(packet_id: u8) -> [u8; 7] {
    [packet_id, 2, 0, 0, 0, 0, 0]
}

fn start_world_entry_server(
    runtime: RenaissanceTcpRuntime,
    mut issuer: Issuer,
    fixture: RenaissanceLoginTailFixture,
) -> std::thread::JoinHandle<
    Result<Option<(TcpStream, RenaissanceWorldEntrySession)>, TcpRuntimeError<()>>,
> {
    std::thread::spawn(move || {
        assert!(runtime
            .serve_world_entry_next(&mut issuer, 0x0003, avatar(), &fixture)
            .unwrap()
            .is_none());
        runtime.serve_world_entry_next(&mut issuer, 0x0003, avatar(), &fixture)
    })
}

fn admit_game_socket(addr: SocketAddr) -> TcpStream {
    let mut first = client(addr);
    first.write_all(&[1, 2, 3, 4]).unwrap();
    first.write_all(&login()).unwrap();
    exact(&mut first, 46);
    first.write_all(&[0xA0, 0, 0]).unwrap();
    exact(&mut first, 11);
    drop(first);

    let mut game_socket = client(addr);
    game_socket
        .write_all(&0x1122_3344u32.to_be_bytes())
        .unwrap();
    game_socket.write_all(&game(0x1122_3344)).unwrap();
    game_socket
}

#[test]
fn admitted_game_socket_echoes_pings_before_movement_and_keeps_socket_open() {
    let (runtime, issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = start_world_entry_server(runtime, issuer, login_tail_fixture());
    let mut game_socket = admit_game_socket(addr);

    assert_eq!(exact(&mut game_socket, 3), [0xB3, 0x06, 0x9A]);
    assert_eq!(
        exact(&mut game_socket, 85),
        [
            0x81, 0x7F, 0x25, 0xA2, 0x59, 0xAA, 0x0C, 0x6F, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12,
            0xE8,
        ]
    );
    let mut play = [0; 73];
    play[0] = 0x5D;
    play[65..69].copy_from_slice(&0i32.to_be_bytes());
    game_socket.write_all(&play).unwrap();
    assert_eq!(
        exact(&mut game_socket, 20),
        [
            0x48, 0x01, 0xF0, 0x0F, 0xAE, 0x97, 0x94, 0x6B, 0x54, 0x55, 0x11, 0x8B, 0x16, 0x2C,
            0x40, 0x0B, 0x23, 0x88, 0x00, 0x1A,
        ]
    );
    for expected in expected_login_tail_packets() {
        assert_eq!(exact(&mut game_socket, expected.len()), expected);
    }
    let mut post_login_packets = vec![0x73, 0xA5, 0x73, 0x00];
    post_login_packets.extend_from_slice(&movement_request(0x02));
    let second_movement = [0x02, 2, 1, 0, 0, 0, 0];
    post_login_packets.extend_from_slice(&second_movement);
    game_socket.write_all(&post_login_packets).unwrap();
    let first_ping_ack = rustuo_protocol::compress_legacy_packet(&[0x73, 0xA5]).unwrap();
    assert_eq!(
        exact(&mut game_socket, first_ping_ack.len()),
        first_ping_ack
    );
    let second_ping_ack = rustuo_protocol::compress_legacy_packet(&[0x73, 0x00]).unwrap();
    assert_eq!(
        exact(&mut game_socket, second_ping_ack.len()),
        second_ping_ack
    );
    let movement_ack = rustuo_protocol::compress_legacy_packet(&[0x22, 0, 1]).unwrap();
    assert_eq!(exact(&mut game_socket, movement_ack.len()), movement_ack);

    let (mut handed_off, session) = server.join().unwrap().unwrap().unwrap();
    assert_eq!(session.account().username(), "alice");
    let mut pending_movement = [0; 7];
    handed_off.read_exact(&mut pending_movement).unwrap();
    assert_eq!(pending_movement, second_movement);
    handed_off.write_all(b"still-open").unwrap();
    assert_eq!(exact(&mut game_socket, 10), b"still-open");
}

#[test]
fn world_entry_echoes_complete_ping_before_rejecting_truncated_ping() {
    let (runtime, issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = start_world_entry_server(runtime, issuer, login_tail_fixture());
    let mut game_socket = admit_game_socket(addr);
    exact(&mut game_socket, 3);
    exact(&mut game_socket, 85);
    let mut play = [0; 73];
    play[0] = 0x5D;
    game_socket.write_all(&play).unwrap();
    assert_eq!(exact(&mut game_socket, 20).first(), Some(&0x48));
    for expected in expected_login_tail_packets() {
        assert_eq!(exact(&mut game_socket, expected.len()), expected);
    }

    game_socket.write_all(&[0x73, 0xA5, 0x73]).unwrap();
    game_socket.shutdown(std::net::Shutdown::Write).unwrap();
    let ping_ack = rustuo_protocol::compress_legacy_packet(&[0x73, 0xA5]).unwrap();
    assert_eq!(exact(&mut game_socket, ping_ack.len()), ping_ack);
    assert!(matches!(
        server.join().unwrap(),
        Err(TcpRuntimeError::Io(error))
            if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
    let mut response = [0; 1];
    assert_eq!(game_socket.read(&mut response).unwrap(), 0);
}

#[test]
fn world_entry_rejects_nonzero_and_malformed_character_slots() {
    for (slot, packet_id) in [(1i32, 0x5D), (0, 0x7F)] {
        let (runtime, issuer) = runtime();
        let addr = runtime.local_addr().unwrap();
        let server = start_world_entry_server(runtime, issuer, login_tail_fixture());
        let mut game_socket = admit_game_socket(addr);
        exact(&mut game_socket, 3);
        exact(&mut game_socket, 85);
        let mut play = [0; 73];
        play[0] = packet_id;
        play[65..69].copy_from_slice(&slot.to_be_bytes());
        game_socket.write_all(&play).unwrap();
        let result = server.join().unwrap();
        if packet_id == 0x5D {
            assert!(matches!(
                result,
                Err(TcpRuntimeError::WorldEntry(
                    WorldEntryError::UnsupportedSlot(1)
                ))
            ));
        } else {
            assert!(matches!(
                result,
                Err(TcpRuntimeError::WorldEntry(WorldEntryError::Decode(_)))
            ));
        }
        let mut response = [0; 1];
        assert_eq!(game_socket.read(&mut response).unwrap(), 0);
    }
}

#[test]
fn world_entry_reports_truncated_play_request_as_unexpected_eof() {
    let (runtime, issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = start_world_entry_server(runtime, issuer, login_tail_fixture());
    let mut game_socket = admit_game_socket(addr);
    exact(&mut game_socket, 3);
    exact(&mut game_socket, 85);
    game_socket.write_all(&[0x5D; 12]).unwrap();
    drop(game_socket);
    assert!(matches!(
        server.join().unwrap(),
        Err(TcpRuntimeError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
}

#[test]
fn world_entry_reports_malformed_complete_movement_as_world_entry_error() {
    let (runtime, issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = start_world_entry_server(runtime, issuer, login_tail_fixture());
    let mut game_socket = admit_game_socket(addr);
    exact(&mut game_socket, 3);
    exact(&mut game_socket, 85);
    let mut play = [0; 73];
    play[0] = 0x5D;
    game_socket.write_all(&play).unwrap();
    exact(&mut game_socket, 20);
    for expected in expected_login_tail_packets() {
        assert_eq!(exact(&mut game_socket, expected.len()), expected);
    }
    game_socket.write_all(&movement_request(0x03)).unwrap();

    assert!(matches!(
        server.join().unwrap(),
        Err(TcpRuntimeError::WorldEntry(
            WorldEntryError::MovementDecode(_)
        ))
    ));
    let mut response = [0; 1];
    assert_eq!(game_socket.read(&mut response).unwrap(), 0);
}

#[test]
fn world_entry_reports_truncated_movement_as_unexpected_eof() {
    let (runtime, issuer) = runtime();
    let addr = runtime.local_addr().unwrap();
    let server = start_world_entry_server(runtime, issuer, login_tail_fixture());
    let mut game_socket = admit_game_socket(addr);
    exact(&mut game_socket, 3);
    exact(&mut game_socket, 85);
    let mut play = [0; 73];
    play[0] = 0x5D;
    game_socket.write_all(&play).unwrap();
    exact(&mut game_socket, 20);
    for expected in expected_login_tail_packets() {
        assert_eq!(exact(&mut game_socket, expected.len()), expected);
    }
    game_socket.write_all(&movement_request(0x02)[..6]).unwrap();
    drop(game_socket);

    assert!(matches!(
        server.join().unwrap(),
        Err(TcpRuntimeError::Io(error))
            if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
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
