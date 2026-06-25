// APPLICATIONS/MEDIA_PLAYER
// MEDIA PLAYER WITH MP3 DECODING & DOWNSAMPLING.
// + PLAYLIST MANAGEMENT AND FAVOURITES LIST SUPPORT
// THIS IS Qwackify

// ───────────────────────────────────────────────────────────────────────
// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "Qwackify",
    description: "Play MP3 songs from the SD card",
    launch: open_app,
    icon: crate::base::assets::QWACKIFY_PNG,
};

pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, crate::gui::pages::Page::MediaPlayer as u8);
}

// ───────────────────────────────────────────────────────────────────────

use alloc::string::String;
use alloc::vec::Vec;
use alloc::string::ToString;

// ───────────────────────────────────────────────────────────────────────
// TYPES & GLOBAL STATE
#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum PlaybackCommand {
    Play,
    Pause,
    Next,
    Prev,
    Stop,
    Clear,
    Heart,
    PlayTrack(usize),
}

pub static PLAYBACK_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    PlaybackCommand,
    4,
> = embassy_sync::channel::Channel::new();


#[derive(Clone, Copy, Debug, PartialEq, defmt::Format)]
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

// ───────────────────────────────────────────────────────────────────────
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


// ───────────────────────────────────────────────────────────────────────
// HELPERS
pub fn playlist_len() -> usize {
    critical_section::with(|cs| PLAYLIST.borrow_ref(cs).len())
}

// FETCH A FRESH COPY OF THE PLAYLIST TITLES (ALLOCATED)
pub fn playlist_titles() -> heapless::Vec<heapless::String<64>, 32> {
    critical_section::with(|cs| {
        let pl = PLAYLIST.borrow_ref(cs);
        pl.iter().map(|t| heapless::String::<64>::try_from(t.title.as_str()).unwrap()).collect()
    })
}


// ───────────────────────────────────────────────────────────────────────
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


// ───────────────────────────────────────────────────────────────────────
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


// ───────────────────────────────────────────────────────────────────────
// GET CURRENTLY PLAYING TRACK
pub fn current_track_title() -> Option<String> {
    critical_section::with(|cs| {
        CURRENT_TRACK_TITLE.borrow_ref(cs).clone()
    })
}


// ───────────────────────────────────────────────────────────────────────
// PLAY
pub async fn play() -> Result<(), &'static str> {
    let _ = PLAYBACK_CMD.send(PlaybackCommand::Play).await;
    Ok(())
}

// PAUSE
pub async fn pause() -> Result<(), &'static str> {
    let _ = PLAYBACK_CMD.send(PlaybackCommand::Pause).await;
    Ok(())
}

// PLAY/PAUSE
pub async fn play_pause() -> Result<(), &'static str> {
    if !crate::load!(crate::state::SD_READY) {
        crate::components::storage::ensure_sd_ready();
        embassy_time::Timer::after_secs(1).await;
    }
    let is_playing = crate::load!(crate::state::MEDIA_IS_PLAYING);
    if is_playing { 
        let _ = PLAYBACK_CMD.send(PlaybackCommand::Pause).await;
    } else { let _ = PLAYBACK_CMD.send(PlaybackCommand::Play).await; }
    Ok(())
}

// NEXT TRACK
pub async fn next() {
    let _ = PLAYBACK_CMD.send(PlaybackCommand::Next).await;
}

// PREVIOUS TRACK
pub async fn prev() {
    let _ = PLAYBACK_CMD.send(PlaybackCommand::Prev).await;
}

// HEART
pub async fn heart() {
    let _ = PLAYBACK_CMD.send(PlaybackCommand::Heart).await;
}

// CLEAR
pub async fn clear() {
    let _ = PLAYBACK_CMD.send(PlaybackCommand::Clear).await;
}

// PLAY FAVOURITES
pub async fn play_favourites() {
    if load_playlist_from_sd("/Music/favourit.m3u").is_ok() { let _ = PLAYBACK_CMD.send(PlaybackCommand::Play).await; }
}

// ───────────────────────────────────────────────────────────────────────
// PLAY NOW (SYNCHRONOUS – NON‑BLOCKING)
pub fn play_now() {
    if let Err(e) = PLAYBACK_CMD.try_send(PlaybackCommand::Play) {
        defmt::warn!("play_now: failed to send Play command ({})", defmt::Debug2Format(&e));
    }
}

pub fn pause_now() { 
    let _ = PLAYBACK_CMD.try_send(PlaybackCommand::Pause); 
}
pub fn stop_now()  { let _ = PLAYBACK_CMD.try_send(PlaybackCommand::Stop); }
pub fn next_now()  { let _ = PLAYBACK_CMD.try_send(PlaybackCommand::Next); }
pub fn prev_now()  { let _ = PLAYBACK_CMD.try_send(PlaybackCommand::Prev); }

// ───────────────────────────────────────────────────────────────────────
// INCREASE VOLUME
pub fn volume_up() {
    let current = crate::load!(crate::state::SPEAKER_VOLUME);
    let new = (current + 5).min(75); // +5 STEP SIZE
    crate::set_speaker_volume(new);
}

// DECREASE VOLUME
pub fn volume_down() {
    let current = crate::load!(crate::state::SPEAKER_VOLUME);
    let new = current.saturating_sub(5); // -5 STEP SIZE
    crate::set_speaker_volume(new);
}


// ───────────────────────────────────────────────────────────────────────
// GET CURRENT TRACK INFO FROM PLAYLIST (OR FALLBACK)
fn current_track_info() -> (String, String) {
    let pl_len = playlist_len();
    if pl_len > 0 {
        critical_section::with(|cs| {
            let pl = PLAYLIST.borrow_ref(cs);
            let player = PLAYER.borrow_ref(cs);
            let idx = player.current_track_index % pl_len;
            (pl[idx].title.clone(), pl[idx].file_path.clone())
        })
    } else {
        // EMPTY/NO PLAYTLIST – GENERATE ONE        
        // ENSURE STORAGE IS INIT
        if !crate::load!(crate::state::SD_READY) {
            crate::components::storage::ensure_sd_ready();
        } 
        crate::components::storage::generate_playlist("duck");
        let _ = load_playlist_from_sd("/Music/playlist.m3u");
        if playlist_len() > 0 {
            critical_section::with(|cs| {
                let pl = PLAYLIST.borrow_ref(cs);
                let player = PLAYER.borrow_ref(cs);
                let idx = player.current_track_index % playlist_len();
                (pl[idx].title.clone(), pl[idx].file_path.clone())
            })
        } else {
            // FALLBACK - PICK ANY SONG
            if let Some((name, _score)) = crate::components::storage::search_song(".") {
                let path = alloc::format!("/Music/{}", name);
                (name.clone(), path)
            } else {
                (String::from("Unknown"), String::new())
            }
        }
    }    
}


// ───────────────────────────────────────────────────────────────────────
// TRACK INDEX MOVER
fn advance_track(cmd: PlaybackCommand) {
    let pl_len = playlist_len();
    if pl_len == 0 { return; }
    critical_section::with(|cs| {
        let mut player = PLAYER.borrow_ref_mut(cs);
        match cmd {
            PlaybackCommand::Next => {
                player.current_track_index = (player.current_track_index + 1) % pl_len;
            }
            PlaybackCommand::Prev => {
                player.current_track_index = if player.current_track_index == 0 {
                    pl_len - 1
                } else {
                    player.current_track_index - 1
                };
            }
            _ => {}
        }
    });
}

// GET FILE PATH FOR A TRACK INDEX IN THE PLAAYLIST
pub fn playlist_track(index: usize) -> Option<(String, String)> {
    critical_section::with(|cs| {
        let pl = PLAYLIST.borrow_ref(cs);
        pl.get(index).map(|t| (t.title.clone(), t.file_path.clone()))
    })
}


// ───────────────────────────────────────────────────────────────────────
// ASYNC MP3 PLAYBACK TASK
#[embassy_executor::task]
pub async fn playback_task(_spawner: embassy_executor::Spawner) {
    let mut state = PlaybackState::Stopped;
    let mut current_file: Option<crate::components::storage::SdFileStream> = None;
    let mut decoder = nanomp3::Decoder::new();
    let mut total_samples_decoded: u64 = 0;
    let mut sample_rate_once: u32 = 0;
    // REUSABLE BUFFERS
    let mut pcm_f32 = [0.0f32; nanomp3::MAX_SAMPLES_PER_FRAME];
    let mut mp3_buffer = [0u8; 4096];
    let mut pos = 0;
    let mut valid = 0;
    let mut track_start: Option<embassy_time::Instant> = None;

    loop {
        match state {
        
            // ───────────────────────────────────────────────────────────────────────
            // STATE: STOPPED / PAUSED
            PlaybackState::Stopped | PlaybackState::Paused => {
                // IDLE - WAIT FOR A COMMAND
                let cmd = PLAYBACK_CMD.receive().await;
                match cmd {
                
                    // ───────────────────────────────────────────────────────────────────────
                    // PLAY COMMAND (WHILE STOPPED/PAUSED)
                    PlaybackCommand::Play => {
                        // ENSURE WE HAVE SPEAKER VOLUME
                        if crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
                            crate::set_speaker_volume(65);
                        }
                        // IF WE ARE PAUSED & HAVE AN OPEN FILE, JUST RESUME PLAYBACK
                        if state == PlaybackState::Paused && current_file.is_some() {
                            state = PlaybackState::Playing;
                            critical_section::with(|cs| {
                                PLAYER.borrow_ref_mut(cs).state = PlaybackState::Playing;
                            });
                            crate::store!(crate::state::MEDIA_IS_PLAYING, true);
                            continue; // JUMP BACK TO THE DECODING LOOP
                        }

                        // START PLAYING PLAYLIST (IF PLAYLIST IS EMPTY IT WILL GENERATE ONE)
                        let (track_title, track_path) = current_track_info();
                        defmt::info!("🎵 Playing: {}", track_title.as_str());
                        
                        // COMPUTE DURATION BEFORE OPENING PLAYBACK STREAM (SO THE VOLUME IS FREE)
                        let duration_ms = estimate_mp3_duration_ms(&track_path);
                        crate::store!(TRACK_DURATION_MS, duration_ms);

                        // DO A FAVOURITE CHECK (AND UPDATE GUI ACCORDINGLY)
                        let is_fav = crate::components::storage::check_favourites(&track_title);
                        if is_fav {
                            defmt::info!("❤️ Favourite: YES");
                        } else { defmt::info!("❤️ Favourite: NO"); }

                        // START STREAMING FILE CONTENT
                        match crate::components::storage::open_file_stream(&track_path) {
                            Ok(f) => {
                                current_file = Some(f);
                                // RESET DECODER & COUNTERS
                                decoder = nanomp3::Decoder::new();
                                total_samples_decoded = 0;
                                sample_rate_once = 0;
                                pos = 0;
                                valid = 0;
                                track_start = Some(embassy_time::Instant::now());
                                // UPDATE GLOBAL STATE
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title);
                                });
                                crate::store!(crate::state::MEDIA_IS_PLAYING, true);
                                state = PlaybackState::Playing;
                                // UPDATE CACHED PLAYLIST IN GUI
                                crate::gui::media_player::invalidate_playlist();
                            }
                            Err(e) => { defmt::error!("Failed to open {}: {:?}", track_path.as_str(), e); }
                        }
                    }
                    
                    // ───────────────────────────────────────────────────────────────────────
                    // PLAY TRACK COMMAND (WHILE STOPPED/PAUSED)
                    PlaybackCommand::PlayTrack(idx) => {
                        let pl_len = playlist_len();
                        if pl_len == 0 { continue; }
                        let new_idx = idx % pl_len;
                        // UPDATE THE PLAYERS CURRENT TRACK INDEX
                        critical_section::with(|cs| {
                            PLAYER.borrow_ref_mut(cs).current_track_index = new_idx;
                        });
                        
                        // START PLAY LOGIC FOR THE SPECIFIED TRACK
                        let (track_title, track_path) = current_track_info();
                        defmt::info!("🎵 Playing: {}", track_title.as_str());
                        
                        // COMPUTE DURATION BEFORE OPENING PLAYBACK STREAM (SO THE VOLUME IS FREE)
                        let duration_ms = estimate_mp3_duration_ms(&track_path);
                        crate::store!(TRACK_DURATION_MS, duration_ms);

                        // DO A FAVOURITE CHECK (AND UPDATE GUI ACCORDINGLY)
                        let is_fav = crate::components::storage::check_favourites(&track_title);
                        if is_fav {
                            defmt::info!("❤️ Favourite: YES");
                        } else { defmt::info!("❤️ Favourite: NO"); }

                        // START STREAMING FILE CONTENT
                        match crate::components::storage::open_file_stream(&track_path) {
                            Ok(f) => {
                                current_file = Some(f);
                                // RESET DECODER & COUNTERS
                                decoder = nanomp3::Decoder::new();
                                total_samples_decoded = 0;
                                sample_rate_once = 0;
                                pos = 0;
                                valid = 0;
                                track_start = Some(embassy_time::Instant::now());
                                // UPDATE GLOBAL STATE
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title);
                                });
                                crate::store!(crate::state::MEDIA_IS_PLAYING, true);
                                state = PlaybackState::Playing;
                                // UPDATE CACHED PLAYLIST IN GUI
                                crate::gui::media_player::invalidate_playlist();
                            }
                            Err(e) => { defmt::error!("Failed to open {}: {:?}", track_path.as_str(), e); }
                        }
                    }
                    
                    
                    // ───────────────────────────────────────────────────────────────────────
                    // PREVIOUS / NEXT COMMAND (WHILE STOPPED/PAUSED)
                    PlaybackCommand::Next | PlaybackCommand::Prev => {
                        advance_track(cmd);
                        let (track_title, track_path) = current_track_info();
                        defmt::info!("🎵 Playing: {}", track_title.as_str());

                        let duration_ms = estimate_mp3_duration_ms(&track_path);
                        crate::store!(TRACK_DURATION_MS, duration_ms);

                        let is_fav = crate::components::storage::check_favourites(&track_title);
                        if is_fav {
                            defmt::info!("❤️ Favourite: YES");
                        } else { defmt::info!("❤️ Favourite: NO"); }

                        match crate::components::storage::open_file_stream(&track_path) {
                            Ok(f) => {
                                current_file = Some(f);
                                decoder = nanomp3::Decoder::new();
                                total_samples_decoded = 0;
                                sample_rate_once = 0;
                                pos = 0;
                                valid = 0;
                                track_start = Some(embassy_time::Instant::now()); 
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title);
                                });
                                crate::store!(crate::state::MEDIA_IS_PLAYING, true);
                                state = PlaybackState::Playing;
                                // UPDATE CACHED PLAYLIST IN GUI
                                crate::gui::media_player::invalidate_playlist();
                            }
                            Err(e) => { defmt::error!("Failed to open {}: {:?}", track_path.as_str(), e); }
                        }
                    }

                    // ───────────────────────────────────────────────────────────────────────
                    // PAUSE COMMAND (WHILE STOPPED/PAUSED)
                    PlaybackCommand::Pause => {
                        // ALREADY PAUSED/STOPPED - DO NOTHING
                    }
                    
                    // ───────────────────────────────────────────────────────────────────────
                    // STOP COMMAND (WHILE STOPPED/PAUSED)
                    PlaybackCommand::Stop => {
                        // STOP & CLEAR STATE
                        current_file = None;
                        critical_section::with(|cs| {
                            *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                            PLAYER.borrow_ref_mut(cs).state = PlaybackState::Stopped;
                        });
                        crate::store!(TRACK_POSITION_MS, 0);
                        crate::store!(TRACK_DURATION_MS, 0);
                        crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                        if let Err(e) = crate::components::storage::flush_favourites_cache() {
                            defmt::error!("Failed to flush favourites: {:?}", e);
                        }
                        state = PlaybackState::Stopped;
                    }

                    // ───────────────────────────────────────────────────────────────────────
                    // UNKNOWN COMMAND (WHILE STOPPED/PAUSED)
                    _ => { defmt::info!("Command {:?} ignored in {:?} state", cmd, state); }
                }
            }
 
            // ───────────────────────────────────────────────────────────────────────
            // STATE: PLAYING
            PlaybackState::Playing => {
            
                // CHECK FOR NEW COMMANDS WITHOUT BLOCKING
                if let Ok(cmd) = PLAYBACK_CMD.try_receive() {
                    match cmd {

                        // ───────────────────────────────────────────────────────────────────────
                        // PAUSE COMMAND (WHILE PLAYING)
                        PlaybackCommand::Pause => {
                            state = PlaybackState::Paused;
                            crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                            critical_section::with(|cs| {
                                PLAYER.borrow_ref_mut(cs).state = PlaybackState::Paused;
                            });
                            continue;
                        }
                        
                        // ───────────────────────────────────────────────────────────────────────
                        // STOP COMMAND (WHILE PLAYING)
                        PlaybackCommand::Stop => {
                            state = PlaybackState::Stopped;
                            current_file = None;
                            critical_section::with(|cs| {
                                *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                                PLAYER.borrow_ref_mut(cs).state = PlaybackState::Stopped;
                            });
                            crate::store!(TRACK_POSITION_MS, 0);
                            crate::store!(TRACK_DURATION_MS, 0);
                            crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                            if let Err(e) = crate::components::storage::flush_favourites_cache() {
                                defmt::error!("Failed to flush favourites: {:?}", e);
                            }
                            continue;
                        }
                        
                        // ───────────────────────────────────────────────────────────────────────
                        // PREVIOUS / NEXT COMMAND (WHILE PLAYING)
                        PlaybackCommand::Next | PlaybackCommand::Prev => {
                            // DROP THE CURRENT FILE FIRST TO RELEASE THE BORROW (AND VOLUME)
                            drop(current_file.take());
                            // VOLUME NOW FREE – COMPUTE THE DURATION SAFELY
                            advance_track(cmd);
                            let (track_title, track_path) = current_track_info();
                            defmt::info!("🎵 Skipped to: {}", track_title.as_str());
                            
                            let duration_ms = estimate_mp3_duration_ms(&track_path);
                            crate::store!(TRACK_DURATION_MS, duration_ms);

                            // DO A FAVOURITE CHECK (AND UPDATE THE GUI)
                            let is_fav = crate::components::storage::check_favourites(&track_title);
                            if is_fav {
                                defmt::info!("❤️ Favourite: YES");
                            } else { defmt::info!("❤️ Favourite: NO"); }

                            // START STREAMING FILE CONTENT
                            if let Ok(new_file) = crate::components::storage::open_file_stream(&track_path) {
                                current_file = Some(new_file);
                                decoder = nanomp3::Decoder::new();
                                total_samples_decoded = 0;
                                sample_rate_once = 0;
                                pos = 0;
                                valid = 0;
                                track_start = Some(embassy_time::Instant::now()); 
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title);
                                });
                            } else {
                                // FAILED TO OPEN NEW FILE – STOP
                                state = PlaybackState::Stopped;
                                current_file = None;
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                                });
                                crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                                if let Err(e) = crate::components::storage::flush_favourites_cache() {
                                    defmt::error!("Failed to flush favourites: {:?}", e);
                                }
                                continue;
                            }
                            // CONTINUE THE DECODING LOOP WITH THE NEW FILE
                        }
                        
                        // ───────────────────────────────────────────────────────────────────────
                        // PLAY COMMAND (WHILE PLAYING)
                        PlaybackCommand::Play => {
                            // ALREADY PLAYING - IGNORE
                        }
                        
                        // ───────────────────────────────────────────────────────────────────────
                        // PLAY TRACK COMMAND (WHILE PLAYING)                        
                        PlaybackCommand::PlayTrack(idx) => {
                            // DROP THE CURRENT FILE FIRST TO RELEASE THE BORROW (AND VOLUME)
                            drop(current_file.take());
                            // VOLUME NOW FREE – SET THE NEW INDEX
                            let pl_len = playlist_len();
                            if pl_len == 0 { state = PlaybackState::Stopped; continue; }
                            let new_idx = idx % pl_len;
                            critical_section::with(|cs| {
                                PLAYER.borrow_ref_mut(cs).current_track_index = new_idx;
                            });
                            // START THE NEW TRACK
                            let (track_title, track_path) = current_track_info();
                            defmt::info!("🎵 Skipped to: {}", track_title.as_str());
                            
                            let duration_ms = estimate_mp3_duration_ms(&track_path);
                            crate::store!(TRACK_DURATION_MS, duration_ms);

                            // DO A FAVOURITE CHECK (AND UPDATE THE GUI)
                            let is_fav = crate::components::storage::check_favourites(&track_title);
                            if is_fav {
                                defmt::info!("❤️ Favourite: YES");
                            } else { defmt::info!("❤️ Favourite: NO"); }

                            // START STREAMING FILE CONTENT
                            if let Ok(new_file) = crate::components::storage::open_file_stream(&track_path) {
                                current_file = Some(new_file);
                                decoder = nanomp3::Decoder::new();
                                total_samples_decoded = 0;
                                sample_rate_once = 0;
                                pos = 0;
                                valid = 0;
                                track_start = Some(embassy_time::Instant::now()); 
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title);
                                });
                            } else {
                                // FAILED TO OPEN NEW FILE – STOP
                                state = PlaybackState::Stopped;
                                current_file = None;
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                                });
                                crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                                if let Err(e) = crate::components::storage::flush_favourites_cache() {
                                    defmt::error!("Failed to flush favourites: {:?}", e);
                                }
                                continue;
                            }
                            // CONTINUE THE DECODING LOOP WITH THE NEW FILE
                        }   
                        
                        // ───────────────────────────────────────────────────────────────────────
                        // CLEAR COMMAND (WHILE PLAYING)
                        PlaybackCommand::Clear => {
                            // CLEAR THE PLAYLIST
                            defmt::debug!("Received Clear command");
                            crate::components::storage::clear_playlist();
                        }
                        
                        // ───────────────────────────────────────────────────────────────────────
                        // HEART COMMAND (WHILE PLAYING)
                        PlaybackCommand::Heart => {
                            // GET CURRENT TRACK
                            let current_title = critical_section::with(|cs| {
                                CURRENT_TRACK_TITLE.borrow_ref(cs).clone()
                            });

                            // CHECK IF ITS ALREADY A FAVOURITE TRACK - IN THAT CASE REMOVE IT FROM THE FAVOURITE LIST
                            if let Some(title) = current_title {
                                let already_liked = crate::load!(crate::state::MEDIA_IS_LIKED);
                                if already_liked {
                                    crate::components::storage::cache_remove_favourite(&title);
                                    crate::store!(crate::state::MEDIA_IS_LIKED, false);
                                    defmt::info!("❤️ Removed from favourites");
                                } else { // OTHERWISE ADD IT AS A FAVOURITE TRACK
                                    crate::components::storage::cache_add_favourite(&title);
                                    crate::store!(crate::state::MEDIA_IS_LIKED, true);
                                    defmt::info!("❤️ Added to favourites (cached)");
                                } // AND REFRESH GUI
                                crate::dirty!();
                            } else { defmt::warn!("Heart command ignored – no track playing"); }
                        }
                    }
                }
            
            
                // WE NEED A MUTABLE REFERENCE TO THE FILE HERE
                // IF WE SWAPPED IT ABOVE, THAT CASE HAS ALREADY BEEN HANDLED.
                // AT THIS POINT, CURRENT_FILE IS EITHER NONE (ALREADY HANDLED)
                // OR STILL SOME, SO UNWRAPPING IT IS SAFE
                let file = match current_file.as_mut() {
                    Some(f) => f,
                    None => {
                        state = PlaybackState::Stopped;
                        continue;
                    }
                };
            

                // ───────────────────────────────────────────────────────────────────────            
                // DECODE & PLAY LOGIC
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
                            Ok(0) => {
                                // CLEAN UP CURRENT FILE
                                current_file = None;
                                // DEBUG LOG TRACK DURATION CALCULATION
                                if let Some(start) = track_start.take() {
                                    let elapsed_ms = start.elapsed().as_millis() as u32;
                                    let estimated_ms = crate::load!(TRACK_DURATION_MS);
                                    defmt::debug!(
                                        "🎵 track finished – elapsed: {} ms, estimated: {} ms, diff: {} ms",
                                        elapsed_ms,
                                        estimated_ms,
                                        (elapsed_ms as i64 - estimated_ms as i64)
                                    );
                                }
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                                    PLAYER.borrow_ref_mut(cs).state = PlaybackState::Stopped;
                                });
                                crate::store!(TRACK_POSITION_MS, 0);
                                crate::store!(TRACK_DURATION_MS, 0);
                                crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                                // FLUSH FAVOURITES CACHE IF NEEDED
                                if let Err(e) = crate::components::storage::flush_favourites_cache() {
                                    defmt::error!("failed to flush favourites: {:?}", e);
                                }
                            
                                // IF PLAYLIST HAS MORE TRACKS - JUMP TO NEXT
                                if playlist_len() > 0 {
                                    let (current_idx, is_last) = critical_section::with(|cs| {
                                        let player = PLAYER.borrow_ref(cs);
                                        let idx = player.current_track_index;
                                        let last = idx + 1 >= playlist_len();
                                        (idx, last)
                                    });

                                    // STOP AFTER ALL TRACKS HAS BEEN PLAYED
                                    if is_last {
                                        defmt::debug!("End of playlist reached ({} tracks).", playlist_len());
                                        state = PlaybackState::Stopped;
                                        continue;
                                    }

                                    // NOT LAST TRACK – PLAY NEXT ONE
                                    advance_track(PlaybackCommand::Next);
                                    let (track_title, track_path) = current_track_info();

                                    let total = playlist_len();
                                    let idx = critical_section::with(|cs| PLAYER.borrow_ref(cs).current_track_index);
                                    defmt::info!(
                                        "🎵 playing next: {} ({}/{})",
                                        track_title.as_str(),
                                        idx + 1,
                                        total
                                    );

                                    let duration_ms = estimate_mp3_duration_ms(&track_path);
                                    crate::store!(TRACK_DURATION_MS, duration_ms);

                                    let is_fav = crate::components::storage::check_favourites(&track_title);
                                    if is_fav {
                                        defmt::info!("❤️ Favourite: YES");
                                    } else { defmt::info!("❤️ Favourite: NO"); }

                                    match crate::components::storage::open_file_stream(&track_path) {
                                        Ok(f) => {
                                            current_file = Some(f);
                                            // RESET DECODER STATE
                                            decoder = nanomp3::Decoder::new();
                                            total_samples_decoded = 0;
                                            sample_rate_once = 0;
                                            pos = 0;
                                            valid = 0;
                                            track_start = Some(embassy_time::Instant::now());
                                            critical_section::with(|cs| {
                                                *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = Some(track_title);
                                            });
                                            crate::store!(crate::state::MEDIA_IS_PLAYING, true);
                                            state = PlaybackState::Playing;
                                            // UPDATE CACHED PLAYLIST IN GUI
                                            crate::gui::media_player::invalidate_playlist();
                                        }
                                        Err(e) => {
                                            defmt::error!("failed to open next track {}: {:?}", track_path.as_str(), e);
                                            state = PlaybackState::Stopped;
                                        }
                                    }
                                } else {
                                    // NO MORE TRACKS! - STAY STOPPED
                                    state = PlaybackState::Stopped;
                                }
                                continue;
                            }
                            Ok(n) => {
                                valid += n;
                                defmt::debug!("playback_task: read {} bytes, total valid={}", n, valid);
                            }
                            Err(e) => {
                                defmt::error!("playback_task: SD read error: {:?}", e);
                                state = PlaybackState::Stopped;
                                current_file = None;
                                critical_section::with(|cs| {
                                    *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                                    PLAYER.borrow_ref_mut(cs).state = PlaybackState::Stopped;
                                });
                                crate::store!(TRACK_POSITION_MS, 0);
                                crate::store!(TRACK_DURATION_MS, 0);
                                crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                                if let Err(e) = crate::components::storage::flush_favourites_cache() {
                                    defmt::error!("Failed to flush favourites: {:?}", e);
                                }
                                continue;
                            }
                        }
                    }
                }
            
                let (consumed, frame_info) = decoder.decode(&mp3_buffer[pos..valid], &mut pcm_f32);
                if consumed == 0 {
                    if valid == 0 {
                        state = PlaybackState::Stopped;
                        current_file = None;
                        critical_section::with(|cs| {
                            *CURRENT_TRACK_TITLE.borrow_ref_mut(cs) = None;
                            PLAYER.borrow_ref_mut(cs).state = PlaybackState::Stopped;
                        });
                        crate::store!(TRACK_POSITION_MS, 0);
                        crate::store!(TRACK_DURATION_MS, 0);
                        crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                        if let Err(e) = crate::components::storage::flush_favourites_cache() {
                            defmt::error!("failed to flush favourites: {:?}", e);
                        }
                        continue;
                    }
                    embassy_time::Timer::after_millis(1).await;
                    continue;
                }
                pos += consumed;
                defmt::debug!("playback_task: decoded {} samples", frame_info.map_or(0, |f| f.samples_produced));
            
                if let Some(info) = frame_info {
                    let samples = info.samples_produced;
                    let channels = info.channels.num() as usize;
                    let total_original = samples * channels;
                    let sample_rate = info.sample_rate;
            
                    if sample_rate_once == 0 { sample_rate_once = info.sample_rate; }
                    total_samples_decoded += info.samples_produced as u64;
                    let elapsed_ms = (total_samples_decoded * 1000 / sample_rate_once as u64) as u32;
                    crate::store!(TRACK_POSITION_MS, elapsed_ms);
            
                    // TARGET I2S SAMPLE RATE IS 16000 Hz
                    let decimation = (sample_rate / crate::state::I2S_SAMPLE_RATE) as usize;
                    let total_downsampled = if decimation > 1 && sample_rate % crate::state::I2S_SAMPLE_RATE == 0 {
                        total_original / decimation
                    } else {
                        total_original
                    };
                    let step = if decimation > 1 { decimation } else { 1 };
            
                    let volume = crate::load!(crate::state::SPEAKER_VOLUME);
                    let vol_factor = volume as f32 / 100.0;
            
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
        }
    }
}



// ───────────────────────────────────────────────────────────────────────
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


// ───────────────────────────────────────────────────────────────────────
// FETCH/UPDATE PLAYLIST FROM URL 
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
    // UPDATE CACHED PLAYLIST IN GUI
    crate::gui::media_player::invalidate_playlist();
    defmt::info!("🎵 Playlist updated with {} tracks", playlist_len());
    Ok(())
}


// ───────────────────────────────────────────────────────────────────────
// LOAD SDCARD PLAYLIST IINTO MEMORY
pub fn load_playlist_from_sd(path: &str) -> Result<(), crate::components::storage::SdError> {
    defmt::debug!("load_playlist_from_sd: trying to read {}", path);
    let data = crate::components::storage::read_file_to_vec(path)?;
    let body_str = core::str::from_utf8(&data).map_err(|_| {
        crate::components::storage::SdError::Read
    })?;
    let mut tracks = parse_m3u(body_str);

    for track in &mut tracks {
        if !track.file_path.starts_with('/') {
            track.file_path = alloc::format!("/Music/{}", track.file_path);
        }
    }

    critical_section::with(|cs| {
        *PLAYLIST.borrow_ref_mut(cs) = tracks;
        let mut player = PLAYER.borrow_ref_mut(cs);
        player.current_track_index = 0;
        player.state = PlaybackState::Stopped;
    });

    // UPDATE CACHED PLAYLIST IN GUI
    crate::gui::media_player::invalidate_playlist();
    defmt::info!("🎵 Loaded {} tracks from {}", playlist_len(), path);
    Ok(())
}
