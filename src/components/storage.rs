// COMPONENTS/STORAGE
// MICRO SECURE DIGITAL CARD DRIVER (SPI)
// FILESYSTEM: FAT32
// PROVIDES EVERYTHING NEEDED FOR:
// SD CARD INIT ++ FUZZY SEARCHING FILES ON THE SD CARD FOR EASY VOICE COMMAND PLAYBACK  (be drunk & speak japanese, it's all good)

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

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
                defmt::info!("Storage: enabling and initialising SD card...");
                let vol_mgr = embedded_sdmmc::VolumeManager::new(card, DummyTime);
                critical_section::with(|cs| {
                    *VOL_MGR.borrow(cs).borrow_mut() = Some(vol_mgr);
                });
                enabled = true;
                crate::store!(crate::state::SD_READY, true);
                defmt::info!("Storage: ready");
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

        let _ = vol_mgr.close_dir(music_dir);
        let _ = vol_mgr.close_dir(root_dir);
        let _ = vol_mgr.close_volume(volume);

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

        let _ = vol_mgr.close_dir(music_dir);
        let _ = vol_mgr.close_dir(root_dir);
        let _ = vol_mgr.close_volume(volume);
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
            Ok(_) => defmt::debug!("file_size: dir iteration completed with no break"),
            Err(e) => defmt::error!("file_size: iterate_dir_lfn error"),
        }

        // CLOSING TIME
        let _ = vol_mgr.close_dir(dir);
        let _ = vol_mgr.close_dir(root_dir);
        let _ = vol_mgr.close_volume(volume);

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
        let _ = vol_mgr.close_dir(dir);
        let _ = vol_mgr.close_dir(root_dir);
        let _ = vol_mgr.close_volume(volume);
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


// OPENS A FILE FOR STREAMING READS
// TO AVOID HAVING TO LOAD ENTIRE FILE INTO MEMORY
pub fn open_file_stream(path: &str) -> Result<SdFileStream, SdError> {
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

        let (dir_name, file_name) = split_path(path);

        let dir = if dir_name.is_empty() {
            root_dir // CLOSED IN DROP
        } else {
            // OPEN SUBDIR, THEN CLOSE THE ROOT HANDLE
            let sub_dir = vol_mgr
                .open_dir(root_dir, dir_name.as_str())
                .map_err(|_| SdError::Directory)?;
            vol_mgr.close_dir(root_dir).ok(); // root_dir is no longer needed
            sub_dir
        };

        let raw_file = vol_mgr
            .open_long_name_file_in_dir(dir, &file_name, embedded_sdmmc::Mode::ReadOnly)
            .map_err(|_| SdError::File)?;

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
    let stripped = full.trim_start_matches('/');
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
    let size = file_size(path)?;
    // LIMIT BY SIZE
    if size > 4_000_000 {
        return Err(SdError::File);
    }
    let mut buffer = Vec::with_capacity(size as usize);
    // SAFETY FIRST: JUST ALLOCATE THE VEC
    let mut tmp = [0u8; 512];
    let mut stream = open_file_stream(path)?;
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&tmp[..n]);
    }
    Ok(buffer)
}


// OPEN A FILE FOR WRITING. THE FILE IS CREATED IF IT DOES NOT EXIST,
// DIR MUST EXIST!
pub fn create_file_for_writing(path: &str) -> Result<SdFileWriter, SdError> {
    ensure_sd_ready()?;
    defmt::info!("create_file_for_writing: '{}'", path);

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

        let volume = vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0))
            .map_err(|_| SdError::Volume)?;
        let root_dir = vol_mgr.open_root_dir(volume)
            .map_err(|_| SdError::RootDir)?;

        let (dir_name, _) = split_path(path);
        let dir = if dir_name.is_empty() {
            root_dir
        } else {
            let sub = vol_mgr.open_dir(root_dir, dir_name.as_str())
                .map_err(|_| SdError::Directory)?;
            vol_mgr.close_dir(root_dir).ok();
            sub
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
                    if ext.is_empty() { base.trim().to_string() }
                    else { format!("{}.{}", base.trim(), ext.trim()) }
                };
                entries.push((name, entry.attributes.is_directory(), entry.size));
            }
            core::ops::ControlFlow::Continue(())
        });

        let _ = vol_mgr.close_dir(dir);
        let _ = vol_mgr.close_volume(volume);
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
fn ensure_sd_ready() -> Result<(), SdError> {
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
