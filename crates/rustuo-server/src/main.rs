#![forbid(unsafe_code)]

use std::env;
use std::io::{self, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process;

use rustuo_server::account_repository::LegacyXmlAccountRepository;
use rustuo_server::renaissance_tcp_runtime::{RenaissanceTcpRuntime, TcpRuntimeError};
use rustuo_server::renaissance_world_entry_session::{
    AvatarPresentation, RenaissanceLoginTailFixture,
};
use rustuo_server::AuthIdIssuer;

const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:2593";
const USAGE: &str = "Usage: rustuo-server --accounts <accounts.xml> [--listen <IPv4-loopback:port>] [--once]\n\nRuns the serial, loopback-only Renaissance 5.0.8.3 smoke server. The default\nlistener is 127.0.0.1:2593. --once exits after one successful movement reply.\nThis fixture is not production-ready and does not establish real-client compatibility.\n";

struct Options {
    accounts: PathBuf,
    listen: SocketAddr,
    once: bool,
}

enum Command {
    Help,
    Run(Options),
}

fn main() {
    match parse_arguments(env::args().skip(1)) {
        Ok(Command::Help) => print!("{USAGE}"),
        Ok(Command::Run(options)) => {
            if let Err(error) = serve(options) {
                eprintln!("rustuo-server: {error}");
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("rustuo-server: {error}\n{USAGE}");
            process::exit(2);
        }
    }
}

fn parse_arguments(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let mut accounts = None;
    let mut listen = DEFAULT_LISTEN_ADDRESS
        .parse::<SocketAddr>()
        .expect("default listener address is valid");
    let mut once = false;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--accounts" => {
                let path = args
                    .next()
                    .filter(|value| !value.starts_with('-'))
                    .ok_or_else(|| "--accounts requires a file path".to_owned())?;
                accounts = Some(PathBuf::from(path));
            }
            "--listen" => {
                let value = args.next().ok_or_else(|| {
                    "--listen requires an IPv4 loopback socket address".to_owned()
                })?;
                let address = value
                    .parse::<SocketAddr>()
                    .map_err(|_| "--listen requires an IPv4 loopback socket address".to_owned())?;
                if !matches!(address, SocketAddr::V4(address) if address.ip().is_loopback()) {
                    return Err("--listen must be an IPv4 loopback address".to_owned());
                }
                listen = address;
            }
            "--once" => once = true,
            _ => return Err(format!("unknown option: {argument}")),
        }
    }

    let accounts = accounts.ok_or_else(|| "--accounts is required".to_owned())?;
    Ok(Command::Run(Options {
        accounts,
        listen,
        once,
    }))
}

fn serve(options: Options) -> Result<(), String> {
    let repository = LegacyXmlAccountRepository::open(&options.accounts)
        .map_err(|error| format!("could not load accounts XML: {error:?}"))?;
    let advertised_address = match options.listen.ip() {
        IpAddr::V4(address) => u32::from_le_bytes(address.octets()),
        IpAddr::V6(_) => unreachable!("CLI rejects non-IPv4 listener addresses"),
    };
    let runtime = RenaissanceTcpRuntime::bind(
        options.listen,
        repository,
        b"RustUO".to_vec(),
        advertised_address,
    )
    .map_err(|error| format!("could not bind loopback listener: {error}"))?;
    let local_address = runtime
        .local_addr()
        .map_err(|error| format!("could not inspect loopback listener: {error}"))?;
    println!("LISTENING {local_address}");
    io::stdout()
        .flush()
        .map_err(|error| format!("could not report listener address: {error}"))?;

    let mut issuer = LocalAuthIdIssuer::new();
    let avatar = smoke_avatar();
    let login_tail = smoke_login_tail();
    loop {
        match runtime.serve_world_entry_next(&mut issuer, 0x0003, avatar.clone(), &login_tail) {
            Ok(None) => {}
            Ok(Some((_stream, session))) => {
                println!(
                    "SERVED one movement request for {}",
                    session.account().username()
                );
                io::stdout()
                    .flush()
                    .map_err(|error| format!("could not report completed exchange: {error}"))?;
                if options.once {
                    return Ok(());
                }
            }
            Err(TcpRuntimeError::Accept(error)) => {
                return Err(format!("could not accept loopback client: {error}"));
            }
            Err(_) => {
                eprintln!("rustuo-server: client exchange failed; continuing to accept clients")
            }
        }
    }
}

struct LocalAuthIdIssuer {
    next: Option<u32>,
}

#[derive(Debug)]
struct AuthIdExhausted;

impl LocalAuthIdIssuer {
    fn new() -> Self {
        Self { next: Some(1) }
    }
}

impl AuthIdIssuer for LocalAuthIdIssuer {
    type Error = AuthIdExhausted;

    fn issue_auth_id(&mut self) -> Result<u32, Self::Error> {
        let auth_id = self.next.ok_or(AuthIdExhausted)?;
        self.next = auth_id.checked_add(1);
        Ok(auth_id)
    }
}

fn smoke_avatar() -> AvatarPresentation {
    AvatarPresentation {
        name: b"Alice".to_vec(),
        body: 0x0190,
        direction: 2,
        hue: 0x0456,
        old_flags: 0x40,
        notoriety: 1,
    }
}

fn smoke_login_tail() -> RenaissanceLoginTailFixture {
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
