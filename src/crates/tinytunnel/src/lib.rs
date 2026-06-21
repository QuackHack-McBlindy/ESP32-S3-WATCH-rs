// WIP


use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use embassy_net::udp::{UdpSocket, PacketMetadata};
use embassy_net::IpEndpoint;
use core::net::SocketAddr;
use defmt::{info, debug, error};
use defmt::Debug2Format;

use alloc::vec::Vec;

use alloc::boxed::Box;
use embassy_net::Stack;
use embassy_time::{Duration, Timer, Instant};
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use core::net::SocketAddr;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use libm::sqrtf;
use defmt::Format;

extern crate alloc;

// ───────────────────────────────────────────────────────────────────────
// CONSTANTS
const UDP_RX_BUF_SIZE: usize = 1024;
const UDP_TX_BUF_SIZE: usize = 4096;

// ───────────────────────────────────────────────────────────────────────
// EMBASSY SYNC COMMANDS

// WG COMMANDS
#[derive(Format)]
pub enum WgCommand {
    Enable,
    Disable,
}

pub static WG_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    WgCommand,
    1,
> = embassy_sync::channel::Channel::new();

pub static WG_SEND: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    Vec<u8>,
    4,
> = embassy_sync::channel::Channel::new();

/// Signal that the handshake has finished (optional, for user logic).
pub static WG_READY: embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    (),
> = embassy_sync::signal::Signal::new();

// ───────────────────────────────────────────────────────────────────────
// HELPERS


// LOAD HEX KEYS FROM ENV (OPTIONAL)
const EMBEDDED_WG_KEY: [u8; 32] = match crate::state::WG_PRIVATE_KEY_HEX {
    Some(hex) => hex_to_bytes(hex),
    None => [0u8; 32],
};

static WG_KEY: embassy_sync::once_lock::OnceLock<sunset::SignKey> = embassy_sync::once_lock::OnceLock::new();

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


// ───────────────────────────────────────────────────────────────────────
// WIREGUARD TASK
#[embassy_executor::task]
pub async fn wireguard_tsk(
    stack: &'static Stack<'static>,
    endpoint: &'static str,          // server IP (e.g. "192.168.1.211")
    port: u16,                       // server WireGuard port (e.g. 51820)
) {
    // Parse the server address once.
    let ip: core::net::Ipv4Addr = match endpoint.parse() {
        Ok(ip) => ip,
        Err(_) => {
            error!("Invalid WireGuard endpoint IP: {}", endpoint);
            return;
        }
    };
    let server_addr = SocketAddr::V4(core::net::SocketAddrV4::new(ip, port));

    loop {
        // Wait until someone enables WireGuard.
        info!("🛡️  💤");
        let cmd = WG_CMD.receive().await;
        debug!("WG command: {:?}", cmd);

        match cmd {
            WgCommand::Enable => {
                info!("🛡️  ☑️  Starting WireGuard...");

                // ── build the session ──
                let static_sk = StaticPrivateKey(EMBEDDED_WG_KEY);
                let mut config = Config::new(static_sk);

                // TODO: replace with your server’s actual public key (32 bytes)
                let server_pk = PublicKey([0x00; 32]);   // <-- FIX ME
                let peer_config = StaticPeerConfig::new(
                    server_pk,
                    None,                // no pre‑shared key for now
                    Some(server_addr),
                );
                let peer_id = config.insert_peer(peer_config);

                // RNG – seed from hardware TRNG once, store in a static (or generate here)
                // For simplicity, we use a constant seed; replace with real entropy.
                let mut rng = ChaCha12Rng::seed_from_u64(0xDEAD_BEEF_1234_5678);
                let mut sessions = Sessions::new(config, &mut rng);

                // ── create a UDP socket ──
                // Buffers for smoltcp UDP. Sizes are generous.
                let mut rx_meta = [PacketMetadata::EMPTY; 4];
                let mut rx_buf = [0u8; 2048];
                let mut tx_meta = [PacketMetadata::EMPTY; 4];
                let mut tx_buf = [0u8; 2048];
                let mut socket = UdpSocket::new(
                    stack.clone(),
                    &mut rx_meta,
                    &mut rx_buf,
                    &mut tx_meta,
                    &mut tx_buf,
                );

                // Bind to any available port (WireGuard doesn’t require a fixed source port).
                socket.bind(embassy_net::IpListenEndpoint { addr: None, port: 12346 }).unwrap();

                let mut disabled = false;
                let mut handshake_done_signalled = false;

                // ── main loop ──
                loop {
                    // Check for Disable command (non‑blocking).
                    if let Ok(WgCommand::Disable) = WG_CMD.try_receive() {
                        disabled = true;
                        info!("WireGuard disabled");
                        break;
                    }

                    // 1. Timer tick – 100 ms gives ~10 Hz.
                    let now = tai64_now();
                    let maintenance = sessions.turn(now, &mut rng);
                    if let Some(maintenance) = maintenance {
                        // Send the handshake / keepalive packet.
                        let dest = maintenance.to();
                        let data = maintenance.data();
                        let endpoint = IpEndpoint::new(dest.ip().into(), dest.port());
                        socket.send_to(data, endpoint).await.ok();
                    }

                    // 2. Check for incoming UDP packet (non‑blocking).
                    match socket.try_recv_from(&mut rx_buf) {
                        Ok((n, meta)) => {
                            let src = SocketAddr::new(meta.endpoint.addr.into(), meta.endpoint.port());
                            let packet = &mut rx_buf[..n];

                            match sessions.recv_message(src, packet) {
                                Ok(rustyguard_core::Message::Write(reply)) => {
                                    // Reply is a slice of our buffer – send it back.
                                    let dest = meta.endpoint;
                                    socket.send_to(reply, dest).await.ok();
                                }
                                Ok(rustyguard_core::Message::Read(peer_id, plaintext)) => {
                                    // plaintext points into `packet`; it’s the decrypted payload.
                                    info!("WG recv {} bytes", plaintext.len());
                                    // Here you can process the data (e.g. send it over a channel).
                                }
                                Ok(rustyguard_core::Message::HandshakeComplete(encrypter)) => {
                                    info!("WG handshake complete");
                                    if !handshake_done_signalled {
                                        WG_READY.signal(());
                                        handshake_done_signalled = true;
                                    }
                                    // If you want to send the first keep‑alive immediately:
                                    // (optional)
                                }
                                Ok(rustyguard_core::Message::Noop) => {}
                                Err(e) => {
                                    warn!("WG recv error: {:?}", e);
                                }
                            }
                        }
                        Err(embassy_net::TryError::WouldBlock) => {} // no data
                        Err(e) => {
                            warn!("UDP recv error: {:?}", e);
                        }
                    }

                    // 3. Send any pending user data.
                    if handshake_done_signalled {
                        if let Ok(data) = WG_SEND.try_receive() {
                            let peer_id = peer_id; // captured from outer scope
                            let mut buf = vec![0u8; 16 + data.len() + 16]; // header+payload+tag
                            buf[16..16+data.len()].copy_from_slice(&data);

                            match sessions.send_message(peer_id, &mut buf[16..16+data.len()]) {
                                Ok(rustyguard_core::SendMessage::Data(_, metadata)) => {
                                    metadata.frame_in_place(&mut buf[..16+data.len()+16]);
                                    let dest = IpEndpoint::new(ip.into(), port);
                                    socket.send_to(&buf[..16+data.len()+16], dest).await.ok();
                                }
                                Ok(rustyguard_core::SendMessage::Maintenance(m)) => {
                                    // shouldn’t happen if handshake is done, but send it anyway
                                    let dest = m.to();
                                    socket.send_to(m.data(), IpEndpoint::new(dest.ip().into(), dest.port())).await.ok();
                                }
                                Err(_) => {
                                    warn!("Failed to encrypt outgoing packet");
                                }
                            }
                        }
                    }

                    // Yield for ~10 ms so other tasks get CPU time.
                    embassy_time::Timer::after_millis(10).await;
                }

                // Clean up – drop socket implicitly when it goes out of scope.
                if disabled {
                    info!("WireGuard stopped, returning to idle");
                } else {
                    info!("WireGuard loop exited unexpectedly, restarting in 5s");
                    embassy_time::Timer::after_secs(5).await;
                }
            }

            WgCommand::Disable => {
                debug!("Disable received while idle – ignoring");
            }
        }
    }
}

/// Convert embassy-time monotonic microseconds to Tai64N.
fn tai64_now() -> Tai64N {
    let micros = embassy_time::Instant::now().as_micros();
    let secs = (micros / 1_000_000) as u64;
    let subsec_nanos = ((micros % 1_000_000) * 1000) as u32;
    // Adjust epoch if needed; here we assume zero = UNIX epoch.
    Tai64N::from_secs(secs) + core::time::Duration::from_nanos(subsec_nanos as u64)
}
