// COMPONENTS/STORAGE
// MICRO SECURE DIGITAL CARD DRIVER (SPI)
// FILESYSTEM: FAT32
// PROVIDES EVERYTHING NEEDED FOR:
// SD CARD INIT ++ FUZZY SEARCHING FILES ON THE SD CARD FOR EASY VOICE COMMAND PLAYBACK  (be drunk & speak japanese, it's all good)

// ───────────────────────────────────────────────────────────────────────
// SD CARD SETUP (ON A DESKTOP)
// 1. INSERT THE SD CARD AND FIND ITS DEVICE NAME ( EXAMPLE: /dev/sdb )
//    lsblk
//
// 2. FORMAT THE WHOLE CARD AS FAT32:
//    sudo mkfs.fat -F32 /dev/sdX # REPLACE SDX WITH YOUR DEVICE
//
// 3. CREATE A TEMPORARY MOUNT POINT, MOUNT, CREATE DIRECTORIES, AND COPY FILES:
//    sudo mkdir -p /mnt/sdcard
//    sudo mount /dev/sdX /mnt/sdcard
//    sudo mkdir /mnt/sdcard/Music /mnt/sdcard/share
//    sudo cp *.mp3 /mnt/sdcard/Music/
//    sudo umount /mnt/sdcard

// 4. INSERT SD CARD INTO THE ESP32 & RUN THE COMMAND BELOW FOR AUTOMATIC TESTING:
//    cargo run --release --features sd-test
// ───────────────────────────────────────────────────────────────────────

extern crate alloc;
use alloc::{format, string::{String, ToString}, vec::Vec};

// ───────────────────────────────────────────────────────────────────────
// TYPES & GLOBAL STATE
pub enum StorageCommand {
    Enable,
    Disable,
}

pub static STORAGE_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    StorageCommand,
    1,
> = embassy_sync::channel::Channel::new();


pub enum SdState {
    NotInserted,
    Mounted,
    Error,
}

#[derive(Debug, defmt::Format)] 
pub enum SdError {
    NotInitialized,
    Volume,
    RootDir,
    Directory,
    File,
    Read,
    Write,
}

// DUMMY TIME
struct DummyTime;
impl embedded_sdmmc::TimeSource for DummyTime {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp::from_calendar(2026, 4, 6, 12, 0, 0).unwrap()
    }
}

type SpiDevice = embedded_hal_bus::spi::ExclusiveDevice<
    esp_hal::spi::master::Spi<'static, esp_hal::Blocking>,
    esp_hal::gpio::Output<'static>,
    embedded_hal_bus::spi::NoDelay,
>;
type SdCardType = embedded_sdmmc::SdCard<SpiDevice, esp_hal::delay::Delay>;
type VolumeMgrType = embedded_sdmmc::VolumeManager<SdCardType, DummyTime>;
type FileType<'a> = embedded_sdmmc::File<'a, SdCardType, DummyTime, 12, 12, 1>;

static VOL_MGR: critical_section::Mutex<core::cell::RefCell<Option<VolumeMgrType>>> = critical_section::Mutex::new(core::cell::RefCell::new(None));


// ───────────────────────────────────────────────────────────────────────
// CACHE
static FAVOURITES_CACHE: critical_section::Mutex<core::cell::RefCell<Vec<String>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(Vec::new()));

static FAVOURITES_DIRTY: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
    
// ───────────────────────────────────────────────────────────────────────
// INIT
#[embassy_executor::task]
pub async fn storage_init_task(mut card: SdCardType) {
    let mut enabled = false;
    let mut card_opt = Some(card);

    loop {
        let cmd = STORAGE_CMD.receive().await;
        match cmd {
            StorageCommand::Enable => {
                if enabled {
                    continue;
                }
                let card = match card_opt.take() {
                    Some(c) => c,
                    None => {
                        defmt::error!("Storage: card already consumed");
                        continue;
                    }
                };
                defmt::debug!("STORAGE: enabling and initialising SD card...");
                let vol_mgr = embedded_sdmmc::VolumeManager::new(card, DummyTime);
                critical_section::with(|cs| {
                    *VOL_MGR.borrow(cs).borrow_mut() = Some(vol_mgr);
                });
                enabled = true;
                crate::store!(crate::state::SD_READY, true);
                defmt::info!("STORAGE: READY!");
                
                ensure_m3u_files_exist();
                embassy_time::Timer::after_millis(100).await;
                
                match load_favourites_cache() {
                    Ok(()) => defmt::debug!("❤️ Favourites cache loaded"),
                    Err(e) => defmt::error!("❤️ Failed to load favourites cache: {:?}", e),
                }
            }
            StorageCommand::Disable => {
                // TODO: toggling
                defmt::warn!("Disable ignored – storage can only be enabled once");
            }
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// INITIATE SD CARD DRIVER (CALL FROM MAIN)
pub fn init(card: SdCardType, spawner: &embassy_executor::Spawner) {
    crate::spawn!(spawner, storage_init_task(card));
}

// ───────────────────────────────────────────────────────────────────────
// FUZZY SEARCH SONGS 
pub fn search_song(query: &str) -> Option<(String, u8)> {
    ensure_sd_ready().ok()?; // RETURNS `None` IF INIT FAILED
    
    defmt::info!("🪄 fuzzy searching: {}", query);

    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut()?;
        defmt::debug!("VolumeManager obtained");

        let volume = vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0)).ok()?;
        defmt::debug!("Volume opened");

        let root_dir = vol_mgr.open_root_dir(volume).ok()?;
        defmt::debug!("Root directory opened");

        let music_dir = vol_mgr.open_dir(root_dir, "Music").ok()?;
        defmt::info!("📁 /Music directory opened");

        let mut names = Vec::new();

        // ALLOCATE BUFFER FOR LONG FILE NAME (LFN) RECONSTRUCTION
        let mut lfn_storage = [0u8; 256];
        let mut lfn_buf = embedded_sdmmc::LfnBuffer::new(&mut lfn_storage);

        let _ = vol_mgr.iterate_dir_lfn(music_dir, &mut lfn_buf, |entry: &embedded_sdmmc::DirEntry, lfn: Option<&str>| {
            if !entry.attributes.is_directory() {
                let full = if let Some(name) = lfn {
                    name.to_string()
                } else {
                    let base = core::str::from_utf8(&entry.name.base_name()).unwrap_or("");
                    let ext  = core::str::from_utf8(&entry.name.extension()).unwrap_or("");
                    if ext.is_empty() {
                        format!("{}", base.trim())
                    } else { format!("{}.{}", base.trim(), ext.trim()) }
                };
                // PRINT EACH FOUND FILE ++ FILE SIZE
                let size = entry.size;
                defmt::info!("📁 found file: {} ({} bytes)", full.as_str(), size);
                names.push(full.as_bytes().to_vec());
            }
            core::ops::ControlFlow::Continue(())
        });
        defmt::debug!("Total files found: {}", names.len());

        if let Err(e) = vol_mgr.close_dir(music_dir) {
            defmt::error!("close_dir (music) failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = vol_mgr.close_dir(root_dir) {
            defmt::error!("close_dir (root) failed: {}", defmt::Debug2Format(&e));
        }        
        if let Err(e) = vol_mgr.close_volume(volume) {
            defmt::error!("close_volume failed: {}", defmt::Debug2Format(&e));
        }

        if names.is_empty() {
            defmt::warn!("⚠️ No files in /Music");
            return None;
        }

        // TWO-STAGE FILTER FUZZY MATCHING
        // PROVIDED BY `BARELY_FUZZY`
        // TAKES ALL FILES AND COMPARES TO INPUT & RETURNS HIGHEST SCORED FILES
        let candidates: Vec<&[u8]> = names.iter().map(|v| v.as_slice()).collect();
        let (best_bytes, score) = barely_fuzzy::best_fuz(query.as_bytes(), &candidates, 5);
        defmt::debug!("best score: {}, best bytes: {:?}", score, best_bytes);

        let best_name = core::str::from_utf8(best_bytes).ok()?.to_string();
        defmt::info!("🏆 Best match: {} ({}%)", best_name.as_str(), score);
        Some((best_name, score))
 
    })
}


pub fn search_top_n(query: &str, n: usize) -> Vec<(String, u8)> {
    if ensure_sd_ready().is_err() {
        return Vec::new();
    }
    // GRAB ALL FILENAMES FROM `/Music` (inside crit section)
    let filenames: Vec<String> = critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = match guard.as_mut() {
            Some(v) => v,
            None => return Vec::new(),
        };

        let volume = vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0)).ok();
        let volume = match volume {
            Some(v) => v,
            None => return Vec::new(),
        };
        let root_dir = vol_mgr.open_root_dir(volume).ok();
        let root_dir = match root_dir {
            Some(d) => d,
            None => { vol_mgr.close_volume(volume).ok(); return Vec::new(); }
        };
        let music_dir = vol_mgr.open_dir(root_dir, "Music").ok();
        let music_dir = match music_dir {
            Some(d) => d,
            None => { vol_mgr.close_dir(root_dir).ok(); vol_mgr.close_volume(volume).ok(); return Vec::new(); }
        };

        let mut names = Vec::new();
        let mut lfn_storage = [0u8; 256];
        let mut lfn_buf = embedded_sdmmc::LfnBuffer::new(&mut lfn_storage);

        let _ = vol_mgr.iterate_dir_lfn(music_dir, &mut lfn_buf, |entry, lfn| {
            if !entry.attributes.is_directory() {
                let name = if let Some(l) = lfn {
                    l.to_string()
                } else {
                    let base = core::str::from_utf8(&entry.name.base_name()).unwrap_or("");
                    let ext  = core::str::from_utf8(&entry.name.extension()).unwrap_or("");
                    if ext.is_empty() {
                        alloc::format!("{}", base.trim())
                    } else {
                        alloc::format!("{}.{}", base.trim(), ext.trim())
                    }
                };
                names.push(name);
            }
            core::ops::ControlFlow::Continue(())
        });

        if let Err(e) = vol_mgr.close_dir(music_dir) {
            defmt::error!("close_dir (music) failed: {}", defmt::Debug2Format(&e));
        }        
        if let Err(e) = vol_mgr.close_dir(root_dir) {
            defmt::error!("close_dir (root) failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = vol_mgr.close_volume(volume) {
            defmt::error!("close_volume failed: {}", defmt::Debug2Format(&e));
        }
        names
    });

    // RUN FUZZY MATCHING (outside critical section)
    let cands: Vec<&[u8]> = filenames.iter().map(|s| s.as_bytes()).collect();
    let matches = barely_fuzzy::best_fuz_n(query.as_bytes(), &cands, n, 5);

    // CONVERT BACK TO OWNED STRINGS
    matches.into_iter()
        .map(|(bytes, score)| {
            let name = core::str::from_utf8(bytes).unwrap_or("").to_string();
            (name, score)
        })
        .collect()
}

// CREATES `/Music/playlist.m3u` WITH TOP 10 MATCHES.
// RETURNS THE PLAYLIST OR AN ERROR
pub fn generate_playlist(query: &str) -> Result<String, SdError> {
    let top = search_top_n(query, 10);
    if top.is_empty() {
        defmt::info!("🧙‍♂️ No match found!");
        return Err(SdError::File);
    }

    // BUILD M3U CONTENT
    let mut content = String::new();
    content.push_str("#EXTM3U\n");
    for (name, score) in &top {
        content.push_str(&alloc::format!("#EXTINF:-1,{} ({}%)\n", name, score));
        content.push_str(&alloc::format!("{}\n", name));
    }

    let path = "/Music/playlist.m3u";
    write_file(path, content.as_bytes())?;
    
    let mut verify_buf = [0u8; 128];
    let read_result = read_file(path, &mut verify_buf);
    match read_result {
        Ok(n) => defmt::debug!("Verification: read {} bytes from playlist", n),
        Err(e) => defmt::error!("Verification FAILED: {:?}", e),
    }
    
    embassy_time::block_for(embassy_time::Duration::from_millis(100));
    Ok(path.to_string())
}

// ───────────────────────────────────────────────────────────────────────
// GET THE SIZE OF A FILE (BYTES) BY FULL PATH
pub fn file_size(path: &str) -> Result<u32, SdError> {
    ensure_sd_ready()?;
    defmt::debug!("file_size: path='{}'", path);

    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = match guard.as_mut() {
            Some(v) => v,
            None => {
                defmt::error!("file_size: VolumeManager not initialized");
                return Err(SdError::NotInitialized);
            }
        };
        defmt::debug!("file_size: VolumeManager obtained");

        let volume = match vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0)) {
            Ok(v) => v,
            Err(e) => {
                defmt::error!("file_size: open_raw_volume failed");
                return Err(SdError::Volume);
            }
        };
        defmt::debug!("file_size: volume opened");

        let root_dir = match vol_mgr.open_root_dir(volume) {
            Ok(d) => d,
            Err(e) => {
                defmt::error!("file_size: open_root_dir failed");
                return Err(SdError::RootDir);
            }
        };
        defmt::debug!("file_size: root dir opened");

        let (dir_name, file_name) = split_path(path);
        defmt::debug!("file_size: dir='{}', file='{}'", dir_name.as_str(), file_name.as_str());

        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            match vol_mgr.open_dir(root_dir, dir_name.as_str()) {
                Ok(d) => {
                    defmt::debug!("file_size: subdir '{}' opened", dir_name.as_str());
                    d
                },
                Err(e) => {
                    defmt::error!("file_size: open_dir '{}' failed", dir_name.as_str());
                    return Err(SdError::Directory);
                }
            }
        };

        let mut lfn_storage = [0u8; 256];
        let mut lfn_buf = embedded_sdmmc::LfnBuffer::new(&mut lfn_storage);

        let mut found_size: Option<u32> = None;

        let iter_result = vol_mgr.iterate_dir_lfn(dir, &mut lfn_buf, |entry, lfn| {
            if !entry.attributes.is_directory() {
                let full = if let Some(name) = lfn {
                    name.to_string()
                } else {
                    let base = core::str::from_utf8(&entry.name.base_name()).unwrap_or("");
                    let ext = core::str::from_utf8(&entry.name.extension()).unwrap_or("");
                    if ext.is_empty() {
                        alloc::format!("{}", base.trim())
                    } else {
                        alloc::format!("{}.{}", base.trim(), ext.trim())
                    }
                };
                defmt::trace!("file_size: checking entry '{}'", full.as_str());
                if full.eq_ignore_ascii_case(&file_name) {
                    found_size = Some(entry.size);
                    defmt::debug!("file_size: match! size={}", entry.size);
                    return core::ops::ControlFlow::Break(());
                }
            }
            core::ops::ControlFlow::Continue(())
        });

        match iter_result {
            Ok(_) => defmt::debug!("file_size: iteration done, found={:?}", found_size),           
            Err(e) => defmt::error!("file_size: iterate_dir_lfn error"),
        }

        // CLOSING TIME
        if let Err(e) = vol_mgr.close_dir(dir) {
            defmt::error!("close_dir (dir) failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = vol_mgr.close_dir(root_dir) {
            defmt::error!("close_dir (root) failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = vol_mgr.close_volume(volume) {
            defmt::error!("close_volume failed: {}", defmt::Debug2Format(&e));
        }

        match found_size {
            Some(s) => Ok(s),
            None => {
                defmt::warn!("file_size: file not found in directory");
                Err(SdError::File)
            }
        }
    })
}


pub fn read_file(path: &str, buffer: &mut [u8]) -> Result<usize, SdError> {
    ensure_sd_ready()?;
    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;

        let volume = vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0)).map_err(|_| SdError::Volume)?;
        let root_dir = vol_mgr.open_root_dir(volume).map_err(|_| SdError::RootDir)?;

        let (dir_name, file_name) = split_path(path);
        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            vol_mgr.open_dir(root_dir, dir_name.as_str()).map_err(|_| SdError::Directory)?
        };
        let file = vol_mgr.open_file_in_dir(dir, file_name.as_str(), embedded_sdmmc::Mode::ReadOnly)
            .map_err(|_| SdError::File)?;

        let bytes_read = vol_mgr.read(file, buffer).map_err(|_| SdError::Read)?;
        vol_mgr.close_file(file)
            .map_err(|_| SdError::File)?;

        
        if let Err(e) = vol_mgr.close_dir(dir) {
            defmt::error!("close_dir (dir) failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = vol_mgr.close_dir(root_dir) {
            defmt::error!("close_dir (root) failed: {}", defmt::Debug2Format(&e));
        }
        if let Err(e) = vol_mgr.close_volume(volume) {
            defmt::error!("close_volume failed: {}", defmt::Debug2Format(&e));
        }
        Ok(bytes_read)
    })
}


// ───────────────────────────────────────────────────────────────────────
// STREAM FILE

// STREAMING FILE READER – HOLDS THE VOLUME MANAGER LOCK WHILE THE FILE IS OPEN.
pub struct SdFileStream {
    volume: embedded_sdmmc::RawVolume,
    dir: embedded_sdmmc::RawDirectory,
    raw_file: embedded_sdmmc::RawFile,
}

// READS UP TO `buf.len()` BYTES FROM THE FILE INTO `buf`.
// RETURNS THE NUMBER OF BYTES READ (0 AT EOF)
impl SdFileStream {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, SdError> {
        critical_section::with(|cs| {
            let mut guard = VOL_MGR.borrow(cs).borrow_mut();
            let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;

            let n = vol_mgr
                .read(self.raw_file, buf)
                .map_err(|_| SdError::Read)?;

            Ok(n)
        })
    }
}


impl Drop for SdFileStream {
    fn drop(&mut self) {
        critical_section::with(|cs| {
            let mut guard = VOL_MGR.borrow(cs).borrow_mut();
            if let Some(vol_mgr) = guard.as_mut() {
                let _ = vol_mgr.close_file(self.raw_file);
                let _ = vol_mgr.close_dir(self.dir);
                let _ = vol_mgr.close_volume(self.volume);
            }
        });
    }
}



// WALK A DIRECTORY PATH AND RETURN THE FINAL DIRECTORY HANDLE
// THE CALLER IS RESPONSIBLE FOR CLOSING THE RETURNED DIRECTORY AND VOLUME
fn walk_to_dir(
    vol_mgr: &mut VolumeMgrType,
    dir_path: &str,
) -> Result<(embedded_sdmmc::RawDirectory, embedded_sdmmc::RawVolume), SdError> {
    let volume = vol_mgr
        .open_raw_volume(embedded_sdmmc::VolumeIdx(0))
        .map_err(|_| SdError::Volume)?;

    let root_dir = vol_mgr
        .open_root_dir(volume)
        .map_err(|_| {
            vol_mgr.close_volume(volume).ok();
            SdError::RootDir
        })?;

    if dir_path.is_empty() {
        return Ok((root_dir, volume));
    }

    let components: Vec<&str> = dir_path.split('/').collect();
    let mut current_dir = root_dir;

    for comp in &components {
        // FIND THE EXACT DIRECTORY NAME (CASE-INSENSITIVE)
        let mut found_name: Option<String> = None;
        let mut lfn_storage = [0u8; 256];
        let mut lfn_buf = embedded_sdmmc::LfnBuffer::new(&mut lfn_storage);

        let _ = vol_mgr.iterate_dir_lfn(current_dir, &mut lfn_buf, |entry, lfn| {
            if entry.attributes.is_directory() && !entry.attributes.is_volume() {
                let name = if let Some(l) = lfn {
                    l.to_string()
                } else {
                    let base = core::str::from_utf8(&entry.name.base_name()).unwrap_or("").trim().to_string();
                    let ext  = core::str::from_utf8(&entry.name.extension()).unwrap_or("").trim();
                    if ext.is_empty() { base } else { format!("{}.{}", base, ext) }
                };
                if name.eq_ignore_ascii_case(comp) {
                    found_name = Some(name.clone());
                    return core::ops::ControlFlow::Break(());
                }
            }
            core::ops::ControlFlow::Continue(())
        });

        let exact_name = found_name.ok_or_else(|| {
            vol_mgr.close_dir(current_dir).ok();
            vol_mgr.close_volume(volume).ok();
            SdError::Directory
        })?;

        let sub = vol_mgr.open_dir(current_dir, &*exact_name)
            .map_err(|_| {
                vol_mgr.close_dir(current_dir).ok();
                vol_mgr.close_volume(volume).ok();
                SdError::Directory
            })?;

        if let Err(e) = vol_mgr.close_dir(current_dir) {
            defmt::error!("walk_to_dir: close_dir failed: {}", defmt::Debug2Format(&e));
        }
        current_dir = sub;
    }

    Ok((current_dir, volume))
}



pub fn open_file_stream(path: &str) -> Result<SdFileStream, SdError> {
    ensure_sd_ready()?;
    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;

        let (dir_path, file_name) = split_path(path);
        if file_name.is_empty() {
            return Err(SdError::Directory); // NOT A FILE
        }

        let (dir, volume) = walk_to_dir(vol_mgr, &dir_path)?;

        // SEARCH FOR THE FILE BY LONG NAME (CASE-iINSENSITIVE) AND GET ITS SHORT NAME
        let mut lfn_storage = [0u8; 256];
        let mut lfn_buf = embedded_sdmmc::LfnBuffer::new(&mut lfn_storage);
        let mut short_name: Option<String> = None;

        let _ = vol_mgr.iterate_dir_lfn(dir, &mut lfn_buf, |entry, lfn| {
            if !entry.attributes.is_directory() {
                let full = if let Some(name) = lfn {
                    name.to_string()
                } else {
                    let base = core::str::from_utf8(&entry.name.base_name()).unwrap_or("");
                    let ext  = core::str::from_utf8(&entry.name.extension()).unwrap_or("");
                    if ext.is_empty() {
                        base.trim().to_string()
                    } else {
                        format!("{}.{}", base.trim(), ext.trim())
                    }
                };
                if full.eq_ignore_ascii_case(&file_name) {
                    // BUILD THE SHORT NAME FROM THE RAW ENTRY (8.3)
                    let sname = format!(
                        "{}.{}",
                        core::str::from_utf8(&entry.name.base_name()).unwrap_or("").trim(),
                        core::str::from_utf8(&entry.name.extension()).unwrap_or("").trim()
                    );
                    short_name = Some(sname);
                    return core::ops::ControlFlow::Break(());
                }
            }
            core::ops::ControlFlow::Continue(())
        });

        let short_name = match short_name {
            Some(name) => name,
            None => {
                // CLEANUP BEFORE RETURNING THE ERROR
                vol_mgr.close_dir(dir).ok();
                vol_mgr.close_volume(volume).ok();
                return Err(SdError::File);
            }
        };

        let raw_file = vol_mgr
            .open_file_in_dir(dir, &*short_name, embedded_sdmmc::Mode::ReadOnly)
            .map_err(|_| {
                vol_mgr.close_dir(dir).ok();
                vol_mgr.close_volume(volume).ok();
                SdError::File
            })?;

        Ok(SdFileStream {
            volume,
            dir,
            raw_file,
        })
    })
}



// OPENS A FILE FOR STREAMING READS
// TO AVOID HAVING TO LOAD ENTIRE FILE INTO MEMORY
pub fn open_file_streamm(path: &str) -> Result<SdFileStream, SdError> {
    ensure_sd_ready()?;
    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;

        let volume = vol_mgr
            .open_raw_volume(embedded_sdmmc::VolumeIdx(0))
            .map_err(|_| SdError::Volume)?;

        let root_dir = vol_mgr
            .open_root_dir(volume)
            .map_err(|_| {
                vol_mgr.close_volume(volume).ok();
                SdError::RootDir
            })?;

        let (dir_name, file_name) = split_path(path);

        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            match vol_mgr.open_dir(root_dir, dir_name.as_str()) {
                Ok(sub) => {
                    vol_mgr.close_dir(root_dir).ok();
                    sub
                }
                Err(_) => {
                    vol_mgr.close_dir(root_dir).ok();
                    vol_mgr.close_volume(volume).ok();
                    return Err(SdError::Directory);
                }
            }
        };

        let raw_file = vol_mgr
            .open_long_name_file_in_dir(dir, &file_name, embedded_sdmmc::Mode::ReadOnly)
            .map_err(|_| {
                vol_mgr.close_dir(dir).ok();
                vol_mgr.close_volume(volume).ok();
                SdError::File
            })?;

        Ok(SdFileStream {
            volume,
            dir,
            raw_file,
        })
    })
}



// STREAMING FILE WRITER – HANDS OUT RAW HANDLES, DROP CLEANS UP
pub struct SdFileWriter {
    raw_file: embedded_sdmmc::RawFile,
    volume: embedded_sdmmc::RawVolume,
    dir: embedded_sdmmc::RawDirectory,
}

impl SdFileWriter {
    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), SdError> {
        critical_section::with(|cs| {
            let mut guard = VOL_MGR.borrow(cs).borrow_mut();
            let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;
            vol_mgr
                .write(self.raw_file, buf)
                .map_err(|_| SdError::Write)
        })
    }

    pub fn close(self) -> Result<(), SdError> {
        let result = critical_section::with(|cs| {
            let mut guard = VOL_MGR.borrow(cs).borrow_mut();
            let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;
            vol_mgr.close_file(self.raw_file).map_err(|_| SdError::Write)?;
            vol_mgr.close_dir(self.dir).map_err(|_| SdError::Directory)?;
            vol_mgr.close_volume(self.volume).map_err(|_| SdError::Volume)
        });
        core::mem::forget(self);
        result
    }
}

impl Drop for SdFileWriter {
    fn drop(&mut self) {
        critical_section::with(|cs| {
            let mut guard = VOL_MGR.borrow(cs).borrow_mut();
            if let Some(vol_mgr) = guard.as_mut() {
                let _ = vol_mgr.close_file(self.raw_file);
                let _ = vol_mgr.close_dir(self.dir);
                let _ = vol_mgr.close_volume(self.volume);
            }
        });
    }
}

impl tinyapi::ChunkWriter for SdFileWriter {
    type Error = SdError;
    fn write_all(&mut self, buf: &[u8]) -> Result<(), SdError> {
        self.write_all(buf)
    }
}

impl tinyapi::ChunkReader for SdFileStream {
    type Error = SdError;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }
}

// ───────────────────────────────────────────────────────────────────────
// HELPERS

// SPLIT PATH
fn split_path(full: &str) -> (String, String) {
    // TRIM LEADING & TRAILING SLASHES, THEN COLLAPSE MULTIPLE SLASHES INTO ONE
    let mut stripped = full.trim_matches('/').to_string();
    while stripped.contains("//") {
        stripped = stripped.replace("//", "/");
    }
    // SPLIT ON THE LAST `/` IF PRESENT
    if let Some(pos) = stripped.rfind('/') {
        let dir = &stripped[..pos];
        let file = &stripped[pos + 1..];
        (dir.to_string(), file.to_string())
    } else {
        ("".to_string(), stripped.to_string())
    }
}


// CONVERT LFN TO SFN
fn to_short_filename(name: &str) -> String {
    let (base, ext) = match name.rfind('.') {
        Some(pos) => (&name[..pos], &name[pos+1..]),
        None => (name, ""),
    };
    let short_base = if base.len() > 8 { &base[..8] } else { base };

    let short_ext = if ext.len() > 3 { &ext[..3] } else { ext };
    if short_ext.is_empty() {
        short_base.to_string()
    } else {
        alloc::format!("{}.{}", short_base, short_ext)
    }
}


// WRITE AN ENTIRE SLICE TO A FILE AT `path`
// CREATES THE FILE IF IT EXISTS
pub fn write_file(path: &str, data: &[u8]) -> Result<(), SdError> {
    let mut file = create_file_for_writing(path)?;
    file.write_all(data)?;
    file.close()?;
    Ok(())
}

// READ THE WHOLE FILE INTO A `Vec<u8>`
pub fn read_file_to_vec(path: &str) -> Result<Vec<u8>, SdError> {
    let mut buffer = Vec::with_capacity(512);
    let mut tmp = [0u8; 512];
    let mut stream = open_file_stream(path)?;   // single volume open
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&tmp[..n]);
        if buffer.len() > 4_000_000 {
            return Err(SdError::File);
        }
    }
    Ok(buffer)
}


// OPEN A FILE FOR WRITING. THE FILE IS CREATED IF IT DOES NOT EXIST,
// DIR MUST EXIST!
pub fn create_file_for_writing(path: &str) -> Result<SdFileWriter, SdError> {
    ensure_sd_ready()?;
    defmt::debug!("create_file_for_writing: '{}'", path);

    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = match guard.as_mut() {
            Some(v) => v,
            None => {
                defmt::error!("VolumeManager not initialized");
                return Err(SdError::NotInitialized);
            }
        };

        // OPEN VOLUME
        let volume = vol_mgr
            .open_raw_volume(embedded_sdmmc::VolumeIdx(0))
            .map_err(|_| {
                defmt::error!("open_raw_volume failed");
                SdError::Volume
            })?;

        // OPEN ROOT DIR
        let root_dir = vol_mgr
            .open_root_dir(volume)
            .map_err(|_| {
                defmt::error!("open_root_dir failed");
                // CLOSE VOLUME INB4 RETURNING ERROR
                vol_mgr.close_volume(volume).ok();
                SdError::RootDir
            })?;

        let (dir_name, file_name) = split_path(path);
        defmt::debug!("dir='{}', file='{}'", dir_name.as_str(), file_name.as_str());

        // OPEN TARGET DIR 
        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            match vol_mgr.open_dir(root_dir, dir_name.as_str()) {
                Ok(sub) => {
                    vol_mgr.close_dir(root_dir).ok();
                    sub
                }
                Err(_) => {
                    defmt::error!("open_dir '{}' failed", dir_name.as_str());
                    // CLEANUP
                    vol_mgr.close_dir(root_dir).ok();
                    vol_mgr.close_volume(volume).ok();
                    return Err(SdError::Directory);
                }
            }
        };

        // CONVERT TO 8.3 SHORT NAME
        let short_name = to_short_filename(&file_name);
        defmt::debug!("short name: '{}'", short_name.as_str());

        // OPEN/CREATE/TRUNCATE FILE
        let raw_file = vol_mgr
            .open_file_in_dir(
                dir,
                short_name.as_str(),
                embedded_sdmmc::Mode::ReadWriteCreateOrTruncate,
            )
            .map_err(|_| {
                defmt::error!("open_file_in_dir '{}' failed", short_name.as_str());
                // CLEANUP
                vol_mgr.close_dir(dir).ok();
                vol_mgr.close_volume(volume).ok();
                SdError::File
            })?;

        defmt::debug!("file created successfully");
        Ok(SdFileWriter {
            raw_file,
            volume,
            dir,
        })
    })
}


// LIST ALL ENTRIES IN DIR
pub fn list_dir(path: &str) -> Result<Vec<(String, bool, u32)>, SdError> {
    ensure_sd_ready()?;
    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;

        let volume = vol_mgr
            .open_raw_volume(embedded_sdmmc::VolumeIdx(0))
            .map_err(|_| SdError::Volume)?;
        let root_dir = vol_mgr
            .open_root_dir(volume)
            .map_err(|_| SdError::RootDir)?;

        let (dir_name, _) = split_path(path);
        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            match vol_mgr.open_dir(root_dir, dir_name.as_str()) {
                Ok(sub) => {
                    vol_mgr.close_dir(root_dir).ok();
                    sub
                }
                Err(_) => {
                    // CLEANUP BEFORE RETURNING ERROR
                    vol_mgr.close_dir(root_dir).ok();
                    vol_mgr.close_volume(volume).ok();
                    return Err(SdError::Directory);
                }
            }
        };

        let mut entries = Vec::new();
        let mut lfn_storage = [0u8; 256];
        let mut lfn_buf = embedded_sdmmc::LfnBuffer::new(&mut lfn_storage);

        let _ = vol_mgr.iterate_dir_lfn(dir, &mut lfn_buf, |entry, lfn| {
            if !entry.attributes.is_volume() {
                let name = if let Some(lfn) = lfn {
                    lfn.to_string()
                } else {
                    let base = core::str::from_utf8(&entry.name.base_name()).unwrap_or("");
                    let ext = core::str::from_utf8(&entry.name.extension()).unwrap_or("");
                    if ext.is_empty() {
                        base.trim().to_string()
                    } else {
                        format!("{}.{}", base.trim(), ext.trim())
                    }
                };

                // SKIP THE DIRECTORY SHORTCUTS `.` AND `..`
                if name != "." && name != ".." {
                    entries.push((name, entry.attributes.is_directory(), entry.size));
                }
            }
            core::ops::ControlFlow::Continue(())
        });

        if let Err(e) = vol_mgr.close_dir(dir) {
            defmt::error!("close_dir (dir) failed: {}", defmt::Debug2Format(&e));
        }

        if let Err(e) = vol_mgr.close_volume(volume) {
            defmt::error!("close_volume failed: {}", defmt::Debug2Format(&e));
        }
        Ok(entries)
    })
}



// DELETE A FILE BY FULL PATH
pub fn delete_file(path: &str) -> Result<(), SdError> {
    ensure_sd_ready()?;
    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;

        let volume = vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0))
            .map_err(|_| SdError::Volume)?;
        let root_dir = vol_mgr.open_root_dir(volume)
            .map_err(|_| SdError::RootDir)?;

        let (dir_name, file_name) = split_path(path);
        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            let sub = vol_mgr.open_dir(root_dir, dir_name.as_str())
                .map_err(|_| SdError::Directory)?;
            vol_mgr.close_dir(root_dir).ok();
            sub
        };

        let short_name = to_short_filename(&file_name);
        vol_mgr.delete_entry_in_dir(dir, short_name.as_str())
            .map_err(|_| SdError::File)?;

        vol_mgr.close_dir(dir).ok();
        vol_mgr.close_volume(volume).ok();
        Ok(())
    })
}

// BLOCK THE CURRENT THREAD UNTIL THE SD CARD IS MOUNTED
pub fn ensure_sd_ready() -> Result<(), SdError> {
    if crate::load!(crate::state::SD_READY) {
        Ok(())
    } else {
        // FIRE ENABLE COMMAND
        let _ = STORAGE_CMD.try_send(StorageCommand::Enable);
        Err(SdError::NotInitialized)
    }
}

// ───────────────────────────────────────────────────────────────────────
// PLAYLIST MANIPULATION
const PLAYLIST_PATH: &str = "/Music/playlist.m3u";

// APPEND A SONG TO THE PLAYLIST (FILENAME)
pub fn append_to_playlist(song_name: &str) -> Result<(), SdError> {
    let mut entries = read_playlist_entries()?;
    entries.push((Some(alloc::format!("#EXTINF:-1,{}", song_name)), song_name.to_string()));
    write_playlist_entries(&entries)
}

// REMOVE ENTRY FROM THE PLAYLIST.
pub fn remove_from_playlist(song_name: &str) -> Result<(), SdError> {
    let entries = read_playlist_entries()?;
    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|(_, name)| name != song_name)
        .collect();
    // IF NO ENTRIES LEFT STILL WRITE HEADER
    write_playlist_entries(&filtered)
}

// CLEAR THE ENTIRE PLAYLIST.
pub fn clear_playlist() -> Result<(), SdError> {
    write_playlist_entries(&[])
}


// ───────────────────────────────────────────────────────────────────────
// FAVOURITES MANIPULATION
const FAVOURITES_PATH: &str = "/Music/favourit.m3u";

// APPEND A SONG TO THE FAVOURITES LIST
pub fn append_to_favourites(song_name: &str) -> Result<(), SdError> {
    let mut entries = read_favourites_entries()?;
    // AVOID DUPLICATION
    if entries.iter().any(|(_, name)| name == song_name) {
        return Ok(());
    }
    entries.push((
        Some(alloc::format!("#EXTINF:-1,{}", song_name)),
        song_name.to_string(),
    ));
    write_favourites_entries(&entries)
}

// REMOVE A SONG FROM THE FAVOURITES LIST
pub fn remove_from_favourites(song_name: &str) -> Result<(), SdError> {
    let entries = read_favourites_entries()?;
    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|(_, name)| name != song_name)
        .collect();
    write_favourites_entries(&filtered)
}

// CHECK IF A SONG IS IN THE FAVOURITES LIST
pub fn check_favourites(song_name: &str) -> bool {
    let is_fav = critical_section::with(|cs| {
        FAVOURITES_CACHE
            .borrow_ref(cs)
            .iter()
            .any(|name| name == song_name)
    });
    crate::store!(crate::state::MEDIA_IS_LIKED, is_fav);
    crate::dirty!();

    is_fav
}

pub fn cache_add_favourite(song_name: &str) {
    critical_section::with(|cs| {
        let mut favs = FAVOURITES_CACHE.borrow_ref_mut(cs);
        if !favs.iter().any(|name| name == song_name) {
            favs.push(song_name.to_string());
            FAVOURITES_DIRTY.store(true, core::sync::atomic::Ordering::Release);
        }
    });
}

// REMOVE A FAVOURITE FROM THE CACHE (RAM ONLY) - MARK DIRTY FOR LATER FLUSH
pub fn cache_remove_favourite(song_name: &str) {
    critical_section::with(|cs| {
        let mut favs = FAVOURITES_CACHE.borrow_ref_mut(cs);
        if favs.iter().any(|name| name == song_name) {
            favs.retain(|name| name != song_name);
            FAVOURITES_DIRTY.store(true, core::sync::atomic::Ordering::Release);
        }
    });
}

// WRITE THE CACHED FAVOURITES LIST TO THE SD CARD IF CACHE IS DIRTY
pub fn flush_favourites_cache() -> Result<(), SdError> {
    if !FAVOURITES_DIRTY.load(core::sync::atomic::Ordering::Acquire) {
        return Ok(());
    }

    let entries: Vec<(Option<String>, String)> = critical_section::with(|cs| {
        FAVOURITES_CACHE
            .borrow_ref(cs)
            .iter()
            .map(|name| (Some(alloc::format!("#EXTINF:-1,{}", name)), name.clone()))
            .collect()
    });

    write_favourites_entries(&entries)?;
    FAVOURITES_DIRTY.store(false, core::sync::atomic::Ordering::Release);
    Ok(())
}

// LOAD FAVOURITES LIST CACHE FROM STORAGE AT STARTUP
pub fn load_favourites_cache() -> Result<(), SdError> {
    let entries = read_favourites_entries()?;
    let names: Vec<String> = entries.into_iter().map(|(_, name)| name).collect();
    critical_section::with(|cs| {
        *FAVOURITES_CACHE.borrow_ref_mut(cs) = names;
    });
    FAVOURITES_DIRTY.store(false, core::sync::atomic::Ordering::Release);
    Ok(())
}


// ───────────────────────────────────────────────────────────────────────
// PRIVATE PLAYLIST HELPERS

// READ THE PLAYLIST INTO A LIST OF `(OPTIONAL #EXTINF LINE, FILE PATH)`
fn read_playlist_entries() -> Result<Vec<(Option<String>, String)>, SdError> {
    match read_file_to_vec(PLAYLIST_PATH) {
        Ok(data) => {
            let text = core::str::from_utf8(&data).map_err(|_| SdError::Read)?;
            Ok(parse_playlist_entries(text))
        }
        Err(SdError::File) => {
            // NO PLAYLIST YET – TREAT AS EMPTY.
            Ok(Vec::new())
        }
        Err(e) => Err(e),
    }
}

// WRITE THE LIST OF ENTRIES BACK TO THE PLAYLIST FILE
fn write_playlist_entries(entries: &[(Option<String>, String)]) -> Result<(), SdError> {
    let mut content = String::with_capacity(256);
    content.push_str("#EXTM3U\n");
    for (tag, path) in entries {
        if let Some(t) = tag {
            content.push_str(t);
            content.push('\n');
        }
        content.push_str(path);
        content.push('\n');
    }
    write_file(PLAYLIST_PATH, content.as_bytes())
}

// PARSE PLAYLIST TEXT INTO ENTRIES - IGNORING THE HEADER `#EXTM3U`
fn parse_playlist_entries(text: &str) -> Vec<(Option<String>, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut entries = Vec::new();
    let mut i = 0;

    // SKIP OPTIONAL HEADER
    if i < lines.len() && lines[i].trim() == "#EXTM3U" {
        i += 1;
    }

    while i < lines.len() {
        let line = lines[i].trim().to_string();
        if line.is_empty() {
            i += 1;
            continue;
        }
        if line.starts_with("#EXTINF") {
            let tag = Some(line);
            i += 1;
            if i < lines.len() {
                let path = lines[i].trim().to_string();
                entries.push((tag, path));
                i += 1;
            } else {
                // DANGLING #EXTINF – IGNORE IT!
                break;
            }
        } else {
            // PLAIN FILE PATH WITHOUT A PRECEDING TAG
            entries.push((None, line));
            i += 1;
        }
    }
    entries
}


// CREATES MEDIA PLAYER PLAYLIST FILES IF THEY DO NOT ALREADY EXIST
fn ensure_m3u_files_exist() {
    let files = [PLAYLIST_PATH, FAVOURITES_PATH];
    for &path in &files {
        match open_file_stream(path) {
            Ok(_stream) => { defmt::debug!("{} already exists", path); }
            Err(SdError::File) => {
                defmt::info!("creating {} because it was missing", path);
                match write_file(path, b"#EXTM3U\n") {
                    Ok(()) => defmt::debug!("created {}", path),
                    Err(e) => defmt::error!("failed to create {}: {:?}", path, e),
                }
            }
            Err(e) => { defmt::error!("ERROR checking {}: {:?}", path, e); }
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// PRIVATE FAVOURITES HELPERS
fn read_favourites_entries() -> Result<Vec<(Option<String>, String)>, SdError> {
    match read_file_to_vec(FAVOURITES_PATH) {
        Ok(data) => {
            let text = core::str::from_utf8(&data).map_err(|_| SdError::Read)?;
            Ok(parse_playlist_entries(text))
        }
        Err(SdError::File) => {
            // NO FAVOURITES FILE YET – TREAT AS EMPTY
            Ok(Vec::new())
        }
        Err(e) => Err(e),
    }
}

fn write_favourites_entries(entries: &[(Option<String>, String)]) -> Result<(), SdError> {
    let mut content = String::with_capacity(256);
    content.push_str("#EXTM3U\n");
    for (tag, path) in entries {
        if let Some(t) = tag {
            content.push_str(t);
            content.push('\n');
        }
        content.push_str(path);
        content.push('\n');
    }
    write_file(FAVOURITES_PATH, content.as_bytes())
}


// ────────────────────────────────────────────────────────────
// AUTOMATIC SD CARD SELF‑TEST
// RUN THE COMMAND BELOW TO RUN THE AUTOMATIC TESTING OF YOUR SD-CARD:
//   cargo run --release --features sd-test
// ────────────────────────────────────────────────────────────
#[cfg(feature = "sd-test")]
struct TestResult {
    name: &'static str,
    ok: bool,
    detail: Option<&'static str>,
}

#[cfg(feature = "sd-test")]
fn print_result(results: &[TestResult]) {
    let passed = results.iter().filter(|r| r.ok).count();
    let total = results.len();
    defmt::info!("TEST_RESULT: {}/{} passed", passed, total);
    for r in results {
        if r.ok {
            defmt::info!("   PASS: {}", r.name);
        } else {
            defmt::error!("   FAIL: {} ({})", r.name, r.detail.unwrap_or("no detail"));
        }
    }
    if passed == total {
        defmt::info!("ALL TESTS PASSED");
    } else {
        defmt::error!("SOME TESTS FAILED");
    }
}

// INDIVIDUAL TEST HELPERS

#[cfg(feature = "sd-test")]
async fn wait_for_sd(timeout_secs: u8) -> bool {
    for _ in 0..(timeout_secs * 2) {
        if crate::load!(crate::state::SD_READY) {
            return true;
        }
        embassy_time::Timer::after_millis(500).await;
    }
    false
}

#[cfg(feature = "sd-test")]
fn test_write_read_small_file() -> TestResult {
    let path = "/test_write.txt";
    let data = b"Hello, SD card!";

    if write_file(path, data).is_err() {
        return TestResult { name: "write_read", ok: false, detail: Some("write failed") };
    }

    let mut buf = [0u8; 64];
    match read_file(path, &mut buf) {
        Ok(n) if &buf[..n] == data => {
            let _ = delete_file(path);
            TestResult { name: "write_read", ok: true, detail: None }
        }
        Ok(_) => {
            let _ = delete_file(path);
            TestResult { name: "write_read", ok: false, detail: Some("data mismatch") }
        }
        Err(_) => {
            TestResult { name: "write_read", ok: false, detail: Some("read failed") }
        }
    }
}

#[cfg(feature = "sd-test")]
fn test_fuzzy_search() -> TestResult {
    match search_song("test") {
        Some((_, score)) if score > 0 => {
            TestResult { name: "fuzzy_search", ok: true, detail: None }
        }
        Some(_) => TestResult { name: "fuzzy_search", ok: false, detail: Some("zero score") },
        None => TestResult { name: "fuzzy_search", ok: false, detail: Some("no match") },
    }
}

#[cfg(feature = "sd-test")]
fn test_playlist_generation() -> TestResult {
    match generate_playlist("demo") {
        Ok(path) => {
            let mut buf = [0u8; 20];
            if let Ok(n) = read_file(&path, &mut buf) {
                if n > 10 {
                    let _ = delete_file(&path);
                    return TestResult { name: "playlist_gen", ok: true, detail: None };
                }
            }
            TestResult { name: "playlist_gen", ok: false, detail: Some("verification read failed") }
        }
        Err(_) => TestResult { name: "playlist_gen", ok: false, detail: Some("generate_playlist error") },
    }
}

#[cfg(feature = "sd-test")]
fn test_file_streaming() -> TestResult {
    let data = [0xAAu8; 4096];
    if write_file("/stream_test.bin", &data).is_err() {
        return TestResult { name: "streaming", ok: false, detail: Some("write failed") };
    }

    match open_file_stream("/stream_test.bin") {
        Ok(mut stream) => {
            let mut total = 0;
            let mut buf = [0u8; 512];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => {
                        let _ = delete_file("/stream_test.bin");
                        return TestResult { name: "streaming", ok: false, detail: Some("read error") };
                    }
                }
            }
            let _ = delete_file("/stream_test.bin");
            if total == data.len() {
                TestResult { name: "streaming", ok: true, detail: None }
            } else {
                TestResult { name: "streaming", ok: false, detail: Some("size mismatch") }
            }
        }
        Err(_) => {
            let _ = delete_file("/stream_test.bin");
            TestResult { name: "streaming", ok: false, detail: Some("open failed") }
        }
    }
}

#[cfg(feature = "sd-test")]
fn test_delete_file() -> TestResult {
    let path = "/delete_me.txt";
    let _ = write_file(path, b"temp");
    if delete_file(path).is_err() {
        return TestResult { name: "delete", ok: false, detail: Some("delete returned error") };
    }
    let mut buf = [0u8; 1];
    match read_file(path, &mut buf) {
        Err(_) => TestResult { name: "delete", ok: true, detail: None },
        Ok(_) => TestResult { name: "delete", ok: false, detail: Some("file still exists") },
    }
}

#[cfg(feature = "sd-test")]
fn test_nonexistent_file() -> TestResult {
    let mut buf = [0u8; 1];
    match read_file("/definitely_not_there.xyz", &mut buf) {
        Err(_) => TestResult { name: "nonexistent", ok: true, detail: None },
        Ok(_) => TestResult { name: "nonexistent", ok: false, detail: Some("unexpected success") },
    }
}

// THE PUBLIC TEST TASK THAT YOU SPAWN FROM MAIN
#[cfg(feature = "sd-test")]
#[embassy_executor::task]
pub async fn test_task() {
    defmt::info!("SD Card Self‑Test Starting..");
    ensure_sd_ready().ok();
    
    embassy_time::Timer::after_secs(5).await;    
    
    // WAIT UP TO 15 SECONDS FOR STORAGE TO BE READY
    if !wait_for_sd(10).await {
        defmt::error!("STORAGE DID NOT BECOME READY");
        let r = [TestResult { name: "storage_init", ok: false, detail: Some("timeout") }];
        print_result(&r);
        loop { embassy_time::Timer::after_secs(1).await; }
    }

    // GIVE FILESYSTEM A BREATH
    embassy_time::Timer::after_millis(500).await;

    let results = [
        test_write_read_small_file(),
        test_fuzzy_search(),
        test_playlist_generation(),
        test_file_streaming(),
        test_delete_file(),
        test_nonexistent_file(),
    ];

    print_result(&results);

    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}

