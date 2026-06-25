// BASE/WIREGUARD
// VPN CLIENT USING EMBASSY-NET
// IMPLEMENTING THE WireGuard™ PROTOCOL

// ───────────────────────────────────────────────────────────────────────
// DEBUGGING: (EVERYTHING SENT FROM THE ESP WILL HAVE THE PORT 23456)
// RUN THE COMMAND BELOW ON THE SERVER TO SEE IF ANY HANDSHAKE WAS RECIEVED
//     sudo wg show wg0
// VIEW PACKETS ON SERVER
//    sudo tcpdump -i any udp port 51820
// VIEW WIREGUARD KERNEL LOGS ON SERVER
//    sudo dmesg -w | grep wireguard
// ───────────────────────────────────────────────────────────────────────

#![no_std]
#![forbid(unsafe_code)]
#![allow(unused)]

extern crate alloc;

use alloc::vec::Vec;
use embassy_net_driver::LinkState;

// ───────────────────────────────────────────────────────────────────────
// CONFIGURATION FROM wg-client.conf
pub const WG_CONF_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.wg-client.conf"
));

#[derive(Debug)]
pub struct WgConfig {
    pub private_key: [u8; 32],
    pub peer_public_key: [u8; 32],
    pub endpoint_host: &'static str,
    pub endpoint_port: u16,
    pub address: embassy_net::Ipv4Cidr,
    pub dns: Option<embassy_net::Ipv4Address>,
    pub persistent_keepalive: u16,
}

pub fn parse_wg_conf() -> Option<WgConfig> {
    let text = core::str::from_utf8(WG_CONF_BYTES).ok()?;
    let mut private_key = None;
    let mut address = None;
    let mut dns = None;
    let mut peer_public_key = None;
    let mut endpoint_host = None;
    let mut endpoint_port = None;
    let mut persistent_keepalive = 0u16;
    let mut current_section = Section::None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len()-1].trim().to_lowercase();
            match section.as_str() {
                "interface" => current_section = Section::Interface,
                "peer" => current_section = Section::Peer,
                _ => current_section = Section::None,
            }
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim();
            match current_section {
                Section::Interface => match key.as_str() {
                    "privatekey" => private_key = Some(value),
                    "address" => address = Some(value),
                    "dns" => dns = Some(value),
                    _ => {}
                },
                Section::Peer => match key.as_str() {
                    "publickey" => peer_public_key = Some(value),
                    "endpoint" => {
                        if let Some((host, port_str)) = value.rsplit_once(':') {
                            endpoint_host = Some(host);
                            endpoint_port = port_str.parse().ok();
                        }
                    },
                    "persistentkeepalive" => {
                        persistent_keepalive = value.parse().unwrap_or(0);
                    },
                    _ => {}
                },
                Section::None => {}
            }
        }
    }


    let private_key = base64_decode(private_key?)?;
    let peer_public_key = base64_decode(peer_public_key?)?;


    let address_cidr: embassy_net::Ipv4Cidr = {
        let addr_str = address?;
        let (ip_str, prefix_str) = addr_str.split_once('/')?;
        let ip: [u8; 4] = {
            let mut parts = ip_str.splitn(4, '.');
            let mut arr = [0u8; 4];
            for (i, p) in parts.enumerate() {
                if i >= 4 { return None; }
                arr[i] = p.parse().ok()?;
            }
            arr
        };
        let prefix: u8 = prefix_str.parse().ok()?;
        embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::from_octets(ip), prefix)
    };

    let dns = dns.and_then(|s| {
        let parts: Vec<&str> = s.splitn(4, '.').collect();
        if parts.len() != 4 { return None; }
        let mut arr = [0u8; 4];
        for (i, p) in parts.iter().enumerate() {
            arr[i] = p.parse().ok()?;
        }
        Some(embassy_net::Ipv4Address::from_octets(arr))
    });

    let endpoint_host = endpoint_host?;
    let endpoint_port = endpoint_port?;

    Some(WgConfig {
        private_key,
        peer_public_key,
        endpoint_host,
        endpoint_port,
        address: address_cidr,
        dns,
        persistent_keepalive,
    })
}

enum Section {
    None,
    Interface,
    Peer,
}

fn base64_decode(input: &str) -> Option<[u8; 32]> {
    if input.len() != 44 { return None; }
    let mut out = [0u8; 32];
    let mut buf = 0u32;
    let mut bits = 0u8;
    let mut byte_idx = 0;
    for &c in input.as_bytes().iter() {
        let val = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out[byte_idx] = (buf >> bits) as u8;
            byte_idx += 1;
            buf &= (1 << bits) - 1;
        }
    }
    if byte_idx == 32 { Some(out) } else { None }
}

// ───────────────────────────────────────────────────────────────────────

macro_rules! mac_protected {
    ($i:ident) => {
        impl HasMac for $i {
            fn verify_mac1(&self, mac1_key: &Key) -> Result<(), CryptoError> {
                let actual = self.compute_mac1(mac1_key);
                if constant_time_eq(&actual, self.get_mac1()) {
                    Ok(())
                } else {
                    Err(CryptoError::Rejected)
                }
            }

            fn verify_mac2(&self, cookie: &Cookie) -> Result<(), CryptoError> {
                let actual = self.compute_mac2(cookie);
                if constant_time_eq(&actual, self.get_mac2()) {
                    Ok(())
                } else {
                    Err(CryptoError::Rejected)
                }
            }

            fn compute_mac1(&self, mac1_key: &Key) -> Mac {
                let offset = core::mem::offset_of!($i, mac1);
                Core::blake2s_mac(mac1_key, &zerocopy::IntoBytes::as_bytes(self)[..offset])
            }

            fn compute_mac2(&self, cookie: &Cookie) -> Mac {
                let offset = core::mem::offset_of!($i, mac2);
                Core::blake2s_mac(&cookie.0, &zerocopy::IntoBytes::as_bytes(self)[..offset])
            }

            fn get_mac1(&self) -> &Mac { &self.mac1 }
            fn get_mac2(&self) -> &Mac { &self.mac2 }
        }
    };
}


// ───────────────────────────────────────────────────────────────────────
// EMBASSY CHANNELS
pub static WG_CMD: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, WgCommand, 1> = embassy_sync::channel::Channel::new();
pub static WG_SEND: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, alloc::vec::Vec<u8>, 4> = embassy_sync::channel::Channel::new();
pub static WG_READY: embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, ()> = embassy_sync::signal::Signal::new();


use embassy_net_driver_channel as ch;

// MTU = 1420 (LEAVE ROOM FOR WG HEADER), 4 RX SLOTS, 4 TX SLOTS
pub type WgChannelState = ch::State<1420, 2, 2  >;

// HOLDS CHANNEL BUFFERS
use static_cell::StaticCell;
static WG_CHANNEL_STATE: StaticCell<WgChannelState> = StaticCell::new();

// INIT THE WG CHANNEL STATE & RETURN THE TWO ENDS:
// - `Device` - USE WITH `embassy_net::Stack::new()` TO CREATE `vpn_stack`
// - `Runner` - USE INSIDE THE WIREGUARD I/O TASK
pub fn init_wg_channel() -> (ch::Device<'static, 1420>, ch::Runner<'static, 1420>) {
    let state = WG_CHANNEL_STATE.init(WgChannelState::new());
    let (runner, device) = ch::new(state, embassy_net_driver::HardwareAddress::Ip);
    (device, runner)   // swap
}

// TRUE WHEN VPN IS ENABLED & TRANSPORT SESSION IS ACTIVE
pub static VPN_ACTIVE: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

// RETURNS THE STACK THAT SHOULD BE USED FOR SOCKETS.
pub fn active_stack<'a>(
    wifi: &'a embassy_net::Stack<'static>,
    vpn: &'a embassy_net::Stack<'static>,
) -> &'a embassy_net::Stack<'static> {
    if VPN_ACTIVE.load(core::sync::atomic::Ordering::Relaxed) {
        vpn
    } else {
        wifi
    }
}

// ───────────────────────────────────────────────────────────────────────
// COMMAND ENUM
#[derive(defmt::Format)]
pub enum WgCommand { Enable, Disable }


// ───────────────────────────────────────────────────────────────────────
// CONSTANTS
const REKEY_AFTER_MESSAGES: u64 = 1 << 60;
const REJECT_AFTER_MESSAGES: u64 = u64::MAX - (1 << 13);
const REKEY_AFTER_TIME: core::time::Duration = core::time::Duration::from_secs(120);
const REJECT_AFTER_TIME: core::time::Duration = core::time::Duration::from_secs(180);
const REKEY_ATTEMPT_TIME: core::time::Duration = core::time::Duration::from_secs(90);
const REKEY_TIMEOUT: core::time::Duration = core::time::Duration::from_secs(5);
const KEEPALIVE_TIMEOUT: core::time::Duration = core::time::Duration::from_secs(10);


// ───────────────────────────────────────────────────────────────────────
// CRYPTO TYPES & PRIMITIVES
pub type Key = [u8; 32];
pub type Mac = [u8; 16];

#[derive(Debug)]
pub enum CryptoError {
    KeyExchangeError,
    DecryptionError,
    Rejected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StaticPrivateKey(pub [u8; 32]);
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 32]);

#[derive(Clone)]
pub struct EphemeralPrivateKey(StaticPrivateKey);

impl EphemeralPrivateKey {
    pub fn generate(rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) -> Self {
        let mut b = [0u8; 32];
        rand_core::RngCore::fill_bytes(rng, &mut b);
        EphemeralPrivateKey(StaticPrivateKey(b))
    }
}


// ───────────────────────────────────────────────────────────────────────
// NOISE IKPSK2 CONSTANTS
const CONSTRUCTION_HASH: [u8; 32] = [
    96, 226, 109, 174, 243, 39, 239, 192, 46, 195, 53, 226, 160, 37, 210, 208, 22, 235, 66, 6, 248,
    114, 119, 245, 45, 56, 209, 152, 139, 120, 205, 54,
];
const IDENTIFIER_HASH: [u8; 32] = [
    34, 17, 179, 97, 8, 26, 197, 102, 105, 18, 67, 219, 69, 138, 213, 50, 45, 156, 108, 102, 34,
    147, 232, 183, 14, 225, 156, 101, 186, 7, 158, 243,
];
const LABEL_MAC1: [u8; 8] = *b"mac1----";
const LABEL_COOKIE: [u8; 8] = *b"cookie--";

fn nonce(counter: u64) -> [u8; 12] {
    let mut n = [0; 12];
    n[4..].copy_from_slice(&u64::to_le_bytes(counter));
    n
}


// ───────────────────────────────────────────────────────────────────────
// CRYPTOGRAPHIC PRIMITIVES TRAIT AND CORE IMPLEMENTATION
pub trait CryptoPrimitives {
    fn blake2s_hash(left: &[u8], right: &[u8]) -> Key;
    fn blake2s_mac(key: &[u8], msg: &[u8]) -> Mac;
    fn hmac_blake2s(key: &Key, msg: &[u8]) -> Key;
    fn hkdf_blake2s<const N: usize>(key: &mut Key, msg: &[u8], output: &mut [Key; N]);
    fn x25519(secret: &StaticPrivateKey, public: &PublicKey) -> Result<Key, CryptoError>;
    fn x25519_pubkey(secret: &StaticPrivateKey) -> PublicKey;
    fn chacha20poly1305_enc(key: &Key, nonce: &[u8; 12], aad: &[u8], payload: &mut [u8], tag: &mut [u8; 16]);
    fn chacha20poly1305_dec(key: &Key, nonce: &[u8; 12], aad: &[u8], payload: &mut [u8], tag: &[u8; 16]) -> Result<(), CryptoError>;
    fn xchacha20poly1305_enc(key: &Key, nonce: &[u8; 24], aad: &[u8], payload: &mut [u8], tag: &mut [u8; 16]);
    fn xchacha20poly1305_dec(key: &Key, nonce: &[u8; 24], aad: &[u8], payload: &mut [u8], tag: &[u8; 16]) -> Result<(), CryptoError>;
}

pub struct Core;

impl CryptoPrimitives for Core {
    fn blake2s_hash(left: &[u8], right: &[u8]) -> Key {
        let mut mac = blake2s_simd::State::new();
        mac.update(left);
        mac.update(right);
        *mac.finalize().as_array()
    }

    fn blake2s_mac(key: &[u8], msg: &[u8]) -> Mac {
        blake2s_simd::Params::new()
            .hash_length(16)
            .key(key)
            .hash(msg)
            .as_bytes()
            .try_into()
            .unwrap()
    }

    fn hmac_blake2s(key: &Key, msg: &[u8]) -> Key {
        let (ipad_key, opad_key) = get_pad(key);
        hmac_inner(&ipad_key, &opad_key, [msg])
    }

    fn hkdf_blake2s<const N: usize>(key: &mut Key, msg: &[u8], output: &mut [Key; N]) {
        assert!(N < 255);
        let (ipad_key, opad_key) = get_pad(&Self::hmac_blake2s(key, msg));
        let mut ti = hmac_inner(&ipad_key, &opad_key, [&[1u8]]);
        *key = ti;
        for i in 0..N as u8 {
            ti = hmac_inner(&ipad_key, &opad_key, [&ti[..], &[i + 2]]);
            output[i as usize] = ti;
        }
    }

    fn x25519(secret: &StaticPrivateKey, public: &PublicKey) -> Result<Key, CryptoError> {
        let ss = x25519_dalek::StaticSecret::from(secret.0);
        let pk = x25519_dalek::PublicKey::from(public.0);
        let shared = ss.diffie_hellman(&pk);
        let key = shared.to_bytes();

        // REJECT ALL-ZERO SHARED SECRET (LOW-ORDER POINT ATTACK)
        if key.iter().all(|&b| b == 0) {
            return Err(CryptoError::Rejected);
        }

        Ok(key)
    }

    fn x25519_pubkey(secret: &StaticPrivateKey) -> PublicKey {
        let ss = x25519_dalek::StaticSecret::from(secret.0);
        let pk = x25519_dalek::PublicKey::from(&ss);
        PublicKey(pk.to_bytes())
    }

    fn chacha20poly1305_enc(key: &Key, nonce: &[u8; 12], aad: &[u8], payload: &mut [u8], tag: &mut [u8; 16]) {
        let cipher = <chacha20poly1305::ChaCha20Poly1305 as chacha20poly1305::KeyInit>::new(key.into());
        let n = chacha20poly1305::aead::generic_array::GenericArray::from_slice(nonce);
        let t = <chacha20poly1305::ChaCha20Poly1305 as chacha20poly1305::aead::AeadInPlace>::encrypt_in_place_detached(
            &cipher, n, aad, payload,
        ).expect("ChaCha20Poly1305 encrypt failed");
        tag.copy_from_slice(&t);
    }

    fn chacha20poly1305_dec(key: &Key, nonce: &[u8; 12], aad: &[u8], payload: &mut [u8], tag: &[u8; 16]) -> Result<(), CryptoError> {
        let cipher = <chacha20poly1305::ChaCha20Poly1305 as chacha20poly1305::KeyInit>::new(key.into());
        let n = chacha20poly1305::aead::generic_array::GenericArray::from_slice(nonce);
        let t = chacha20poly1305::aead::generic_array::GenericArray::from_slice(tag);
        <chacha20poly1305::ChaCha20Poly1305 as chacha20poly1305::aead::AeadInPlace>::decrypt_in_place_detached(
            &cipher, n, aad, payload, t,
        ).map_err(|_| CryptoError::DecryptionError)
    }

    fn xchacha20poly1305_enc(key: &Key, nonce: &[u8; 24], aad: &[u8], payload: &mut [u8], tag: &mut [u8; 16]) {
        let cipher = <chacha20poly1305::XChaCha20Poly1305 as chacha20poly1305::KeyInit>::new(key.into());
        let n = chacha20poly1305::aead::generic_array::GenericArray::from_slice(nonce);
        let t = <chacha20poly1305::XChaCha20Poly1305 as chacha20poly1305::aead::AeadInPlace>::encrypt_in_place_detached(
            &cipher, n, aad, payload,
        ).expect("XChaCha20Poly1305 encrypt failed");
        tag.copy_from_slice(&t);
    }

    fn xchacha20poly1305_dec(key: &Key, nonce: &[u8; 24], aad: &[u8], payload: &mut [u8], tag: &[u8; 16]) -> Result<(), CryptoError> {
        let cipher = <chacha20poly1305::XChaCha20Poly1305 as chacha20poly1305::KeyInit>::new(key.into());
        let n = chacha20poly1305::aead::generic_array::GenericArray::from_slice(nonce);
        let t = chacha20poly1305::aead::generic_array::GenericArray::from_slice(tag);
        <chacha20poly1305::XChaCha20Poly1305 as chacha20poly1305::aead::AeadInPlace>::decrypt_in_place_detached(
            &cipher, n, aad, payload, t,
        ).map_err(|_| CryptoError::DecryptionError)
    }
}


// ───────────────────────────────────────────────────────────────────────
// HMAC HELPERS AND KEY DERIVATION
fn get_der_key(key: &Key) -> [u8; 64] {
    let mut der_key = [0u8; 64];
    der_key[..key.len()].copy_from_slice(key);
    der_key
}

fn get_pad(key: &Key) -> ([u8; 64], [u8; 64]) {
    let der_key = get_der_key(key);
    let mut ipad_key = der_key;
    for b in ipad_key.iter_mut() { *b ^= 0x36; }
    let mut opad_key = der_key;
    for b in opad_key.iter_mut() { *b ^= 0x5C; }
    (ipad_key, opad_key)
}

fn hmac_inner<const M: usize>(ipad_key: &[u8; 64], opad_key: &[u8; 64], msg: [&[u8]; M]) -> Key {
    let mut digest = blake2s_simd::State::new();
    digest.update(ipad_key);
    for m in msg { digest.update(m); }
    let mut h = blake2s_simd::State::new();
    h.update(opad_key);
    h.update(digest.finalize().as_bytes());
    *h.finalize().as_array()
}

pub fn mac1_key(spk: &[u8]) -> Key { Core::blake2s_hash(&LABEL_MAC1, spk) }
pub fn cookie_key(spk: &[u8]) -> Key { Core::blake2s_hash(&LABEL_COOKIE, spk) }

// ───────────────────────────────────────────────────────────────────────
// COOKIE STATE AND ENCRYPTED COOKIE
#[derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
pub struct CookieState {
    key: Key,
}

impl CookieState {
    pub fn new(rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) -> Self {
        let mut key = Key::default();
        rand_core::RngCore::fill_bytes(rng, &mut key);
        Self { key }
    }

    pub fn generate(&mut self, rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) {
        rand_core::RngCore::fill_bytes(rng, &mut self.key);
    }

    pub fn new_cookie(&self, addr: core::net::SocketAddr) -> Cookie {
        let mut a = [0; 18];
        match addr.ip() {
            core::net::IpAddr::V4(ipv4) => a[..4].copy_from_slice(&ipv4.octets()),
            core::net::IpAddr::V6(ipv6) => a[..16].copy_from_slice(&ipv6.octets()),
        }
        a[16..].copy_from_slice(&addr.port().to_le_bytes());
        Cookie(Core::blake2s_mac(&self.key, &a))
    }
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
pub struct Cookie(pub Mac);

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct EncryptedCookie {
    pub msg: Cookie,
    pub tag: Tag,
}

pub fn decrypt_cookie<'c>(cookie: &'c mut EncryptedCookie, key: &Key, nonce: &[u8; 24], aad: &[u8]) -> Result<&'c mut Cookie, CryptoError> {
    Core::xchacha20poly1305_dec(key, nonce, aad, &mut cookie.msg.0, &cookie.tag.0)?;
    Ok(&mut cookie.msg)
}

pub fn encrypt_cookie(cookie: Cookie, key: &Key, nonce: &[u8; 24], aad: &[u8]) -> EncryptedCookie {
    let mut out = EncryptedCookie { msg: cookie, tag: Tag([0; 16]) };
    Core::xchacha20poly1305_enc(key, nonce, aad, &mut out.msg.0, &mut out.tag.0);
    out
}


// ───────────────────────────────────────────────────────────────────────
// HANDSHAKE STATE
#[derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop, Clone)]
pub struct HandshakeState {
    hash: [u8; 32],
    chain: Key,
}

impl Default for HandshakeState {
    fn default() -> Self {
        Self { chain: CONSTRUCTION_HASH, hash: IDENTIFIER_HASH }
    }
}

impl HandshakeState {
    pub fn mix_chain<C: CryptoPrimitives>(&mut self, b: &[u8]) {
        C::hkdf_blake2s(&mut self.chain, b, &mut []);
    }

    pub fn mix_dh<C: CryptoPrimitives>(&mut self, sk: &StaticPrivateKey, pk: &PublicKey) -> Result<(), CryptoError> {
        let shared_secret = C::x25519(sk, pk)?;
        C::hkdf_blake2s(&mut self.chain, &shared_secret, &mut []);
        Ok(())
    }

    pub fn mix_key_dh<C: CryptoPrimitives>(&mut self, sk: &StaticPrivateKey, pk: &PublicKey) -> Result<Key, CryptoError> {
        let shared_secret = C::x25519(sk, pk)?;
        Ok(self.mix_key::<C>(&shared_secret))
    }

    pub fn mix_edh<C: CryptoPrimitives>(&mut self, sk: &EphemeralPrivateKey, pk: &PublicKey) -> Result<(), CryptoError> {
        self.mix_dh::<C>(&sk.0, pk)
    }

    pub fn mix_key_edh<C: CryptoPrimitives>(&mut self, sk: &EphemeralPrivateKey, pk: &PublicKey) -> Result<Key, CryptoError> {
        self.mix_key_dh::<C>(&sk.0, pk)
    }

    fn mix_key<C: CryptoPrimitives>(&mut self, b: &[u8]) -> Key {
        let mut k = Key::default();
        C::hkdf_blake2s(&mut self.chain, b, core::array::from_mut(&mut k));
        k
    }

    pub fn mix_key_and_hash<C: CryptoPrimitives>(&mut self, b: &[u8]) -> Key {
        let mut tk = [Key::default(); 2];
        C::hkdf_blake2s(&mut self.chain, b, &mut tk);
        self.mix_hash::<C>(&tk[0]);
        tk[1]
    }

    pub fn mix_hash<C: CryptoPrimitives>(&mut self, b: &[u8]) {
        self.hash = C::blake2s_hash(&self.hash, b);
    }

    pub fn split<C: CryptoPrimitives>(&mut self, initiator: bool) -> (EncryptionKey, DecryptionKey) {
        let mut k2 = Key::default();
        C::hkdf_blake2s(&mut self.chain, &[], core::array::from_mut(&mut k2));
        let k1 = self.chain;
        zeroize::Zeroize::zeroize(self);
        if initiator {
            (EncryptionKey::new(k1), DecryptionKey::new(k2))
        } else {
            (EncryptionKey::new(k2), DecryptionKey::new(k1))
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// WIREGUARD PACKET TYPES
pub const MSG_FIRST: u32 = 1;
pub const MSG_SECOND: u32 = 2;
pub const MSG_COOKIE: u32 = 3;
pub const MSG_DATA: u32 = 4;

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct HandshakeInit {
    pub _type: zerocopy::little_endian::U32,
    pub sender: zerocopy::little_endian::U32,
    pub ephemeral_key: [u8; 32],
    pub static_key: EncryptedPublicKey,
    pub timestamp: EncryptedTimestamp,
    pub mac1: [u8; 16],
    pub mac2: [u8; 16],
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct HandshakeResp {
    pub _type: zerocopy::little_endian::U32,
    pub sender: zerocopy::little_endian::U32,
    pub receiver: zerocopy::little_endian::U32,
    pub ephemeral_key: [u8; 32],
    pub empty: EncryptedEmpty,
    pub mac1: [u8; 16],
    pub mac2: [u8; 16],
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct CookieMessage {
    pub _type: zerocopy::little_endian::U32,
    pub receiver: zerocopy::little_endian::U32,
    pub nonce: [u8; 24],
    pub cookie: EncryptedCookie,
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct DataHeader {
    pub _type: zerocopy::little_endian::U32,
    pub receiver: zerocopy::little_endian::U32,
    pub counter: zerocopy::little_endian::U64,
}

impl DataHeader {
    /// SPLIT A MUTABLE BYTE SLICE INTO A DATAHEADER AND THE REMAINING PAYLOAD+TAG.
    pub fn message_mut_from(msg: &mut [u8]) -> Option<(&mut Self, &mut [u8])> {
        let (header, rest) = zerocopy::Ref::<_, DataHeader>::from_prefix(msg).ok()?;
        Some((zerocopy::Ref::into_mut(header), rest))
    }
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct Tag(pub [u8; 16]);

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct EncryptedEmpty {
    pub msg: [u8; 0],
    pub tag: Tag,
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct EncryptedTimestamp {
    pub msg: [u8; 12],
    pub tag: Tag,
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
pub struct EncryptedPublicKey {
    pub msg: [u8; 32],
    pub tag: Tag,
}


// ───────────────────────────────────────────────────────────────────────
// ENCRYPTED TRAIT AND IMPLEMENTATIONS
pub trait Encrypted<const N: usize> {
    fn decrypt_and_hash<C: CryptoPrimitives>(&mut self, state: &mut HandshakeState, key: &Key) -> Result<&mut [u8; N], CryptoError>;
    fn encrypt_and_hash<C: CryptoPrimitives>(msg: [u8; N], state: &mut HandshakeState, key: &Key) -> Self;
}

macro_rules! encrypted {
    ($i:ident, $n:literal) => {
        impl Encrypted<$n> for $i {
            fn decrypt_and_hash<C: CryptoPrimitives>(&mut self, state: &mut HandshakeState, key: &Key) -> Result<&mut [u8; $n], CryptoError> {
                let aad = state.hash;
                state.mix_hash::<C>(zerocopy::IntoBytes::as_bytes(self));
                C::chacha20poly1305_dec(key, &nonce(0), &aad, &mut self.msg, &self.tag.0)?;
                Ok(&mut self.msg)
            }

            fn encrypt_and_hash<C: CryptoPrimitives>(msg: [u8; $n], state: &mut HandshakeState, key: &Key) -> Self {
                let aad = state.hash;
                let mut out = Self { msg, tag: Tag([0; 16]) };
                C::chacha20poly1305_enc(key, &nonce(0), &aad, &mut out.msg, &mut out.tag.0);
                state.mix_hash::<C>(zerocopy::IntoBytes::as_bytes(&out));
                out
            }
        }
    };
}

encrypted!(EncryptedEmpty, 0);
encrypted!(EncryptedTimestamp, 12);
encrypted!(EncryptedPublicKey, 32);


// ───────────────────────────────────────────────────────────────────────
// HASMAC TRAIT AND IMPLEMENTATIONS
pub trait HasMac: zerocopy::FromBytes + zerocopy::IntoBytes + Sized {
    fn verify<'m>(&'m mut self, config: &StaticInitiatorConfig, overload: bool, cookie: &CookieState, addr: core::net::SocketAddr) -> Result<core::ops::ControlFlow<Cookie, &'m mut Self>, CryptoError> {
        self.verify_mac1(&config.mac1_key)?;
        if overload {
            let c = cookie.new_cookie(addr);
            if self.verify_mac2(&c).is_err() {
                return Ok(core::ops::ControlFlow::Break(c));
            }
        }
        Ok(core::ops::ControlFlow::Continue(self))
    }

    fn verify_mac1(&self, mac1_key: &Key) -> Result<(), CryptoError>;
    fn verify_mac2(&self, cookie: &Cookie) -> Result<(), CryptoError>;
    fn compute_mac1(&self, mac1_key: &Key) -> Mac;
    fn compute_mac2(&self, cookie: &Cookie) -> Mac;
    fn get_mac1(&self) -> &Mac;
    fn get_mac2(&self) -> &Mac;
}

mac_protected!(HandshakeInit);
mac_protected!(HandshakeResp);


// ───────────────────────────────────────────────────────────────────────
// DECRYPTED HANDSHAKE INIT
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
pub struct DecryptedHandshakeInit(pub HandshakeInit);

impl DecryptedHandshakeInit {
    pub fn static_key(&self) -> PublicKey { PublicKey(self.0.static_key.msg) }
    pub fn timestamp(&self) -> &[u8; 12] { &self.0.timestamp.msg }
}


// ───────────────────────────────────────────────────────────────────────
// CONFIGURATION TYPES
pub struct StaticPeerConfig {
    pub key: PublicKey,
    pub preshared_key: Key,
    pub mac1_key: Key,
    pub cookie_key: Key,
    pub endpoint: Option<core::net::SocketAddr>,
}

impl StaticPeerConfig {
    pub fn new(key: PublicKey, preshared_key: Option<Key>, endpoint: Option<core::net::SocketAddr>) -> Self {
        Self {
            mac1_key: mac1_key(&key.0),
            cookie_key: cookie_key(&key.0),
            key,
            preshared_key: preshared_key.unwrap_or_default(),
            endpoint,
        }
    }
}

pub struct StaticInitiatorConfig {
    pub private_key: StaticPrivateKey,
    pub public_key: PublicKey,
    pub mac1_key: Key,
    pub cookie_key: Key,
}

impl StaticInitiatorConfig {
    pub fn new(key: StaticPrivateKey) -> Self {
        let public_key = Core::x25519_pubkey(&key);
        Self {
            mac1_key: mac1_key(&public_key.0),
            cookie_key: cookie_key(&public_key.0),
            public_key,
            private_key: key,
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// HANDSHAKE ENCRYPTION/DECRYPTION FUNCTIONS
pub fn encrypt_handshake_init(
    hs: &mut HandshakeState,
    initiator: &StaticInitiatorConfig,
    peer: &StaticPeerConfig,
    esk_i: &EphemeralPrivateKey,
    now: Tai64N,
    sender: u32,
    cookie: Option<&Cookie>,
) -> Result<HandshakeInit, CryptoError> {
    let epk_i = Core::x25519_pubkey(&esk_i.0);
    hs.mix_hash::<Core>(&peer.key.0);
    hs.mix_hash::<Core>(&epk_i.0);
    hs.mix_chain::<Core>(&epk_i.0);
    let k = hs.mix_key_edh::<Core>(esk_i, &peer.key)?;
    let static_key = EncryptedPublicKey::encrypt_and_hash::<Core>(initiator.public_key.0, hs, &k);
    let k = hs.mix_key_dh::<Core>(&initiator.private_key, &peer.key)?;
    let timestamp = EncryptedTimestamp::encrypt_and_hash::<Core>(now.to_bytes(), hs, &k);
    let mut msg = HandshakeInit {
        _type: zerocopy::little_endian::U32::new(MSG_FIRST),
        sender: zerocopy::little_endian::U32::new(sender),
        ephemeral_key: epk_i.0,
        static_key,
        timestamp,
        mac1: [0; 16],
        mac2: [0; 16],
    };
    msg.mac1 = msg.compute_mac1(&peer.mac1_key);
    if let Some(c) = cookie {
        msg.mac2 = msg.compute_mac2(c);
    }
    Ok(msg)
}

pub fn decrypt_handshake_init<'m>(
    init: &'m mut HandshakeInit,
    hs: &mut HandshakeState,
    receiver: &StaticInitiatorConfig,
) -> Result<&'m mut DecryptedHandshakeInit, CryptoError> {
    hs.mix_hash::<Core>(&receiver.public_key.0);
    hs.mix_hash::<Core>(&init.ephemeral_key);
    hs.mix_chain::<Core>(&init.ephemeral_key);
    let epk_i = PublicKey(init.ephemeral_key);
    let k = hs.mix_key_dh::<Core>(&receiver.private_key, &epk_i)?;
    let spk_i = init.static_key.decrypt_and_hash::<Core>(hs, &k)?;
    let spk_i = PublicKey(*spk_i);
    let k = hs.mix_key_dh::<Core>(&receiver.private_key, &spk_i)?;
    let _ = *init.timestamp.decrypt_and_hash::<Core>(hs, &k)?;
    let d = zerocopy::Ref::<_, DecryptedHandshakeInit>::from_bytes(zerocopy::IntoBytes::as_mut_bytes(init))
        .map_err(|_| CryptoError::KeyExchangeError)?;
    Ok(zerocopy::Ref::into_mut(d))
}

pub fn encrypt_handshake_resp(
    hs: &mut HandshakeState,
    data: &DecryptedHandshakeInit,
    esk_r: &EphemeralPrivateKey,
    peer: &StaticPeerConfig,
    sender: u32,
    cookie: Option<&Cookie>,
) -> Result<HandshakeResp, CryptoError> {
    let epk_r = Core::x25519_pubkey(&esk_r.0);
    hs.mix_chain::<Core>(&epk_r.0);
    hs.mix_hash::<Core>(&epk_r.0);
    let epk_i = PublicKey(data.0.ephemeral_key);
    hs.mix_edh::<Core>(esk_r, &epk_i)?;
    let spk_i = PublicKey(data.0.static_key.msg);
    hs.mix_edh::<Core>(esk_r, &spk_i)?;
    let k = hs.mix_key_and_hash::<Core>(&peer.preshared_key);
    let empty = EncryptedEmpty::encrypt_and_hash::<Core>([], hs, &k);
    let mut msg = HandshakeResp {
        _type: zerocopy::little_endian::U32::new(MSG_SECOND),
        sender: zerocopy::little_endian::U32::new(sender),
        receiver: data.0.sender,
        ephemeral_key: epk_r.0,
        empty,
        mac1: [0; 16],
        mac2: [0; 16],
    };
    msg.mac1 = msg.compute_mac1(&peer.mac1_key);
    if let Some(c) = cookie {
        msg.mac2 = msg.compute_mac2(c);
    }
    Ok(msg)
}

pub fn decrypt_handshake_resp(
    resp: &mut HandshakeResp,
    hs: &mut HandshakeState,
    initiator: &StaticInitiatorConfig,
    peer: &StaticPeerConfig,
    esk_i: &EphemeralPrivateKey,
) -> Result<(), CryptoError> {
    let epk_r = PublicKey(resp.ephemeral_key);
    hs.mix_chain::<Core>(&epk_r.0);
    hs.mix_hash::<Core>(&epk_r.0);
    hs.mix_edh::<Core>(esk_i, &epk_r)?;
    hs.mix_dh::<Core>(&initiator.private_key, &epk_r)?;
    let k = hs.mix_key_and_hash::<Core>(&peer.preshared_key);
    resp.empty.decrypt_and_hash::<Core>(hs, &k)?;
    Ok(())
}


// ───────────────────────────────────────────────────────────────────────
// ENCRYPTION / DECRYPTION KEYS AND ANTI-REPLAY
pub struct EncryptionKey {
    key: Key,
    counter: u64,
}

impl EncryptionKey {
    pub fn new(key: Key) -> Self { Self { key, counter: 0 } }
    pub fn encrypt<C: CryptoPrimitives>(&mut self, payload: &mut [u8]) -> Tag {
        let n = self.counter;
        self.counter += 1;
        let nonce = nonce(n);
        let mut tag = [0; 16];
        C::chacha20poly1305_enc(&self.key, &nonce, &[], payload, &mut tag);
        Tag(tag)
    }
    pub fn counter(&self) -> u64 { self.counter }
}

pub struct DecryptionKey {
    key: Key,
    replay: AntiReplay,
}

impl DecryptionKey {
    pub fn new(key: Key) -> Self { Self { key, replay: AntiReplay::default() } }
    pub fn decrypt<'b, C: CryptoPrimitives>(&mut self, counter: u64, payload_and_tag: &'b mut [u8]) -> Result<&'b mut [u8], CryptoError> {
        if !self.replay.would_accept(counter) { return Err(CryptoError::Rejected); }
        let nonce = nonce(counter);
        let (payload, tag) = payload_and_tag.split_at_mut(payload_and_tag.len() - 16);
        let tag: &mut [u8; 16] = tag.try_into().unwrap();
        C::chacha20poly1305_dec(&self.key, &nonce, &[], payload, tag)?;
        self.replay.mark_seen(counter);
        Ok(payload)
    }
}


// ───────────────────────────────────────────────────────────────────────
// ANTI-REPLAY
const SIZE_OF_WORD: usize = core::mem::size_of::<usize>() * 8;
const REDUNDANT_BIT_SHIFTS: u32 = SIZE_OF_WORD.ilog2();

const BITMAP_BITLEN: usize = 2048;
const BITMAP_LEN: usize = BITMAP_BITLEN / SIZE_OF_WORD;
const BITMAP_INDEX_MASK: usize = BITMAP_LEN - 1;
const BITMAP_LOC_MASK: u64 = (SIZE_OF_WORD as u64) - 1;
pub const WINDOW_SIZE: u64 = (BITMAP_BITLEN - SIZE_OF_WORD) as u64;

pub struct AntiReplay {
    bitmap: [usize; BITMAP_LEN],
    last: u64,
}

impl Default for AntiReplay {
    fn default() -> Self {
        Self {
            bitmap: [0; BITMAP_LEN],
            last: 0,
        }
    }
}

impl AntiReplay {
    pub fn would_accept(&self, n: u64) -> bool {
        if n > self.last {
            return true;
        }
        let d = self.last - n;
        if d >= WINDOW_SIZE {
            return false;
        }
        let index = (n >> REDUNDANT_BIT_SHIFTS) as usize;
        let shift = n & BITMAP_LOC_MASK;
        (self.bitmap[index & BITMAP_INDEX_MASK] >> shift) & 1 == 0
    }

    pub fn mark_seen(&mut self, n: u64) {
        let index = (n >> REDUNDANT_BIT_SHIFTS) as usize;
        let shift = n & BITMAP_LOC_MASK;
        if n > self.last {
            let next_index = ((self.last >> REDUNDANT_BIT_SHIFTS) + 1) as usize;
            if index > next_index && index - next_index > BITMAP_LEN {
                self.bitmap = [0; BITMAP_LEN];
            } else {
                for i in next_index..=index {
                    self.bitmap[i & BITMAP_INDEX_MASK] = 0;
                }
            }
            self.last = n;
        }
        self.bitmap[index & BITMAP_INDEX_MASK] |= 1 << shift;
    }
}


// ───────────────────────────────────────────────────────────────────────
// DOS PROTECTION (COUNT-MIN SKETCH)
const CMS_ROWS: usize = 4;
const CMS_COLS: usize = 1024;

pub struct CountMinSketch {
    table: [[u64; CMS_COLS]; CMS_ROWS],
    hasher: foldhash::fast::FixedState,
}

impl CountMinSketch {
    pub fn with_params(_eps: f64, _delta: f64, rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) -> Self {
        // WE IGNORE EPS/DELTA AND USE A FIXED SIZE; THEY ARE ONLY FOR API COMPATIBILITY.
        let mut seed = [0u8; 32];
        rand_core::RngCore::fill_bytes(rng, &mut seed);
        Self {
            table: [[0; CMS_COLS]; CMS_ROWS],
            hasher: foldhash::fast::FixedState::with_seed(u64::from_le_bytes(seed[..8].try_into().unwrap())),
        }
    }

    /// INCREMENT THE COUNT FOR `KEY` AND RETURN THE NEW ESTIMATE
    pub fn count(&mut self, key: &u64) -> u64 {
        let mut min = u64::MAX;
        // USE DIFFERENT SEEDS FOR EACH ROW BY COMBINING THE BASE HASH WITH THE ROW INDEX.
        for row in 0..CMS_ROWS {
            let idx = self.hash(row, *key) % CMS_COLS;
            self.table[row][idx] = self.table[row][idx].saturating_add(1);
            if self.table[row][idx] < min {
                min = self.table[row][idx];
            }
        }
        min
    }

    /// CLEAR ALL COUNTERS AND RANDOMISE THE HASH SEEDS.
    pub fn reset(&mut self, rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) {
        self.table = [[0; CMS_COLS]; CMS_ROWS];
        let mut seed = [0u8; 32];
        rand_core::RngCore::fill_bytes(rng, &mut seed);
        self.hasher = foldhash::fast::FixedState::with_seed(u64::from_le_bytes(seed[..8].try_into().unwrap()));
    }

    fn hash(&self, row: usize, key: u64) -> usize {
        // MIX THE ROW INDEX INTO THE KEY BEFORE HASHING
        let tweaked = key.wrapping_add((row as u64) * 0x9E3779B97F4A7C15);
        core::hash::BuildHasher::hash_one(&self.hasher, &tweaked) as usize
    }
}


// ───────────────────────────────────────────────────────────────────────
// TIMER SYSTEM
pub struct TimerEntry {
    pub time: Tai64N,
    pub kind: TimerEntryType,
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { other.time.cmp(&self.time) }
}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(other)) }
}
impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool { self.time == other.time }
}
impl Eq for TimerEntry {}

pub enum TimerEntryType {
    InitAttempt { session_id: u32 },
    RekeyAttempt { session_id: u32 },
    Keepalive { session_id: u32 },
    ExpireTransport { session_id: u32 },
    ExpireHandshake { session_id: u32 },
}


// ───────────────────────────────────────────────────────────────────────
// SESSION TYPES
pub struct Session {
    pub peer: PeerId,
    pub started: Tai64N,
    pub sent: Tai64N,
    pub state: SessionState,
    pub keepalive_pending: bool,
}

impl Session {
    fn should_reinit(&self, now: Tai64N, _hs: &SessionHandshake) -> bool {
        (self.sent + REKEY_TIMEOUT < now) && (now < self.started + REKEY_ATTEMPT_TIME)
    }
    fn should_rekey(&self, now: Tai64N, ts: &SessionTransport) -> bool {
        (self.started + REKEY_AFTER_TIME < now) || (ts.encrypt.counter() >= REKEY_AFTER_MESSAGES)
    }
    fn should_keepalive(&self, now: Tai64N, _ts: &SessionTransport) -> bool {
        self.sent + KEEPALIVE_TIMEOUT < now
    }
    fn should_reject(&self, now: Tai64N, ts: &SessionTransport) -> bool {
        self.should_expire(now) || (ts.encrypt.counter() >= REJECT_AFTER_MESSAGES)
    }
    fn should_expire(&self, now: Tai64N) -> bool {
        self.started + REJECT_AFTER_TIME < now
    }
}

#[derive(zeroize::ZeroizeOnDrop)]
pub enum SessionState {
    Handshake(SessionHandshake),
    #[zeroize(skip)]
    Transport(SessionTransport),
}

#[derive(zeroize::Zeroize)]
pub struct SessionHandshake {
    #[zeroize(skip)]
    pub esk_i: EphemeralPrivateKey,
    pub state: HandshakeState,
}

pub struct SessionTransport {
    pub receiver: u32,
    pub encrypt: EncryptionKey,
    pub decrypt: DecryptionKey,
}


// ───────────────────────────────────────────────────────────────────────
// PEER ID, PEER LIST, PEER STATE, CONFIG
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, defmt::Format)]
pub struct PeerId(u32);
impl core::fmt::Debug for PeerId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "PeerId({:08X})", self.0) }
}
impl PeerId { pub const fn sentinel() -> Self { Self(u32::MAX) } }

struct PeerList<P>(alloc::vec::Vec<P>);
impl<P> core::ops::Index<PeerId> for PeerList<P> {
    type Output = P;
    fn index(&self, index: PeerId) -> &Self::Output { &self.0[index.0 as usize] }
}
impl<P> core::ops::IndexMut<PeerId> for PeerList<P> {
    fn index_mut(&mut self, index: PeerId) -> &mut Self::Output { &mut self.0[index.0 as usize] }
}
impl<P> PeerList<P> {
    fn get_mut(&mut self, index: PeerId) -> Option<&mut P> { self.0.get_mut(index.0 as usize) }
}

pub struct PeerState {
    pub endpoint: Option<core::net::SocketAddr>,
    pub latest_ts: Tai64NBytes,
    pub cookie: Option<Cookie>,
    pub last_sent_mac1: Mac,
    pub current_handshake: Option<u32>,
    pub current_transport: Option<u32>,
}

pub struct Config {
    pub static_: StaticInitiatorConfig,
    pub peers_by_pubkey: hashbrown::HashTable<PeerId>,
    pub pubkey_hasher: foldhash::fast::FixedState,
    pub peers: PeerList<StaticPeerConfig>,
}

impl Config {
    pub fn new(private_key: StaticPrivateKey) -> Self {
        Config {
            static_: StaticInitiatorConfig::new(private_key),
            pubkey_hasher: foldhash::fast::FixedState::with_seed(0),
            peers_by_pubkey: hashbrown::HashTable::default(),
            peers: PeerList(alloc::vec::Vec::new()),
        }
    }

    pub fn insert_peer(&mut self, peer: StaticPeerConfig) -> PeerId {
        match self.peers_by_pubkey.entry(
            core::hash::BuildHasher::hash_one(&self.pubkey_hasher, &peer.key.0),
            |&i| self.peers[i].key.0 == peer.key.0,
            |&i| core::hash::BuildHasher::hash_one(&self.pubkey_hasher, &self.peers[i].key.0),
        ) {
            hashbrown::hash_table::Entry::Occupied(o) => {
                let id = *o.get();
                self.peers[id] = peer;
                id
            }
            hashbrown::hash_table::Entry::Vacant(v) => {
                let idx = self.peers.0.len();
                let id = PeerId(idx as u32);
                self.peers.0.push(peer);
                v.insert(id);
                id
            }
        }
    }

    fn get_peer_idx(&self, pk: &PublicKey) -> Option<PeerId> {
        self.peers_by_pubkey
            .find(
                core::hash::BuildHasher::hash_one(&self.pubkey_hasher, &pk.0),
                |&i| self.peers[i].key.0 == pk.0,
            )
            .copied()
    }
}


// ───────────────────────────────────────────────────────────────────────
// DYNAMIC STATE AND SESSION MANAGEMENT
type Tai64NBytes = [u8; 12];
type SessionMap = hashbrown::HashMap<u32, alloc::boxed::Box<Session>, foldhash::fast::FixedState>;

pub struct DynamicState {
    rng: rand_chacha::ChaCha20Rng,
    cookie: CookieState,
    last_reseed: Tai64N,
    now: Tai64N,
    last_rate_reset: Tai64N,
    ip_rate_limit: CountMinSketch,
    peers: PeerList<PeerState>,
    peers_by_session: SessionMap,
    timers: alloc::collections::BinaryHeap<TimerEntry>,
}

impl DynamicState {
    fn new(peers: &PeerList<StaticPeerConfig>, rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) -> Self {
        Self {
            cookie: CookieState::new(rng),
            last_reseed: Tai64N(tai64::Tai64(0), 0),
            now: Tai64N(tai64::Tai64(0), 0),
            last_rate_reset: Tai64N(tai64::Tai64(0), 0),
            ip_rate_limit: CountMinSketch::with_params(10.0 / 20_000.0, 0.01, rng),
            rng: <rand_chacha::ChaCha20Rng as rand_core::SeedableRng>::from_rng(rng).expect("RNG failure"),
            peers: PeerList(peers.0.iter().map(|p| PeerState::new(p.endpoint)).collect()),
            peers_by_session: hashbrown::HashMap::default(),
            timers: alloc::collections::BinaryHeap::new(),
        }
    }
    fn allocate_session_id(&mut self) -> u32 {
        loop {
            let id = rand_core::RngCore::next_u32(&mut self.rng);
            if !self.peers_by_session.contains_key(&id) {
                return id;
            }
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// PUBLIC API TYPES
pub enum SendMessage {
    Maintenance(MaintenanceMsg),
    Data(core::net::SocketAddr, EncryptedMetadata),
}

pub enum Message<'a> {
    Write(&'a mut [u8]),
    Read(PeerId, &'a mut [u8]),
    Noop,
    HandshakeComplete(MessageEncrypter),
}

pub struct MessageEncrypter(u32);

pub struct EncryptedMetadata {
    pub header: DataHeader,
    pub tag: Tag,
    pub payload_len: usize,
}

impl EncryptedMetadata {
    pub fn frame_in_place(self, buffer: &mut [u8]) {
        const H: usize = core::mem::size_of::<DataHeader>();
        assert_eq!(self.payload_len + 32, buffer.len());
        buffer[..H].copy_from_slice(zerocopy::IntoBytes::as_bytes(&self.header));
        buffer[H + self.payload_len..].copy_from_slice(zerocopy::IntoBytes::as_bytes(&self.tag));
    }
}

pub struct MaintenanceMsg {
    socket: core::net::SocketAddr,
    data: MaintenanceRepr,
}

impl MaintenanceMsg {
    pub fn to(&self) -> core::net::SocketAddr { self.socket }
    pub fn data(&self) -> &[u8] {
        match &self.data {
            MaintenanceRepr::Init(init) => zerocopy::IntoBytes::as_bytes(init),
            MaintenanceRepr::Data(ka) => zerocopy::IntoBytes::as_bytes(ka),
        }
    }
}

enum MaintenanceRepr {
    Init(HandshakeInit),
    Data(Keepalive),
}

#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::KnownLayout, zerocopy::Immutable)]
#[repr(C)]
struct Keepalive {
    header: DataHeader,
    tag: Tag,
}

fn write_msg<'b, T: zerocopy::IntoBytes + zerocopy::Immutable>(buf: &'b mut [u8], t: &T) -> &'b mut [u8] {
    let resp_msg = &mut buf[..core::mem::size_of::<T>()];
    resp_msg.copy_from_slice(zerocopy::IntoBytes::as_bytes(t));
    resp_msg
}

impl DynamicState {
    fn overloaded(&mut self, ip: core::net::IpAddr) -> bool {
        let key = match ip {
            core::net::IpAddr::V4(v4) => v4.to_bits() as u64,
            core::net::IpAddr::V6(v6) => (v6.to_bits() >> 64) as u64,
        };
        self.ip_rate_limit.count(&key) > 10
    }
}

// ───────────────────────────────────────────────────────────────────────
// CONSTANT-TIME COMPARISON AND UTILITIES
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn socket_addr_to_endpoint(addr: core::net::SocketAddr) -> embassy_net::IpEndpoint {
    match addr {
        core::net::SocketAddr::V4(v4) => v4.into(),
        core::net::SocketAddr::V6(_) => panic!("IPv6 not supported"),
    }
}


// ───────────────────────────────────────────────────────────────────────
// SESSIONS IMPLEMENTATION
pub struct Sessions {
    config: Config,
    dynamic: core::cell::RefCell<DynamicState>,
}

impl Sessions {
    pub fn new(config: Config, rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) -> Self {
        Self {
            dynamic: core::cell::RefCell::new(DynamicState::new(&config.peers, rng)),
            config,
        }
    }

    pub fn turn(&self, now: Tai64N, rng: &mut (impl rand_core::CryptoRng + rand_core::RngCore)) -> Option<MaintenanceMsg> {
        let mut state = self.dynamic.borrow_mut();
        if now > state.now {
            state.now = now;
            if now.duration_since(&state.last_reseed).unwrap() > core::time::Duration::from_secs(120) {
                state.cookie.generate(rng);
                state.rng = <rand_chacha::ChaCha20Rng as rand_core::SeedableRng>::from_rng(&mut *rng).expect("RNG failure");
                state.last_reseed = state.now;
            }
            if now.duration_since(&state.last_rate_reset).unwrap() > core::time::Duration::from_secs(1) {
                state.ip_rate_limit.reset(rng);
            }
        }
        drop(state);
        tick_timers(self)
    }

    pub fn send_message(&mut self, peer_idx: PeerId, payload: &mut [u8]) -> Result<SendMessage, Error> {
        let mut state_ref = self.dynamic.borrow_mut();
        let state = &mut *state_ref;
        let peer = state.peers.get_mut(peer_idx).ok_or(Error::Rejected)?;
        let Some(ep) = peer.endpoint else { return Err(Error::Rejected) };
        match peer.encrypt_message(&mut state.peers_by_session, payload, state.now) {
            Some(metadata) => {
                let session_id = peer.current_transport.unwrap();
                let session = state.peers_by_session.get_mut(&session_id).unwrap();
                let SessionState::Transport(ts) = &session.state else { unreachable!() };
                if ts.encrypt.counter() >= REKEY_AFTER_MESSAGES {
                    state.timers.push(TimerEntry { time: state.now, kind: TimerEntryType::RekeyAttempt { session_id } });
                }
                Ok(SendMessage::Data(ep, metadata))
            }
            None => {
                drop(state_ref);
                Ok(SendMessage::Maintenance(MaintenanceMsg {
                    socket: ep,
                    data: MaintenanceRepr::Init(new_handshake(self, peer_idx)?),
                }))
            }
        }
    }

    pub fn recv_message<'m>(&mut self, socket: core::net::SocketAddr, msg: &'m mut [u8]) -> Result<Message<'m>, Error> {
        if msg.as_ptr().align_offset(16) != 0 { return Err(Error::Unaligned) }
        let (msg_type, _) = <zerocopy::little_endian::U32 as zerocopy::FromBytes>::ref_from_prefix(msg)
            .map_err(|_| Error::InvalidMessage)?;
        match msg_type.get() {
            MSG_FIRST => self.handle_handshake_init(socket, msg).map(Message::Write),
            MSG_SECOND => self.handle_handshake_resp(socket, msg),
            MSG_COOKIE => self.handle_cookie(msg).map(|_| Message::Noop),
            MSG_DATA => self.decrypt_packet(socket, msg).map(|(id, m)| Message::Read(id, m)),
            _ => Err(Error::InvalidMessage),
        }
    }

    fn handle_handshake_init<'m>(
        &mut self,
        socket: core::net::SocketAddr,
        msg: &'m mut [u8],
    ) -> Result<&'m mut [u8], Error> {
        let mut state_ref = self.dynamic.borrow_mut();
        let state = &mut *state_ref;

        let init_ref = zerocopy::Ref::<_, HandshakeInit>::from_bytes(&mut *msg)
            .map_err(|_| Error::InvalidMessage)?;
        let init: &mut HandshakeInit = zerocopy::Ref::into_mut(init_ref);
        let overloaded = state.overloaded(socket.ip());

        // VERIFY MACS (AND POSSIBLY GET A COOKIE REQUEST BACK)
        let mac1 = init.compute_mac1(&self.config.static_.mac1_key);
        let init = match init.verify(&self.config.static_, overloaded, &state.cookie, socket) {
            Ok(core::ops::ControlFlow::Continue(init)) => init,
            Ok(core::ops::ControlFlow::Break(cookie)) => {
                let receiver = init.sender.get();
                return Ok(self.write_cookie_message(mac1, receiver, cookie, msg));
            }
            Err(_) => return Err(Error::Rejected),
        };

        let mut hs = HandshakeState::default();
        let decrypted = decrypt_handshake_init(init, &mut hs, &self.config.static_)?;

        // IDENTIFY PEER BY STATIC KEY
        let spk_i = decrypted.static_key();
        let peer_idx = self
            .config
            .get_peer_idx(&spk_i)
            .ok_or(Error::KeyExchangeError)?;
        let peer_cfg = &self.config.peers[peer_idx];
        let peer = &mut state.peers[peer_idx];

        // TIMESTAMP REPLAY PROTECTION
        if *decrypted.timestamp() < peer.latest_ts {
            return Err(Error::Rejected);
        }
        peer.latest_ts = *decrypted.timestamp();

        // ALLOCATE A FRESH SESSION ID (COLLISION FREE)
        let session_id = state.allocate_session_id();
        let peer = &mut state.peers[peer_idx];  

        let esk_r = EphemeralPrivateKey::generate(&mut state.rng);
        let resp = encrypt_handshake_resp(
            &mut hs,
            decrypted,
            &esk_r,
            peer_cfg,
            session_id,
            peer.cookie.as_ref(),
        )?;
        peer.last_sent_mac1 = resp.mac1;

        let (encrypt, decrypt) = hs.split::<Core>(false);
        let transport = SessionTransport {
            receiver: init.sender.get(),
            encrypt,
            decrypt,
        };
        let session = Session {
            peer: peer_idx,
            started: state.now,
            sent: state.now,
            state: SessionState::Transport(transport),
            keepalive_pending: false,
        };
        state.peers_by_session.insert(session_id, alloc::boxed::Box::new(session));

        // UPDATE PEER STATE
        if let Some(old) = peer.current_transport.take() {
            state.peers_by_session.remove(&old);
        }
        peer.current_transport = Some(session_id);

        // SCHEDULE TRANSPORT EXPIRATION
        state.timers.push(TimerEntry {
            time: state.now + REJECT_AFTER_TIME,
            kind: TimerEntryType::ExpireTransport { session_id },
        });

        // WRITE RESPONSE INTO THE SAME BUFFER (RESPONSE IS SMALLER THAN INIT)
        Ok(write_msg(msg, &resp))
    }

    fn handle_handshake_resp<'m>(
        &mut self,
        socket: core::net::SocketAddr,
        msg: &'m mut [u8],
    ) -> Result<Message<'m>, Error> {
        let mut state_ref = self.dynamic.borrow_mut();
        let state = &mut *state_ref;

        let resp_ref = zerocopy::Ref::<_, HandshakeResp>::from_bytes(&mut *msg)
            .map_err(|_| Error::InvalidMessage)?;
        let resp: &mut HandshakeResp = zerocopy::Ref::into_mut(resp_ref);
        let overloaded = state.overloaded(socket.ip());

        // VERIFY MACS
        let mac1 = resp.compute_mac1(&self.config.static_.mac1_key);
        let resp = match resp.verify(&self.config.static_, overloaded, &state.cookie, socket) {
            Ok(core::ops::ControlFlow::Continue(r)) => r,
            Ok(core::ops::ControlFlow::Break(cookie)) => {
                let receiver = resp.sender.get();
                return Ok(Message::Write(
                    self.write_cookie_message(mac1, receiver, cookie, msg),
                ));
            }
            Err(_) => return Err(Error::Rejected),
        };

        let session_id = resp.receiver.get();
        let Some(session) = state.peers_by_session.get_mut(&session_id) else {
            return Err(Error::Rejected);
        };

        // MUST BE A HANDSHAKE SESSION
        let (esk_i, mut hs_state) = match &mut session.state {
            SessionState::Handshake(hs) => (hs.esk_i.clone(), hs.state.clone()),
            SessionState::Transport(_) => return Err(Error::Rejected),
        };

        let peer_idx = session.peer;
        let peer_cfg = &self.config.peers[peer_idx];
        let initiator_cfg = &self.config.static_;
        let peer = &mut state.peers[peer_idx];

        decrypt_handshake_resp(resp, &mut hs_state, initiator_cfg, peer_cfg, &esk_i)?;

        // MOVE FROM HANDSHAKE -> TRANSPORT
        let (encrypt, decrypt) = hs_state.split::<Core>(true);
        zeroize::Zeroize::zeroize(&mut hs_state); // CLEAR HANDSHAKE REMNANTS

        let transport = SessionTransport {
            receiver: resp.sender.get(),
            encrypt,
            decrypt,
        };
        session.state = SessionState::Transport(transport);
        session.started = state.now;
        session.sent = state.now;

        // UPDATE PEER ENDPOINT AND SESSION POINTERS
        peer.endpoint = Some(socket);
        if let Some(old) = peer.current_transport.take() {
            state.peers_by_session.remove(&old);
        }
        peer.current_handshake = None;
        peer.current_transport = Some(session_id);

        // SCHEDULE RE‑KEY AND EXPIRATION
        state.timers.push(TimerEntry {
            time: state.now + REKEY_AFTER_TIME,
            kind: TimerEntryType::RekeyAttempt { session_id },
        });
        state.timers.push(TimerEntry {
            time: state.now + REJECT_AFTER_TIME,
            kind: TimerEntryType::ExpireTransport { session_id },
        });

        Ok(Message::HandshakeComplete(MessageEncrypter(session_id)))
    }

    fn handle_cookie<'m>(&self, msg: &'m mut [u8]) -> Result<(), Error> {
        let mut state_ref = self.dynamic.borrow_mut();
        let state = &mut *state_ref;

        let cookie_msg = zerocopy::Ref::<_, CookieMessage>::from_bytes(msg)
            .map_err(|_| Error::InvalidMessage)?;
        let cookie_msg = zerocopy::Ref::into_mut(cookie_msg);

        let session_id = cookie_msg.receiver.get();
        let Some(session) = state.peers_by_session.get(&session_id) else {
            return Err(Error::Rejected);
        };
        let peer_idx = session.peer;
        let peer_config = &self.config.peers[peer_idx];
        let peer = &mut state.peers[peer_idx];

        let decrypted = decrypt_cookie(
            &mut cookie_msg.cookie,
            &peer_config.cookie_key,
            &cookie_msg.nonce,
            &peer.last_sent_mac1,
        )?;
        peer.cookie = Some(*decrypted);
        Ok(())
    }

    fn write_cookie_message<'b>(
        &self,
        mac1: Mac,
        receiver: u32,
        cookie: Cookie,
        buf: &'b mut [u8],
    ) -> &'b mut [u8] {
        let mut nonce = [0u8; 24];
        rand_core::RngCore::fill_bytes(&mut self.dynamic.borrow_mut().rng, &mut nonce);
        let enc_cookie = encrypt_cookie(cookie, &self.config.static_.cookie_key, &nonce, &mac1);
        let msg = CookieMessage {
            _type: zerocopy::little_endian::U32::new(MSG_COOKIE),
            receiver: zerocopy::little_endian::U32::new(receiver),
            nonce,
            cookie: enc_cookie,
        };
        write_msg(buf, &msg)
    }
    
    fn decrypt_packet<'m>(
        &self,
        socket: core::net::SocketAddr,
        msg: &'m mut [u8],
    ) -> Result<(PeerId, &'m mut [u8]), Error> {
        let mut state_ref = self.dynamic.borrow_mut();
        let state = &mut *state_ref;
        let (header, payload_and_tag) =
            DataHeader::message_mut_from(msg).ok_or(Error::InvalidMessage)?;
        let session_id = header.receiver.get();
        let Some(session) = state.peers_by_session.get_mut(&session_id) else {
            return Err(Error::Rejected);
        };
        let ts = match &mut session.state {
            SessionState::Handshake(_) => return Err(Error::Rejected),
            SessionState::Transport(ts) => ts,
        };
        let payload = ts.decrypt.decrypt::<Core>(header.counter.get(), payload_and_tag)?;
        let peer_idx = session.peer;
        let needs_keepalive =
            session.sent + KEEPALIVE_TIMEOUT < state.now && !session.keepalive_pending;
        if needs_keepalive {
            session.keepalive_pending = true;
        }
        let peer = &mut state.peers[peer_idx];
        peer.endpoint = Some(socket);
        if needs_keepalive {
            state.timers.push(TimerEntry {
                time: state.now,
                kind: TimerEntryType::Keepalive { session_id },
            });
        }
        Ok((peer_idx, payload))
    }
}


// ───────────────────────────────────────────────────────────────────────
// PEER STATE IMPLEMENTATION
impl PeerState {
    pub fn new(endpoint: Option<core::net::SocketAddr>) -> Self {
        Self {
            endpoint,
            latest_ts: [0; 12],
            cookie: None,
            current_handshake: None,
            current_transport: None,
            last_sent_mac1: [0; 16],
        }
    }

    pub fn encrypt_message(
        &mut self,
        sessions: &mut SessionMap,
        payload: &mut [u8],
        now: Tai64N,
    ) -> Option<EncryptedMetadata> {
        let session_id = self.current_transport?;
        let session = sessions.get_mut(&session_id)?;
        let SessionState::Transport(ts) = &session.state else {
            return None;
        };
        if session.should_reject(now, ts) {
            return None;
        }
        Some(self.force_encrypt(session, payload, now))
    }

    pub fn force_encrypt(
        &mut self,
        session: &mut Session,
        payload: &mut [u8],
        now: Tai64N,
    ) -> EncryptedMetadata {
        // ALLOW ZERO-LENGTH PAYLOAD FOR KEEPALIVE
        if !payload.is_empty() {
            assert_eq!(payload.len() % 16, 0);
        }
        let SessionState::Transport(ts) = &mut session.state else {
            unreachable!()
        };
        let n = ts.encrypt.counter();
        let tag = ts.encrypt.encrypt::<Core>(payload);
        session.sent = now;
        let header = DataHeader {
            _type: zerocopy::little_endian::U32::new(MSG_DATA),
            receiver: zerocopy::little_endian::U32::new(ts.receiver),
            counter: zerocopy::little_endian::U64::new(n),
        };
        EncryptedMetadata {
            header,
            tag,
            payload_len: payload.len(),
        }
    }

    /// CREATE A KEEPALIVE PACKET FOR THE GIVEN TRANSPORT SESSION.
    pub fn keepalive(&mut self, session: &mut Session, now: Tai64N) -> EncryptedMetadata {
        self.force_encrypt(session, &mut [], now)
    }
}


// ───────────────────────────────────────────────────────────────────────
// MESSAGE ENCRYPTER
impl MessageEncrypter {
    pub fn encrypt(self, sessions: &Sessions, payload: &mut [u8]) -> Option<EncryptedMetadata> {
        let mut state_ref = sessions.dynamic.borrow_mut();
        let state = &mut *state_ref;
        let now = state.now;
        let session = state.peers_by_session.get_mut(&self.0)?;
        let SessionState::Transport(ts) = &session.state else { return None };
        if session.should_reject(now, ts) { return None }
        let peer = &mut state.peers[session.peer];
        Some(peer.force_encrypt(session, payload, now))
    }

    pub fn encrypt_and_frame(self, sessions: &Sessions, buffer: &mut [u8]) -> bool {
        let len = buffer.len();
        let payload = &mut buffer[16..len - 16];
        let Some(meta) = self.encrypt(sessions, payload) else { return false };
        meta.frame_in_place(buffer);
        true
    }
}

// ───────────────────────────────────────────────────────────────────────
// HANDSHAKE CREATION
pub fn new_handshake(sessions: &Sessions, peer_idx: PeerId) -> Result<HandshakeInit, Error> {
    let mut state_ref = sessions.dynamic.borrow_mut();
    let state = &mut *state_ref;
    let peer_config = &sessions.config.peers[peer_idx];
    // REMOVE OLD HANDSHAKE WHILE WE HAVE A FULL MUTABLE BORROW
    let peer = &mut state.peers[peer_idx];
    if let Some(session_id) = peer.current_handshake.take() {
        state.peers_by_session.remove(&session_id);
    }

    // ALLOCATE A NEW SESSION ID (ONLY USES RNG, NO PEER BORROW)
    let session_id = state.allocate_session_id();

    // NOW BORROW PEER AGAIN – PREVIOUS BORROW IS DEAD
    let peer = &mut state.peers[peer_idx];
    peer.current_handshake = Some(session_id);

    let esk_i = EphemeralPrivateKey::generate(&mut state.rng);
    let mut hs = HandshakeState::default();
    let now = state.now;

    let init = encrypt_handshake_init(
        &mut hs,
        &sessions.config.static_,
        peer_config,
        &esk_i,
        now,
        session_id,
        peer.cookie.as_ref(),
    )?;

    let session = Session {
        peer: peer_idx,
        started: now,
        sent: now,
        state: SessionState::Handshake(SessionHandshake { esk_i, state: hs }),
        keepalive_pending: false,
    };
    state.peers_by_session.insert(session_id, alloc::boxed::Box::new(session));

    // SCHEDULE HANDSHAKE RETRANSMISSION AND EXPIRATION
    state.timers.push(TimerEntry {
        time: now + REKEY_TIMEOUT,
        kind: TimerEntryType::InitAttempt { session_id },
    });
    state.timers.push(TimerEntry {
        time: now + REKEY_ATTEMPT_TIME,
        kind: TimerEntryType::ExpireHandshake { session_id },
    });

    Ok(init)
}

// ───────────────────────────────────────────────────────────────────────
// TIMER PROCESSING
fn tick_timers(sessions: &Sessions) -> Option<MaintenanceMsg> {
    // COLLECT ACTIONS THAT REQUIRE HANDSHAKE/KEEPALIVE OUTSIDE THE MUTABLE BORROW
    enum Action {
        DoInitHandshake { peer_idx: PeerId },
        DoRekey { peer_idx: PeerId },
        DoKeepalive { session_id: u32 },
    }

    let actions = {
        let mut state = sessions.dynamic.borrow_mut();
        let now = state.now;
        let mut actions = alloc::vec::Vec::new();

        while let Some(entry) = state.timers.peek() {
            if entry.time > now {
                break;
            }
            let entry = state.timers.pop().unwrap();

            match entry.kind {
                TimerEntryType::InitAttempt { session_id } => {
                    if let Some(session) = state.peers_by_session.get(&session_id) {
                        if let SessionState::Handshake(hs) = &session.state {
                            if session.should_reinit(now, hs) {
                                actions.push(Action::DoInitHandshake { peer_idx: session.peer });
                            }
                        }
                    }
                }
                TimerEntryType::RekeyAttempt { session_id } => {
                    if let Some(session) = state.peers_by_session.get(&session_id) {
                        if let SessionState::Transport(ts) = &session.state {
                            if session.should_rekey(now, ts) {
                                actions.push(Action::DoRekey { peer_idx: session.peer });
                            }
                        }
                    }
                }
                TimerEntryType::Keepalive { session_id } => {
                    if let Some(session) = state.peers_by_session.get(&session_id) {
                        if let SessionState::Transport(_) = &session.state {
                            if session.sent + KEEPALIVE_TIMEOUT < now && !session.keepalive_pending {
                                actions.push(Action::DoKeepalive { session_id });
                            }
                        }
                    }
                }
                TimerEntryType::ExpireTransport { session_id } => {
                    if let Some(session) = state.peers_by_session.remove(&session_id) {
                        let peer = &mut state.peers[session.peer];
                        if peer.current_transport == Some(session_id) {
                            peer.current_transport = None;
                        }
                    }
                }
                TimerEntryType::ExpireHandshake { session_id } => {
                    if let Some(session) = state.peers_by_session.remove(&session_id) {
                        let peer = &mut state.peers[session.peer];
                        if peer.current_handshake == Some(session_id) {
                            peer.current_handshake = None;
                        }
                    }
                }
            }
        }
        actions
    };

    // PROCESS COLLECTED ACTIONS (NO ACTIVE BORROW)
    for action in actions {
        match action {
            Action::DoInitHandshake { peer_idx } | Action::DoRekey { peer_idx } => {
                if let Ok(init) = new_handshake(sessions, peer_idx) {
                    let ep = sessions.config.peers[peer_idx].endpoint?;
                    return Some(MaintenanceMsg {
                        socket: ep,
                        data: MaintenanceRepr::Init(init),
                    });
                }
            }
            Action::DoKeepalive { session_id } => {
                let mut state = sessions.dynamic.borrow_mut();
                let DynamicState {
                    ref mut peers,
                    ref mut peers_by_session,
                    now,
                    ..
                } = *state; // SPLIT BORROWS

                if let Some(session) = peers_by_session.get_mut(&session_id) {
                    if let SessionState::Transport(_) = &session.state {
                        let peer_idx = session.peer;
                        if session.sent + KEEPALIVE_TIMEOUT < now && !session.keepalive_pending {
                            let ep = peers[peer_idx].endpoint;
                            session.keepalive_pending = false; // CLEAR EVEN WITHOUT ENDPOINT
                            if let Some(ep) = ep {
                                let keepalive_meta = peers[peer_idx].force_encrypt(session, &mut [], now);
                                return Some(MaintenanceMsg {
                                    socket: ep,
                                    data: MaintenanceRepr::Data(Keepalive {
                                        header: keepalive_meta.header,
                                        tag: keepalive_meta.tag,
                                    }),
                                });
                            }
                        }
                    }
                }
            }
        }    
    }

    None
}

// ───────────────────────────────────────────────────────────────────────
// ERROR TYPE
#[derive(Debug, defmt::Format)]
pub enum Error {
    InvalidMessage,
    DecryptionError,
    KeyExchangeError,
    Unaligned,
    Rejected,
}

impl From<CryptoError> for Error {
    fn from(value: CryptoError) -> Self {
        match value {
            CryptoError::KeyExchangeError => Error::KeyExchangeError,
            CryptoError::DecryptionError => Error::DecryptionError,
            CryptoError::Rejected => Error::Rejected,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// TAI64N FROM UPTIME
pub use tai64::Tai64N;
const BASE_TAI64N: Tai64N = Tai64N(tai64::Tai64(0x6000_0000), 0);
pub fn monotonic_tai64n() -> Tai64N {
    let uptime_us = embassy_time::Instant::now().as_micros();
    BASE_TAI64N + core::time::Duration::from_micros(uptime_us)
}

// ───────────────────────────────────────────────────────────────────────
// FRAMING HELPERS
pub fn frame_payload(raw: &[u8], out: &mut [u8]) -> usize {
    let len = raw.len();
    assert!(len <= 0xFFFF);
    out[0..2].copy_from_slice(&(len as u16).to_be_bytes());
    out[2..2+len].copy_from_slice(raw);
    let total = len + 2;
    let padded = (total + 15) & !15;
    out[total..padded].fill(0);
    padded
}

pub fn unframe_payload(padded: &[u8]) -> Option<&[u8]> {
    if padded.len() < 2 { return None; }
    let len = u16::from_be_bytes([padded[0], padded[1]]) as usize;
    if padded.len() < 2 + len { return None; }
    Some(&padded[2..2+len])
}

// ───────────────────────────────────────────────────────────────────────
// WG KEY HANDLING

fn static_private_key(b: &[u8; 32]) -> StaticPrivateKey { StaticPrivateKey(*b) }
fn public_key(b: &[u8; 32]) -> PublicKey { PublicKey(*b) }

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

const fn const_base64_decode<const N: usize>(input: &str) -> [u8; N] {
    let bytes = input.as_bytes();
    let expected_len = ((N + 2) / 3) * 4;
    assert!(bytes.len() == expected_len, "Invalid Base64 length");

    let mut out = [0u8; N];
    let mut byte_idx = 0;
    let mut bit_buffer = 0u32;
    let mut bits_in_buffer = 0u32;

    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'=' {
            break;
        }
        let val = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("Invalid Base64 character"),
        };

        bit_buffer = (bit_buffer << 6) | val as u32;
        bits_in_buffer += 6;
        if bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            out[byte_idx] = (bit_buffer >> bits_in_buffer) as u8;
            byte_idx += 1;
            bit_buffer &= (1 << bits_in_buffer) - 1;
        }
        i += 1;
    }

    assert!(byte_idx == N, "Decoded length mismatch");
    out
}


// ───────────────────────────────────────────────────────────────────────
// ALIGNED BUFFER
#[repr(align(16))]
struct AlignedBuf<const N: usize>([u8; N]);
impl<const N: usize> AlignedBuf<N> {
    const fn new() -> Self { Self([0u8; N]) }
}

// ───────────────────────────────────────────────────────────────────────
// HARDWARE RNG
fn get_hw_rng(rng: &mut esp_hal::rng::Rng) -> rand_chacha::ChaCha20Rng {
    let mut seed = [0u8; 32];
    for chunk in seed.chunks_mut(4) {
        let val = rng.random();
        chunk.copy_from_slice(&val.to_le_bytes());
    }
    <rand_chacha::ChaCha20Rng as rand_core::SeedableRng>::from_seed(seed)
}


// ───────────────────────────────────────────────────────────────────────
// WIREGUARD TASK
#[embassy_executor::task]
pub async fn wireguard_task(
    wifi_stack: &'static embassy_net::Stack<'static>,
    config: &'static WgConfig,
    mut rng: esp_hal::rng::Rng,
    mut runner: ch::Runner<'static, 1420>,
) {
    defmt::info!("🛡️  💤");

    loop {
        let cmd = WG_CMD.receive().await;
        match cmd {
            WgCommand::Enable => {
                defmt::info!("🛡️  VPN enabled!");

                // DNS RESOLUTION FOR ENDPOINT
                let server_ip = loop {
                    match wifi_stack.dns_query(config.endpoint_host, embassy_net::dns::DnsQueryType::A).await
                    {
                        Ok(addrs) if !addrs.is_empty() => {
                            let ip = match addrs[0] {
                                embassy_net::dns::IpAddress::Ipv4(v4) => core::net::IpAddr::V4(v4),
                            };
                            defmt::info!("🛡️  DNS resolved to: {}", ip);
                            break ip;
                        }
                        Ok(_) => {
                            defmt::error!("🛡️  DNS query for {} returned no addresses", config.endpoint_host);
                            embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
                        }
                        Err(e) => {
                            defmt::error!("🛡️ DNS query failed for {}: {}", config.endpoint_host, e);
                            embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
                        }
                    }
                };
                let server_addr = core::net::SocketAddr::new(server_ip, config.endpoint_port);

                // CONFIG & SESSIONS
                let sk = StaticPrivateKey(config.private_key);
                let pk = PublicKey(config.peer_public_key);
                let mut wg_cfg = Config::new(sk);
                let peer_cfg = StaticPeerConfig::new(pk, Some(Key::default()), Some(server_addr));
                let peer_id = wg_cfg.insert_peer(peer_cfg);

                let mut hw_rng = get_hw_rng(&mut rng);
                let mut sessions = Sessions::new(wg_cfg, &mut hw_rng);

                // UDP SOCKET
                let mut rx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 4];
                let mut rx_ring = [0u8; 2048];
                let mut tx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 4];
                let mut tx_ring = [0u8; 2048];
                let mut socket = embassy_net::udp::UdpSocket::new(
                    *wifi_stack,
                    &mut rx_meta,
                    &mut rx_ring,
                    &mut tx_meta,
                    &mut tx_ring,
                );
                
                // BIND SOCKET (PORT 23456 FOR EASIER DEBUGGING)
                if let Err(e) = socket.bind(embassy_net::IpListenEndpoint { addr: None, port: 23456 }) {
                    defmt::error!("failed to bind UDP socket: {:?}", e);
                    continue; // BACK TO IDLE (WAIT FOR START/STOP)
                }

                // SEND INIT HANDSHAKE!
                defmt::info!("🛡️ attempting first handshake to {}:{}", config.endpoint_host, config.endpoint_port);
                let mut payload = [0u8; 16];
                match sessions.send_message(peer_id, &mut payload) {
                    Ok(SendMessage::Maintenance(m)) => {
                        let data = m.data();
                        defmt::info!("🛡️ initiating handshake: {} bytes to {}", data.len(), m.to());
                        match socket.send_to(data, socket_addr_to_endpoint(m.to())).await {
                            Ok(n) => defmt::info!("🛡️ SENT bytes"),
                            Err(e) => defmt::error!("🛡️ failed to send handshake: {:?}", e),
                        }
                    }
                    Ok(SendMessage::Data(..)) => { defmt::error!("unexpected data message instead of handshake"); }
                    Err(e) => defmt::error!("failed to create handshake: {:?}", e),
                }

                // MAIN LOOP
                loop {
                    // CHECK FOR DISABLE COMMAND
                    if let Ok(WgCommand::Disable) = WG_CMD.try_receive() {
                        defmt::info!("🛡️ 🚫 VPN disabled!");
                        VPN_ACTIVE.store(false, core::sync::atomic::Ordering::Relaxed);
                        runner.set_link_state(LinkState::Down);
                        break;
                    }

                    let now = monotonic_tai64n();

                    // PROCESS TIMERS > MAY PRODUCE MAINTENANCE MESSAGES
                    if let Some(maintenance) = sessions.turn(now, &mut hw_rng) {
                        let data = maintenance.data();
                        defmt::info!("🛡️  sending maintenance (len {}) to {}", data.len(), maintenance.to());
                        match socket.send_to(data, socket_addr_to_endpoint(maintenance.to())).await
                        {
                            Ok(n) => defmt::info!("🛡️ sent {} bytes maintenance", n),
                            Err(e) => defmt::error!("🛡️  send_to (maintenance) failed: {:?}", e),
                        }
                    }

                    // RECEIVE FROM SERVER
                    let mut rx_buf = AlignedBuf::<2048>::new();
                    match socket.recv_from(&mut rx_buf.0).await {
                        Ok((n, meta)) => {
                            let src = core::net::SocketAddr::new(
                                meta.endpoint.addr.into(),
                                meta.endpoint.port,
                            );
                            defmt::info!("🛡️  received {} bytes from {}", n, src);
                            match sessions.recv_message(src, &mut rx_buf.0[..n]) {
                                Ok(Message::Write(response)) => {
                                    defmt::info!("🛡️  sending handshake response (len {})", response.len());
                                    match socket.send_to(response, socket_addr_to_endpoint(src)).await
                                    {
                                        Ok(n) => defmt::info!("🛡️  sent {} bytes response", n),
                                        Err(e) => defmt::error!("🛡️  send_to (response) failed: {:?}", e),
                                    }
                                }
                                Ok(Message::Read(_peer, ip_packet)) => {
                                    defmt::info!("decrypted packet (len {})", ip_packet.len());
                                    if let Some(mut slot) = runner.try_rx_buf() {
                                        let len = ip_packet.len();
                                        slot[..len].copy_from_slice(ip_packet);
                                        runner.rx_done(len);
                                        defmt::info!("forwarded to VPN stack");
                                    } else { defmt::warn!("RX channel full, dropping packet"); }
                                }
                                Ok(Message::HandshakeComplete(_enc)) => {
                                    defmt::info!("🛡️  🎉 handshake complete!");
                                    VPN_ACTIVE.store(true, core::sync::atomic::Ordering::Relaxed);
                                    runner.set_link_state(LinkState::Up);
                                    WG_READY.signal(());
                                }
                                Err(e) => { defmt::error!("recv_message error: {:?}", e); }
                                _ => {}
                            }
                        }
                        Err(e) => { defmt::error!("recv_from error: {:?}", e); }
                    }

                    if let Some(slot) = runner.try_tx_buf() {
                        let len = slot.len();
                        defmt::warn!("TX from stack: {} bytes (dropped – data path not yet implemented)", len);
                        runner.tx_done();
                    }

                    embassy_time::Timer::after_millis(1).await;
                }
            }
            WgCommand::Disable => { defmt::debug!("🛡️ already disabled"); }
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// WAIT UNTIL THE FIRST WIREGUARD TRANSPORT SESSION IS ESTABLISHED
pub async fn wireguard_wait_ready() {
    WG_READY.wait().await;
}

// ───────────────────────────────────────────────────────────────────────
// TOGGLE ON/OFF
pub async fn toggle_vpn() {
    let current = crate::load!(crate::state::WG_STATE);
    if current { // TURN IT OFF
        crate::base::wireguard::WG_CMD.send(crate::base::wireguard::WgCommand::Disable).await;
        crate::store!(crate::state::WG_STATE, false);
        defmt::info!("VPN disabled");
    } else { // TURN IT ON
        crate::base::wireguard::WG_CMD.send(crate::base::wireguard::WgCommand::Enable).await;
        crate::store!(crate::state::WG_STATE, true);
        defmt::info!("VPN enabled");
    }
}

#[embassy_executor::task]
pub async fn vpn_runner_task(
    mut runner: embassy_net::Runner<'static,
        embassy_net_driver_channel::Device<'static, 1420>>
) {
    runner.run().await;
}
