// APPLICATIONS/MEDIA_PLAYER
// MEDIA PLAYER WITH MP3 DECODING & DOWNSAMPLING.
// PLAYBACK USAGE:
// crate::spawn!(spawner, crate::applications::media_player::play_mp3_task(alloc::string::ToString::to_string("/Music/MySong.mp3")));


use alloc::string::String;
use alloc::vec::Vec;
use alloc::string::ToString;


// TYPES & GLOBAL STATE
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

pub use critical_section::Mutex as CsMutex;

static PLAYLIST: CsMutex<core::cell::RefCell<Vec<Track>>> = CsMutex::new(core::cell::RefCell::new(Vec::new()));

struct PlayerInner {
    pub state: PlaybackState,
    pub current_track_index: usize,
}

pub static PLAYER: CsMutex<core::cell::RefCell<PlayerInner>> = CsMutex::new(core::cell::RefCell::new(PlayerInner {
    state: PlaybackState::Stopped,
    current_track_index: 0,
}));

// SIGNAL TO STOP THE CURRENTLY PLAYING TASK
static STOP_SIGNAL: embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, ()> = embassy_sync::signal::Signal::new();

// HELPER
fn playlist_len() -> usize {
    critical_section::with(|cs| PLAYLIST.borrow_ref(cs).len())
}

// PUBLIC API
pub async fn handle_action(spawner: embassy_executor::Spawner, action: &str) -> &'static str {
    match action {
        "play" => { let _ = start_playback(spawner).await; "Playing" }
        "pause" => { pause(); "Paused" }
        "next" => { next(spawner).await; "Next track" }
        "prev" => { prev(spawner).await; "Previous track" }
        "stop" => { stop(); "Stopped" }
        "status" => get_status_text(),
        "volume_up" => { volume_up(); "Volume up" }
        "volume_down" => { volume_down(); "Volume down" }
        _ => { defmt::info!("Unknown media action: {}", action); "Unknown action" }
    }
}

pub fn get_status_text() -> &'static str {
    "status placeholder"
}

async fn start_playback(spawner: embassy_executor::Spawner) -> Result<(), &'static str> {
    let pl_len = playlist_len();
    if pl_len == 0 { return Err("Playlist is empty"); }

    let (track_title, track_path) = critical_section::with(|cs| {
        let pl = PLAYLIST.borrow_ref(cs);
        let player = PLAYER.borrow_ref(cs);
        let idx = player.current_track_index % pl_len;
        (pl[idx].title.clone(), pl[idx].file_path.clone())
    });

    stop(); // STOP ANY CURRENT PLAYBACK

    // SPAWN THE MP3 PLAYBACK TASK
    crate::spawn!(spawner, play_mp3_task(track_path.clone()));
    

    critical_section::with(|cs| {
        PLAYER.borrow_ref_mut(cs).state = PlaybackState::Playing;
    });
    defmt::info!("Now playing: {}", track_title.as_str());
    Ok(())
}

fn pause() {
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        if player.state == PlaybackState::Playing {
            stop();
            player.state = PlaybackState::Paused;
            defmt::info!("Playback paused");
        } else if player.state == PlaybackState::Paused {
            defmt::info!("Press play to resume");
        }
    });
}

fn stop() {
    STOP_SIGNAL.signal(());
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        if player.state != PlaybackState::Stopped {
            player.state = PlaybackState::Stopped;
            defmt::info!("Playback stopped");
        }
    });
}

async fn next(spawner: embassy_executor::Spawner) {
    let pl_len = playlist_len();
    if pl_len == 0 { return; }

    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = (player.current_track_index + 1) % pl_len;
        let title = &PLAYLIST.borrow_ref(cs)[player.current_track_index].title;
        defmt::info!("Switched to next track: {}", title.as_str());
    });

    let was_playing = critical_section::with(|cs| PLAYER.borrow_ref(cs).state) == PlaybackState::Playing;
    if was_playing { let _ = start_playback(spawner).await; }
}

async fn prev(spawner: embassy_executor::Spawner) {
    let pl_len = playlist_len();
    if pl_len == 0 { return; }

    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = if player.current_track_index == 0 { pl_len - 1 } else { player.current_track_index - 1 };
        let title = &PLAYLIST.borrow_ref(cs)[player.current_track_index].title;
        defmt::info!("Switched to previous track: {}", title.as_str());
    });

    let was_playing = critical_section::with(|cs| PLAYER.borrow_ref(cs).state) == PlaybackState::Playing;
    if was_playing { let _ = start_playback(spawner).await; }
}

fn volume_up() {
    let current = crate::load!(crate::state::SPEAKER_VOLUME);
    let new = (current + 5).min(100);
    crate::store!(crate::state::SPEAKER_VOLUME, new);
    defmt::info!("Volume increased to {}%", new);
}

fn volume_down() {
    let current = crate::load!(crate::state::SPEAKER_VOLUME);
    let new = current.saturating_sub(5);
    crate::store!(crate::state::SPEAKER_VOLUME, new);
    defmt::info!("Volume decreased to {}%", new);
}




// ASYNC MP3 PLAYBACK TASK
#[embassy_executor::task]
pub async fn play_mp3_task(path: String) {
    defmt::info!("🎵 MP3 playback task started: {}", path.as_str());

    let mut file = match crate::components::storage::open_file_stream(&path) {
        Ok(f) => f,
        Err(e) => {
            defmt::error!("Failed to open {}: {:?}", path.as_str(), e);
            return;
        }
    };

    let mut decoder = nanomp3::Decoder::new();
    let mut pcm_f32 = [0.0f32; nanomp3::MAX_SAMPLES_PER_FRAME];
    let mut i16_buf = [0i16; nanomp3::MAX_SAMPLES_PER_FRAME];
    let mut mp3_buffer = [0u8; 4096];
    let mut pos = 0;
    let mut valid = 0;

    loop {
        if STOP_SIGNAL.signaled() {
            defmt::info!("🎵 Playback stopped by user");
            break;
        }

        if valid - pos < 1024 {
            let leftover = valid - pos;
            if leftover > 0 {
                mp3_buffer.copy_within(pos..valid, 0);
            }
            valid = leftover;
            pos = 0;
            let space = mp3_buffer.len() - valid;
            if space > 0 {
                match file.read(&mut mp3_buffer[valid..]) {
                    Ok(0) => break,
                    Ok(n) => valid += n,
                    Err(e) => {
                        defmt::error!("SD read error: {:?}", e);
                        break;
                    }
                }
            }
        }

        let (consumed, frame_info) = decoder.decode(&mp3_buffer[pos..valid], &mut pcm_f32);
        if consumed == 0 {
            if valid == 0 {
                break;
            }
            embassy_time::Timer::after_millis(1).await;
            continue;
        }
        pos += consumed;

        if let Some(info) = frame_info {
            let samples = info.samples_produced;
            let channels = info.channels.num() as usize;
            let total_original = samples * channels;
            let sample_rate = info.sample_rate;

            // TARGET I2S SAMPLE RATE IS 16000 Hz (LOADED FROM STATE FILE)
            let decimation = (sample_rate / crate::state::I2S_SAMPLE_RATE) as usize;

            // ONLY DECIMATE IF RATIO IS INTEGER AND GREATER THAN 1
            let total_downsampled = if decimation > 1 && sample_rate % crate::state::I2S_SAMPLE_RATE == 0 {
                total_original / decimation
            } else {
                total_original
            };
            let step = if decimation > 1 { decimation } else { 1 };

            let volume = crate::load!(crate::state::SPEAKER_VOLUME);
            let vol_factor = volume as f32 / 100.0;

            // TEMPORARY BUFFER FOR DOWNSAMPLED i16 SAMPLES
            let mut downsampled = [0i16; nanomp3::MAX_SAMPLES_PER_FRAME];
            let mut out_idx = 0;

            for i in (0..total_original).step_by(step) {
                let s = (pcm_f32[i].clamp(-1.0, 1.0) * vol_factor * 32767.0) as i16;
                downsampled[out_idx] = s;
                out_idx += 1;
            }

            let byte_slice = unsafe {
                core::slice::from_raw_parts(downsampled.as_ptr() as *const u8, out_idx * 2)
            };

            let mut written = 0;
            while written < byte_slice.len() {
                let w = yo_esp::play(&byte_slice[written..]);
                if w == 0 {
                    embassy_time::Timer::after_micros(100).await;
                } else {
                    written += w;
                }
            }
        }
    }

    STOP_SIGNAL.reset();
    defmt::info!("🎵 MP3 playback finished: {}", path.as_str());
}

// M3U PLAYLIST PARSER
fn parse_m3u(data: &str) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut pending_title: Option<String> = None;
    let mut next_id: u32 = 1;

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if line.starts_with("#EXTINF:") {
            if let Some(comma_idx) = line.find(',') {
                let title = String::from(line[comma_idx+1..].trim());
                pending_title = Some(title);
            } else {
                pending_title = None;
            }
        } else if line.starts_with('#') {
            pending_title = None;
        } else {
            let path = String::from(line);
            let title = pending_title.take().unwrap_or_else(|| {
                path.rsplit('/').next().unwrap_or(&path).to_string()
            });
            tracks.push(Track { id: next_id, title, file_path: path });
            next_id += 1;
        }
    }
    tracks
}

// FETCH/UPDATE THE PLAYLIST
pub async fn fetch_playlist(stack: embassy_net::Stack<'_>, url: &str) -> Result<(), &'static str> {
    let mut buf = [0u8; 4096];
    let resp = tinyapi::http_get(stack, url, &mut buf).await.map_err(|_| "HTTP GET failed")?;
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
    defmt::info!("Playlist updated with {} tracks", playlist_len());
    Ok(())
}
