// APPLICATIONS/MEDIA_PLAYER
// MEDIA PLAYER WITH MP3 DECODING & DOWNSAMPLING.
// PLAYBACK USAGE:
// crate::spawn!(spawner, crate::applications::media_player::play_mp3_task(alloc::string::ToString::to_string("/Music/MySong.mp3")));

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "Qwackify",
    description: "Play MP3 songs from the SD card",
    grid_position: crate::applications::GridSlot::TopLeft,
    launch: open_app,
    icon: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/apps/qwackify.png")),
};

pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 10);
}

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

// TOTAL TRACK DURATION IN MILLISECONDS
pub static TRACK_DURATION_MS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
// CURRENT PLAYBACK POSITION IN MILLISECONDS
pub static TRACK_POSITION_MS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
// CURRENTLY PLAYING TRACK TITLE
pub static CURRENT_TRACK_TITLE: critical_section::Mutex<core::cell::RefCell<Option<String>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

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

// HELPERS
fn playlist_len() -> usize {
    critical_section::with(|cs| PLAYLIST.borrow_ref(cs).len())
}

// PARSE A 4‑BYTE MPEG AUDIO FRAME HEADER, RETURNING (BITRATE, SAMPLE_RATE, FRAME_LENGTH)
fn parse_mp3_frame_header(header: &[u8]) -> Option<(u32, u32, u32)> {
    if header.len() < 4 { return None; }
    let sync = (header[0] as u32) << 3 | (header[1] as u32) >> 5;
    if sync != 0x7FF { return None; }

    let version_index = ((header[1] >> 3) & 0x03) as usize;
    let layer_index = ((header[1] >> 1) & 0x03) as usize;
    let bitrate_index = ((header[2] >> 4) & 0x0F) as usize;
    let sample_rate_index = ((header[2] >> 2) & 0x03) as usize;

    // TABLES FOR MPEG1 LAYER3
    let bitrate_table: [[u32; 16]; 4] = [
        // MPEG2.5, MPEG2, MPEG1, RESERVED
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0], // LAYER3 ONLY
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0], // LAYER3 ONLY
        [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];
    let sample_rate_table: [[u32; 4]; 4] = [
        [11025, 12000, 8000, 0],
        [0, 0, 0, 0],
        [22050, 24000, 16000, 0],
        [44100, 48000, 32000, 0],
    ];

    let bitrate = if bitrate_index > 0 { bitrate_table[version_index.min(3)][bitrate_index] * 1000 } else { 0 };
    let sample_rate = sample_rate_table[version_index.min(3)][sample_rate_index];
    if bitrate == 0 || sample_rate == 0 { return None; }

    let padding = ((header[2] >> 1) & 0x01) as u32;
    let frame_length = (144 * bitrate / sample_rate) + padding;

    Some((bitrate, sample_rate, frame_length))
}

// ROUGH ESTIMATE OF MP3 DURATION (BASED ON CBR ASSUMPTION)
fn estimate_mp3_duration_ms(file_path: &str) -> u32 {
    let file_size_bytes = crate::components::storage::file_size(file_path).unwrap_or(0);
    defmt::debug!("file_size = {} bytes", file_size_bytes);
    if file_size_bytes == 0 { return 0; }

    let mut file = match crate::components::storage::open_file_stream(file_path) {
        Ok(f) => f,
        Err(_) => {
            defmt::error!("estimated duration: open_file_stream failed");
            return 0;
        }
    };

    let mut buf = [0u8; 4096];
    let mut header = [0u8; 4];
    let mut total_read = 0;

    // IF FILE STARTS WITH ID3v2 TAG SKIP!,
    let mut id3_skip = 0u32;
    let n = file.read(&mut buf).unwrap_or(0);
    if n >= 10 && &buf[0..3] == b"ID3" {
        // ID3v2 SIZE IS SYNCHSAFE INT AT BYTES 6-9 ( 7 BITS PER BYTE )
        let sz = ((buf[6] as u32) << 21) | ((buf[7] as u32) << 14) |
                 ((buf[8] as u32) << 7)  | (buf[9] as u32);
        id3_skip = sz + 10;  // TAG HEADER+BODY
        defmt::debug!("estimate: ID3v2 tag found, skipping {} bytes", id3_skip);
    }
    // ALREADY READ N BYTES, MAY NEED TO SKIP FURTHER IF ID3_SKIP > N.
    // hmm.. RESET STREAM? JUST RE-OPEN IT & SKIP AHEAD.

    // RE-OPEN TO START FROM BEGINNING, THEN SKIP THE ID3 TAG IF PRESENT
    drop(file); // CLOSE CURRENT STREAM
    let mut file = match crate::components::storage::open_file_stream(file_path) {
        Ok(f) => f,
        Err(_) => {
            defmt::error!("estimate: re-open failed");
            return 0;
        }
    };

    if id3_skip > 0 {
        // SKIP BYTES BY READING AND DISCARDING
        let mut skipped = 0;
        while skipped < id3_skip {
            let to_read = buf.len().min((id3_skip - skipped) as usize);
            let n = file.read(&mut buf[..to_read]).unwrap_or(0);
            if n == 0 { break; }
            skipped += n as u32;
        }
        defmt::debug!("estimate: skipped {} bytes after ID3", skipped);
    }

    // NOW SEARCH FOR FIRST VALID FRAME
    loop {
        let n = file.read(&mut buf).unwrap_or(0);
        if n == 0 {
            defmt::warn!("estimate: EOF before valid header");
            return 0;
        }
        total_read += n;

        for i in 0..n.saturating_sub(3) {
            if buf[i] == 0xFF && (buf[i + 1] & 0xE0) == 0xE0 {
                header.copy_from_slice(&buf[i..i + 4]);
                if let Some((bitrate, sample_rate, _)) = parse_mp3_frame_header(&header) {
                    if bitrate > 0 {
                        defmt::debug!("estimate: valid header at offset {} (bitrate={}, sample_rate={})",
                                     total_read - n + i, bitrate, sample_rate);
                        let duration_ms = (file_size_bytes as u64 * 8 * 1000 / bitrate as u64) as u32 / 2;
                        defmt::debug!("estimate: duration {} ms", duration_ms);
                        return duration_ms;
                    }
                }
                // PARSE FAILED – CONTINUE SCANNING
                defmt::trace!("estimate: false sync at offset {}", total_read - n + i);
            }
        }
    }
}

pub fn current_track_title() -> Option<String> {
    critical_section::with(|cs| {
        CURRENT_TRACK_TITLE.borrow_ref(cs).clone()
    })
}

// PUBLIC API
pub async fn handle_action(spawner: embassy_executor::Spawner, action: &str) -> String {
    match action {
        "play" => { let _ = start_playback(spawner).await; String::from("Playing") }
        "pause" => { pause(); String::from("Paused") }
        "next" => { next(spawner).await; String::from("Next track") }
        "prev" => { prev(spawner).await; String::from("Previous track") }
        "stop" => { stop(); String::from("Stopped") }
        "status" => current_track_title().unwrap_or_else(|| String::from("Not playing")),
        "volume_up" => { volume_up(); String::from("Volume up") }
        "volume_down" => { volume_down(); String::from("Volume down") }
        _ => {
            defmt::info!("Unknown media action: {}", action);
            String::from("Unknown action")
        }
    }
}


async fn start_playback(spawner: embassy_executor::Spawner) -> Result<(), &'static str> {
    let pl_len = playlist_len();
    let (track_title, track_path) = if pl_len > 0 {
        // NORMAL PLAYLIST PATH
        critical_section::with(|cs| {
            let pl = PLAYLIST.borrow_ref(cs);
            let player = PLAYER.borrow_ref(cs);
            let idx = player.current_track_index % pl_len;
            (pl[idx].title.clone(), pl[idx].file_path.clone())
        })
    } else {
        // PLAYLIST EMPTY – PICK ANY SONG FROM `/Music` ON THE SD CARD
        // USE A GENERIC QUERY THAT MATCHES ANY .MP3 FILENAME
        if let Some((name, _score)) = crate::components::storage::search_song(".") {
            let title = name.clone();
            let path = alloc::format!("/Music/{}", name);
            (title, path)
        } else { return Err("No songs found on SD card"); }
    };

    let duration_ms = estimate_mp3_duration_ms(&track_path);
    crate::store!(TRACK_DURATION_MS, duration_ms);
    defmt::debug!("Track duration: {} ms", duration_ms);

    // STORE TRACK TITLE
    critical_section::with(|cs| {
        *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title.clone());
    });

    // IF SOMETHING IS ALREADY PLAYING WE STOP IT
    stop();

    // SPAWN THE MP3 PLAYBACK TASK
    crate::spawn!(spawner, play_mp3_task(track_path.clone()));

    critical_section::with(|cs| {
        PLAYER.borrow_ref_mut(cs).state = PlaybackState::Playing;
    });
    crate::store!(crate::state::MEDIA_IS_PLAYING, true);
    defmt::debug!("Now playing: {}", track_title.as_str());
    Ok(())
}

fn pause() {
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        if player.state == PlaybackState::Playing {
            stop();
            player.state = PlaybackState::Paused;
            defmt::info!("🎵 Playback paused");
            crate::store!(crate::state::MEDIA_IS_PLAYING, false);
        } else if player.state == PlaybackState::Paused {
            defmt::debug!("Press play to resume");
        }
    });
}

fn stop() {
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        if player.state != PlaybackState::Stopped {
            STOP_SIGNAL.signal(());
            player.state = PlaybackState::Stopped;
            defmt::info!("Playback stopped");
            crate::store!(crate::state::MEDIA_IS_PLAYING, false);
            // CLEAR TITLE
            *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
        }
    });
}

// NEXT TRACK
async fn next(spawner: embassy_executor::Spawner) {
    let pl_len = playlist_len();
    if pl_len == 0 { return; }

    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = (player.current_track_index + 1) % pl_len;
        let title = &PLAYLIST.borrow_ref(cs)[player.current_track_index].title;
        defmt::info!("🎵 Skipped to next track: {}", title.as_str());
    });

    let was_playing = critical_section::with(|cs| PLAYER.borrow_ref(cs).state) == PlaybackState::Playing;
    if was_playing { let _ = start_playback(spawner).await; }
}

// PREVIOUS TRACK
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

// INCREASE VOLUME
pub fn volume_up() {
    let current = crate::load!(crate::state::SPEAKER_VOLUME);
    let new = (current + 10).min(100); // +10 STEP SIZE
    crate::set_speaker_volume(new);
}

// DECREASE VOLUME
pub fn volume_down() {
    let current = crate::load!(crate::state::SPEAKER_VOLUME);
    let new = current.saturating_sub(10); // -10 STEP SIZE
    crate::set_speaker_volume(new);
}


// ASYNC MP3 PLAYBACK TASK
#[embassy_executor::task]
pub async fn play_mp3_task(path: String) {
    defmt::info!("🎵 MP3 playback started: {}", path.as_str());

    let mut total_samples_decoded: u64 = 0;
    let mut sample_rate_once: u32 = 0;

    let mut file = match crate::components::storage::open_file_stream(&path) {
        Ok(f) => f,
        Err(e) => {
            defmt::error!("Failed to open {}: {:?}", path.as_str(), e);
            return;
        }
    };

    // DECODE THE MP3
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

            if sample_rate_once == 0 { sample_rate_once = info.sample_rate; }
            total_samples_decoded += info.samples_produced as u64;
            let elapsed_ms = (total_samples_decoded * 1000 / sample_rate_once as u64) as u32;
            crate::store!(TRACK_POSITION_MS, elapsed_ms);

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
    crate::store!(TRACK_POSITION_MS, 0);
    crate::store!(TRACK_DURATION_MS, 0);    
    critical_section::with(|cs| *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None);
    crate::store!(crate::state::MEDIA_IS_PLAYING, false);
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
    defmt::info!("🎵 Playlist updated with {} tracks", playlist_len());
    Ok(())
}

// TASK TO CONSUME TOUCH COMMANDS FROM THE ATOMIC `MEDIA_COMMAND`
// SENT FROM THE GRAPHICAL USER INTERFACE
#[embassy_executor::task]
pub async fn media_command_task(spawner: embassy_executor::Spawner) {
    use core::sync::atomic::Ordering;
    use crate::state::MEDIA_COMMAND;
    use crate::state::MediaCommand;

    loop {
        embassy_time::Timer::after_millis(50).await;

        let cmd_byte = MEDIA_COMMAND.swap(0, Ordering::Relaxed);
        let cmd = MediaCommand::from(cmd_byte);
        match cmd { // PREVIOUS TRACK
            MediaCommand::Prev => {
                defmt::debug!("Received command: Previous track");
                if playlist_len() > 0 {
                    prev(spawner).await;
                }
            } // PLAY/PAUSE
            MediaCommand::PlayPause => {
                defmt::debug!("Received command: Play/Pause media");
                if playlist_len() == 0 {
                    // NO TRACKS IN PLAYLIST – TRY TO START PLAYBACK ANYWAY (WILL PICK ANY MP3)
                    let _ = start_playback(spawner).await;
                } else {
                    let state = critical_section::with(|cs| PLAYER.borrow_ref(cs).state);
                    match state {
                        PlaybackState::Playing => pause(),
                        PlaybackState::Paused | PlaybackState::Stopped => {
                            let _ = start_playback(spawner).await;
                        }
                    }
                }
            } // NEXT TRACK
            MediaCommand::Next => {
                defmt::debug!("Received command: Next track");
                if playlist_len() > 0 {
                    next(spawner).await;
                }
            }
            MediaCommand::None => {}
        }
    }
}
