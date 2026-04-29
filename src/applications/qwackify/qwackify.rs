// APPS/MEDIA
//  QWACKIFY - BARE METAL
//   QWACKTASTIC MEDIA PLAYER

use core::cell::RefCell;
use critical_section::Mutex;
use defmt::{info, error};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::string::ToString;
use crate::SPEAKER_VOLUME;
use tinyapi::{http_get, HttpResponse};
use embassy_net::Stack;

#[derive(Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone)]
pub struct Track {
    pub id: u32,
    pub title: String,
    pub file_path: String,
}

// ALIASES FOR CRITICAL-SECTION mutexes
pub use critical_section::Mutex as CsMutex;

// DYNAMIC PLAYLIST
static PLAYLIST: CsMutex<RefCell<Vec<Track>>> = CsMutex::new(RefCell::new(Vec::new()));

struct PlayerInner {
    pub state: PlaybackState,
    pub current_track_index: usize,
}

pub static PLAYER: CsMutex<RefCell<PlayerInner>> = CsMutex::new(RefCell::new(PlayerInner {
    state: PlaybackState::Stopped,
    current_track_index: 0,
}));

fn playlist_len() -> usize {
    critical_section::with(|cs| PLAYLIST.borrow_ref(cs).len())
}

pub fn handle_action(action: &str) -> &'static str {
    match action {
        "play" => { let _ = play(); "Playing" }
        "pause" => { pause(); "Paused" }
        "next" => { next(); "Next track" }
        "prev" => { prev(); "Previous track" }
        "stop" => { stop(); "Stopped" }
        "status" => get_status_text(),
        "volume_up" => { volume_up(); "Volume up" }
        "volume_down" => { volume_down(); "Volume down" }
        _ => { info!("Unknown media action: {}", action); "Unknown action" }
    }
}

pub fn get_status_text() -> &'static str {
    "status placeholder"
}

fn play() -> Result<(), &'static str> {
    let pl_len = playlist_len();
    if pl_len == 0 { return Err("Playlist is empty"); }

    let (track_title, track_path) = critical_section::with(|cs| {
        let pl = PLAYLIST.borrow_ref(cs);
        let player = PLAYER.borrow_ref(cs);
        let idx = player.current_track_index % pl_len;
        (pl[idx].title.clone(), pl[idx].file_path.clone())
    });

    audio_hardware_stop();
    if let Err(e) = audio_hardware_play(&track_path) {
        error!("Failed to play {}: {}", &*track_path, e);   // &*String -> &str
        critical_section::with(|cs| {
            PLAYER.borrow_ref_mut(cs).state = PlaybackState::Stopped;
        });
        return Err("Playback failed");
    }

    critical_section::with(|cs| {
        PLAYER.borrow_ref_mut(cs).state = PlaybackState::Playing;
    });
    info!("Now playing: {}", &*track_title);                // &*String -> &str
    Ok(())
}

fn pause() {
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        match player.state {
            PlaybackState::Playing => {
                audio_hardware_pause();
                player.state = PlaybackState::Paused;
                info!("Playback paused");
            }
            PlaybackState::Paused => {
                audio_hardware_resume();
                player.state = PlaybackState::Playing;
                info!("Playback resumed");
            }
            _ => (),
        }
    });
}

fn stop() {
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        if player.state != PlaybackState::Stopped {
            audio_hardware_stop();
            player.state = PlaybackState::Stopped;
            info!("Playback stopped");
        }
    });
}

fn next() {
    let pl_len = playlist_len();
    if pl_len == 0 { return; }

    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = (player.current_track_index + 1) % pl_len;
        let title = &PLAYLIST.borrow_ref(cs)[player.current_track_index].title;
        info!("Switched to next track: {}", &**title);    // &String -> &str via deref
    });

    let was_playing = critical_section::with(|cs| PLAYER.borrow_ref(cs).state) == PlaybackState::Playing;
    if was_playing { let _ = play(); }
}

fn prev() {
    let pl_len = playlist_len();
    if pl_len == 0 { return; }

    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = if player.current_track_index == 0 { pl_len - 1 } else { player.current_track_index - 1 };
        let title = &PLAYLIST.borrow_ref(cs)[player.current_track_index].title;
        info!("Switched to previous track: {}", &**title);
    });

    let was_playing = critical_section::with(|cs| PLAYER.borrow_ref(cs).state) == PlaybackState::Playing;
    if was_playing { let _ = play(); }
}

fn volume_up() {
    let current = SPEAKER_VOLUME.load(core::sync::atomic::Ordering::Relaxed);
    let new = (current + 5).min(100);
    SPEAKER_VOLUME.store(new, core::sync::atomic::Ordering::Relaxed);
    info!("Media volume increased to {}%", new);
}

fn volume_down() {
    let current = SPEAKER_VOLUME.load(core::sync::atomic::Ordering::Relaxed);
    let new = current.saturating_sub(5);
    SPEAKER_VOLUME.store(new, core::sync::atomic::Ordering::Relaxed);
    info!("Media volume decreased to {}%", new);
}

// STUB HARDWARE FUNCTIONS
fn audio_hardware_play(file_path: &str) -> Result<(), &'static str> {
    info!("Playing file: {}", file_path);
    Ok(())
}
fn audio_hardware_stop() {}
fn audio_hardware_pause() {}
fn audio_hardware_resume() {}

// M3U PARSER 
fn parse_m3u(data: &str) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut pending_title: Option<String> = None;
    let mut next_id: u32 = 1;

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if line.starts_with("#EXTINF:") {
            if let Some(comma_idx) = line.find(',') {
                let title = String::from(line[comma_idx+1..].trim());   // String::from instead of to_owned
                pending_title = Some(title);
            } else {
                pending_title = None;
            }
        } else if line.starts_with('#') {
            pending_title = None;
        } else {
            let path = String::from(line);
            let title = pending_title.take().unwrap_or_else(|| {
                // FALLBACK -  FILENAME FROM URL
                path.rsplit('/').next().unwrap_or(&path).to_string()
            });
            tracks.push(Track { id: next_id, title, file_path: path });
            next_id += 1;
        }
    }
    tracks
}

/// FETCH REMOTE PLAYLIST & REPLACE THE CURRENT
pub async fn fetch_playlist(stack: Stack<'_>, url: &str) -> Result<(), &'static str> {
    let mut buf = [0u8; 4096];

    let resp = http_get(stack, url, &mut buf).await.map_err(|_| "HTTP GET failed")?;
    if resp.status != 200 {
        return Err("Server returned non-200 status");
    }

    let body_str = core::str::from_utf8(resp.body).map_err(|_| "Invalid UTF-8")?;
    let new_playlist = parse_m3u(body_str);
    if new_playlist.is_empty() {
        return Err("Parsed playlist is empty");
    }

    critical_section::with(|cs| {
        let mut pl = PLAYLIST.borrow_ref_mut(cs);
        *pl = new_playlist;
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = 0;
        player.state = PlaybackState::Stopped;
    });

    info!("Playlist updated with {} tracks", playlist_len());
    Ok(())
}
