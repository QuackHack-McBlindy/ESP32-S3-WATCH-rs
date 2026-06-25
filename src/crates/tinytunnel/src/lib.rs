use defmt::{info, debug, warn, error};
use embassy_net::udp::{UdpSocket, PacketMetadata};
use embassy_net::{IpEndpoint, IpListenEndpoint};
use embassy_time::{Duration, Instant, Timer};
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_sync::once_lock::OnceLock;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

use sunset::{Config, Sessions, StaticPrivateKey, StaticPeerConfig, PublicKey};
use sunset::Message;
use tai64::Tai64N;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;

use alloc::vec::Vec;

// ── Channel definitions (keep yours) ──
pub static WG_CMD: Channel<CriticalSectionRawMutex, WgCommand, 1> = Channel::new();
pub static WG_SEND: Channel<CriticalSectionRawMutex, Vec<u8>, 4> = Channel::new();
pub static WG_READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[derive(defmt::Format)]
pub enum WgCommand {
    Enable,
    Disable,
}

// ── Key loading (keep your const hex parser) ──
const EMBEDDED_WG_KEY: [u8; 32] = match crate::state::WG_PRIVATE_KEY_HEX {
    Some(hex) => hex_to_bytes(hex),
    None => [0u8; 32],
};

const SERVER_PUBKEY: [u8; 32] = match state::WG_SERVER_PUB_KEY {
    Some(hex) => hex_to_bytes(hex),
    None => [0u8; 32],   // fallback (or compile_error!) – fill with real value
};

// ── Helper: embassy time → Tai64N ──
fn tai64_now() -> Tai64N {
    let micros = Instant::now().as_micros();
    let secs = (micros / 1_000_000) as u64;
    let nanos = ((micros % 1_000_000) * 1000) as u32;
    Tai64N::from_unix(secs as i64) + core::time::Duration::from_nanos(nanos as u64)
}

#[embassy_executor::task]
pub async fn wireguard_tsk(
    stack: &'static embassy_net::Stack<'static>,
    endpoint: &'static str,
    port: u16,
) {
    // Parse server IP
    let server_ip: core::net::Ipv4Addr = match endpoint.parse() {
        Ok(ip) => ip,
        Err(_) => {
            error!("Invalid WireGuard endpoint IP: {}", endpoint);
            return;
        }
    };
    let server_addr = core::net::SocketAddrV4::new(server_ip, port);

    // ── Main wait loop ──
    loop {
        info!("🛡️  Waiting for WG enable...");
        let cmd = WG_CMD.receive().await;

        match cmd {
            WgCommand::Enable => {
                info!("🛡️  Starting WireGuard...");

                // Build configuration
                let static_sk = StaticPrivateKey(EMBEDDED_WG_KEY);
                let mut config = Config::new(static_sk);

                let server_pk = PublicKey(SERVER_PUBKEY);
                let peer_config = StaticPeerConfig::new(
                    server_pk,
                    None,               // no pre-shared key
                    Some(server_addr),  // endpoint
                );
                let peer_id = config.insert_peer(peer_config);

                // RNG – replace with real TRNG seed!
                let mut rng = ChaCha12Rng::seed_from_u64(0xDEAD_BEEF_1234_5678);
                let mut sessions = Sessions::new(config, &mut rng);

                // ── UDP socket ──
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
                // Bind to any available port (source port not critical)
                socket.bind(IpListenEndpoint { addr: None, port: 0 }).unwrap();

                let mut disabled = false;
                let mut handshake_done = false;

                loop {
                    // Check disable command
                    if let Ok(WgCommand::Disable) = WG_CMD.try_receive() {
                        disabled = true;
                        info!("WireGuard disabled");
                        break;
                    }

                    // 1. Timer tick – ~10 Hz is enough
                    let now = tai64_now();
                    if let Some(maintenance) = sessions.turn(now, &mut rng) {
                        let dest = IpEndpoint::new(
                            maintenance.to().ip().into(),
                            maintenance.to().port(),
                        );
                        socket.send_to(maintenance.data(), dest).await.ok();
                    }

                    // 2. Receive UDP
                    match socket.try_recv_from(&mut rx_buf) {
                        Ok((n, meta)) => {
                            let src = core::net::SocketAddr::new(
                                meta.endpoint.addr.into(),
                                meta.endpoint.port(),
                            );
                            let packet = &rx_buf[..n];

                            match sessions.recv_message(src, packet) {
                                Ok(Message::Write(reply)) => {
                                    // Send reply back
                                    let dest = meta.endpoint;
                                    socket.send_to(reply, dest).await.ok();
                                }
                                Ok(Message::Read(peer, plaintext)) => {
                                    info!("WG recv {} bytes from peer {:?}", plaintext.len(), peer);
                                    // Push plaintext to your application
                                }
                                Ok(Message::HandshakeComplete(_peer)) => {
                                    info!("WG handshake complete");
                                    if !handshake_done {
                                        WG_READY.signal(());
                                        handshake_done = true;
                                    }
                                }
                                Ok(Message::Noop) => {}
                                Err(e) => warn!("WG recv error: {:?}", e),
                            }
                        }
                        Err(embassy_net::TryError::WouldBlock) => {}
                        Err(e) => warn!("UDP recv error: {:?}", e),
                    }

                    // 3. Send pending application data
                    if handshake_done {
                        if let Ok(data) = WG_SEND.try_receive() {
                            let mut buf = vec![0u8; 16 + data.len() + 16]; // header + payload + tag
                            buf[16..16 + data.len()].copy_from_slice(&data);

                            match sessions.send_message(peer_id, &mut buf[16..16 + data.len()]) {
                                Ok(sunset::SendMessage::Data(_, metadata)) => {
                                    metadata.frame_in_place(&mut buf[..16 + data.len() + 16]);
                                    let dest = IpEndpoint::new(server_ip.into(), port);
                                    socket.send_to(&buf[..16 + data.len() + 16], dest).await.ok();
                                }
                                Ok(sunset::SendMessage::Maintenance(m)) => {
                                    // This shouldn't happen after handshake, but send anyway
                                    let dest = IpEndpoint::new(m.to().ip().into(), m.to().port());
                                    socket.send_to(m.data(), dest).await.ok();
                                }
                                Err(e) => warn!("WG send error: {:?}", e),
                            }
                        }
                    }

                    Timer::after_millis(10).await;
                }

                // socket dropped here, WG paused
                if disabled {
                    info!("WireGuard stopped, returning to idle");
                } else {
                    info!("WireGuard loop exited unexpectedly, restarting in 5s");
                    Timer::after_secs(5).await;
                }
            }
            WgCommand::Disable => debug!("Disable while idle – ignoring"),
        }
    }
}
