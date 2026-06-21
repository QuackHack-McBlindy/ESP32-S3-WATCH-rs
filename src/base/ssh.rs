// BASE/SSH
// SSH SERVER + CLIENT (PORT 2222)
// USED AS A SHELL FRONTEND & KEYBOARD-OVER-SSH FEED
// ENSURES AN ENCRYPTED COMMUNICATION BETWEEN ANY OF MY MACHINES
// + PHONES

use core::fmt::Write as _;

// ─────────────────────────────────────────────────────────────────
pub enum SshCommand {
    Enable,
    Disable,
}

pub static SSH_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    SshCommand,
    1,
> = embassy_sync::channel::Channel::new();

static AUTHORIZED_KEYS: embassy_sync::once_lock::OnceLock<
    heapless::Vec<[u8; 32], { crate::state::MAX_KEYS }>
> = embassy_sync::once_lock::OnceLock::new();


// ─────────────────────────────────────────────────────────────────
// LOAD HEX KEYS FROM ENV (OPTIONAL)
const EMBEDDED_HOST_KEY: [u8; 32] = match crate::state::SSH_HOSTKEY_HEX {
    Some(hex) => hex_to_bytes(hex),
    None => [0u8; 32],
};

static HOST_KEY: embassy_sync::once_lock::OnceLock<sunset::SignKey> = embassy_sync::once_lock::OnceLock::new();

// COMPILE-TIME HEX PARSER
pub const fn hex_to_bytes<const N: usize>(hex: &str) -> [u8; N] {
    let bytes = hex.as_bytes();
    assert!(bytes.len() == N * 2, "Hex string must be exactly 2*N characters");
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        let hi = hex_digit(bytes[2 * i]);
        let lo = hex_digit(bytes[2 * i + 1]);
        out[i] = (hi << 4) | lo;
        i += 1;
    }
    out
}

const fn hex_digit(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("Invalid hex char"),
    }
}

// PARSE PUBLIC KEY DEFINED IN `.env` (OPTIONAL)
fn parse_ed25519_pubkey(line: &str) -> Option<[u8; 32]> {
    let base64_part = if let Some(rest) = line.strip_prefix("ssh-ed25519 ") {
        rest.trim()
    } else { line.trim() };
    let base64_part = base64_part.split_whitespace().next()?;
    let decoded = simple_base64_decode(base64_part)?;
    if decoded.len() < 51 { return None; }
    let key_start = 4 + 11 + 4;
    if &decoded[4..4+11] != b"ssh-ed25519" { return None; }
    let key_len = u32::from_be_bytes([decoded[15], decoded[16], decoded[17], decoded[18]]) as usize;
    if key_len != 32 || decoded.len() < key_start + 32 { return None; }
    let mut raw_key = [0u8; 32];
    raw_key.copy_from_slice(&decoded[key_start..key_start+32]);
    Some(raw_key)
}

// MINI BASE64 DECODER (no‑alloc)
fn simple_base64_decode(input: &str) -> Option<alloc::vec::Vec<u8>> {
    let input = input.trim_end_matches('=');
    let mut bytes = alloc::vec::Vec::new();
    let mut buffer = 0u32;
    let mut bits_collected = 0u32;

    for c in input.chars() {
        let value = match c {
            'A'..='Z' => c as u32 - 65,
            'a'..='z' => c as u32 - 71,
            '0'..='9' => c as u32 + 4,
            '+' => 62,
            '/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | value;
        bits_collected += 6;
        if bits_collected >= 8 {
            bits_collected -= 8;
            bytes.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }
    Some(bytes)
}

fn init_authorized_keys() -> &'static heapless::Vec<[u8; 32], { crate::state::MAX_KEYS }> {
    AUTHORIZED_KEYS.get_or_init(|| {
        let mut keys = heapless::Vec::new();
        for opt_line in &[
            crate::state::SSH_PUBKEY,
            crate::state::SSH_PUBKEY2,
            crate::state::SSH_PUBKEY3,
        ] {
            if let Some(line) = opt_line {
                if let Some(raw) = parse_ed25519_pubkey(line) {
                    let _ = keys.push(raw);
                }
            }
        }
        keys
    })
}

// ─────────────────────────────────────────────────────────────────
// PER-CONNECTION SSH TASK
#[embassy_executor::task]
async fn ssh_task(mut socket: embassy_net::tcp::TcpSocket<'static>) {
    let remote = socket.remote_endpoint().unwrap();
    defmt::info!("╬═══════════════════════════════╬");
    defmt::info!("╬ SSH: {} CONNECTED! ╬", remote.addr);
    defmt::info!("╬═══════════════════════════════╬");

    let (mut rx, mut tx) = socket.split();

    let in_buf = alloc::vec![0u8; 8192];
    let out_buf = alloc::vec![0u8; 8192];
    let server: &'static sunset_async::SSHServer<'static> = alloc::boxed::Box::leak(alloc::boxed::Box::new(sunset_async::SSHServer::new(
        alloc::vec::Vec::leak(in_buf),
        alloc::vec::Vec::leak(out_buf),
    )));

    let hostkey = get_host_key();
    if let sunset::SignKey::Ed25519(key) = hostkey {
        let public_bytes: [u8; 32] = key.verifying_key().to_bytes();
        defmt::info!("SSH hostkey public: {:?}", public_bytes);
    }

    let mut ph = sunset_async::ProgressHolder::new();
    let run_fut = server.run(&mut rx, &mut tx);

    let mut opened_handle: Option<sunset::ChanHandle> = None;

    let progress_fut = async {
        loop {
            match server.progress(&mut ph).await {
                Ok(event) => match event {
                    sunset::event::ServEvent::Hostkeys(h) => {
                        h.hostkeys(&[&hostkey]);
                    }
                    // FIRST AUTH CHECK: USERNAME
                    sunset::event::ServEvent::FirstAuth(mut a) => {
                        if a.username().unwrap() == crate::state::SSH_USER {
                            a.enable_password_auth(false).unwrap();
                            a.enable_pubkey_auth(true).unwrap();
                            defmt::info!("SSH: pubkey auth enabled");
                        }
                    }
                    // SECOND AUTH CHECK: PUBLIC KEY
                    sunset::event::ServEvent::PubkeyAuth(a) => {
                        let pubkey = match a.pubkey() {
                            Ok(pk) => pk,
                            Err(e) => {
                                defmt::warn!("SSH: failed to get pubkey: {:?}", defmt::Debug2Format(&e));
                                a.reject().ok();
                                return;
                            }
                        };
                        let offered_raw: [u8; 32] = match pubkey {
                            sunset::PubKey::Ed25519(k) => k.key.0,
                            _ => {
                                defmt::warn!("SSH: unsupported pubkey type");
                                a.reject().ok();
                                return;
                            }
                        };
                        let authorized = init_authorized_keys()
                            .iter()
                            .any(|k| k == &offered_raw);
                        if !authorized {
                            defmt::warn!("SSH: pubkey not authorized");
                            a.reject().ok();
                            return;
                        }
                        // ENSURES USER KNOWS CONNECTION IS ESTABLISHED
                        //crate::set_speaker_volume(70);
                        //yo_esp::play_ding().await;
                        //crate::delay_s!(1);
                        //crate::set_speaker_volume(0);
                        defmt::info!("SSH: pubkey auth success");
                        a.allow().ok();
                    }
                    sunset::event::ServEvent::Authenticated => {
                        defmt::debug!("SSH: user authenticated");
                    }
                    sunset::event::ServEvent::OpenSession(respond) => {
                        opened_handle = Some(respond.accept().unwrap());
                        defmt::debug!("SSH: session opened");
                    }
                    // SSH SHELL SESSION
                    sunset::event::ServEvent::SessionShell(a) => {
                        a.succeed().unwrap();
                        defmt::debug!("SSH: shell request accepted – spawning shell session");
                        if let Some(handle) = opened_handle.take() {
                            let spawner = unsafe { embassy_executor::Spawner::for_current_executor() }.await;
                            crate::spawn!(spawner, shell_session_task(server, handle));
                        }
                    }
                    sunset::event::ServEvent::SessionExec(a) => {
                        a.succeed().unwrap();
                        defmt::debug!("SSH: exec request accepted – spawning shell session");
                        if let Some(handle) = opened_handle.take() {
                            let spawner = unsafe { embassy_executor::Spawner::for_current_executor() }.await;
                            crate::spawn!(spawner, shell_session_task(server, handle));
                        }
                    }
                    sunset::event::ServEvent::SessionPty(a) => {
                        a.succeed().ok();
                        defmt::debug!("SSH: pty request accepted");
                    }
                    sunset::event::ServEvent::SessionEnv(a) => {
                        a.succeed().ok();
                    }
                    _ => { defmt::debug!("SSH event: {:?}", defmt::Debug2Format(&event)); }
                },
                Err(e) => {
                    defmt::error!("SSH error: {:?}", defmt::Debug2Format(&e));
                    break;
                }
            }
            drop(ph);
            ph = sunset_async::ProgressHolder::new();
        }
    };

    embassy_futures::select::select(run_fut, progress_fut).await;
}

// ─────────────────────────────────────────────────────────────────
// SSH LISTENER TASK
#[embassy_executor::task]
pub async fn sshd_task(stack: &'static embassy_net::Stack<'static>) {
    let _ = get_host_key();

    let mut enabled = crate::load!(crate::state::SSH_STATE);
    if !enabled {
        defmt::info!(">_ 💤");
    }

    loop {
        while !enabled {
            let cmd = SSH_CMD.receive().await;
            match cmd {
                SshCommand::Enable => {
                    enabled = true;
                    crate::store!(crate::state::SSH_STATE, true);
                    defmt::info!(">_ ☑️");
                }
                _ => {}
            }
        }

        let rx_buf = alloc::vec::Vec::leak(alloc::vec![0u8; 1024]);
        let tx_buf = alloc::vec::Vec::leak(alloc::vec![0u8; 1024]);
        let mut socket = embassy_net::tcp::TcpSocket::new(*stack, rx_buf, tx_buf);

        'active: loop {
            let accept_fut = socket.accept(2222);
            let cmd_fut = SSH_CMD.receive();

            match embassy_futures::select::select(accept_fut, cmd_fut).await {
                embassy_futures::select::Either::First(result) => {
                    match result {
                        Ok(()) => {
                            defmt::debug!("SSH connection accepted");
                            let spawner = unsafe {
                                embassy_executor::Spawner::for_current_executor()
                            }
                            .await;
                            crate::spawn!(spawner, ssh_task(socket));

                            let rx_buf = alloc::vec::Vec::leak(alloc::vec![0u8; 1024]);
                            let tx_buf = alloc::vec::Vec::leak(alloc::vec![0u8; 1024]);
                            socket = embassy_net::tcp::TcpSocket::new(*stack, rx_buf, tx_buf);
                        }
                        Err(e) => {
                            defmt::warn!("SSH listen error: {:?}", e);
                            break 'active;
                        }
                    }
                }
                embassy_futures::select::Either::Second(cmd) => {
                    match cmd {
                        SshCommand::Disable => {
                            enabled = false;
                            crate::store!(crate::state::SSH_STATE, false);
                            defmt::info!(">_ 💤");
                            break 'active;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}


// ─────────────────────────────────────────────────────────────────
// SSH SHELL SESSION TASK
#[embassy_executor::task]
async fn shell_session_task(
    server: &'static sunset_async::SSHServer<'static>,
    handle: sunset::ChanHandle,
) {
    let mut chan = server.stdio(handle).await.unwrap();
    let _ = crate::base::shell::shell_session(&mut chan).await;
}


// ─────────────────────────────────────────────────────────────────
// HELPERS
fn get_host_key() -> &'static sunset::SignKey {
    HOST_KEY.get_or_init(|| {
        if EMBEDDED_HOST_KEY != [0u8; 32] {
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&EMBEDDED_HOST_KEY);
            sunset::SignKey::Ed25519(signing_key)
        } else {
            defmt::warn!("No embedded host key – generating a temporary one!");
            let key = sunset::SignKey::generate(sunset::KeyType::Ed25519, None)
                .expect("RNG failure");
            if let sunset::SignKey::Ed25519(k) = &key {
                let public = k.verifying_key().to_bytes();
                let secret = k.to_bytes();
                defmt::info!(
                    "Temporary key pub: {:?}, secret: {:?}",
                    public, secret
                );
                defmt::info!( "Copy the secret bytes and set EMBEDDED_HOST_KEY to make it permanent." );
            }
            key
        }
    })
}
