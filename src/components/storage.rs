//! COMPONENTS/STORAGE
//! MICRO SECURE DIGITAL CARD DRIVER (SPI)
//! FILESYSTEM: FAT32
//! PROVIDES EVERYTHING NEEDED FOR:
//! SD CARD INIT ++ FUZZY SEARCHING FILES ON THE SD CARD FOR PLAYBACK 
//! ALL OPERATIONS ARE PERFORMED INSIDE A CRITICAL SECTION TO AVOID LIFETIME COMPLICATIONS.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

// TYPES & GLOBAL STATE
pub enum SdState {
    NotInserted,
    Mounted,
    Error,
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
type FileType<'a> = embedded_sdmmc::File<'a, SdCardType, DummyTime, 8, 4, 1>;

static VOL_MGR: critical_section::Mutex<core::cell::RefCell<Option<VolumeMgrType>>> = critical_section::Mutex::new(core::cell::RefCell::new(None));


// INITIATE SD CARD DRIVER (CALL FROM MAIN)
pub fn init(card: SdCardType) {
    let vol_mgr = embedded_sdmmc::VolumeManager::new(card, DummyTime);
    critical_section::with(|cs| {
        *VOL_MGR.borrow(cs).borrow_mut() = Some(vol_mgr);
    });
}

// FUZZY SEARCH SONGS 
pub fn search_song(query: &str) -> Option<(String, u8)> {
    defmt::info!("fuzzy searching song: {}", query);

    critical_section::with(|cs| {
        let mut guard = VOL_MGR.borrow(cs).borrow_mut();
        let vol_mgr = guard.as_mut()?;
        defmt::debug!("VolumeManager obtained");

        let volume = vol_mgr.open_raw_volume(embedded_sdmmc::VolumeIdx(0)).ok()?;
        defmt::info!("Volume opened");

        let root_dir = vol_mgr.open_root_dir(volume).ok()?;
        defmt::info!("Root directory opened");

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
                defmt::info!("found file: {}", full.as_str());
                names.push(full.as_bytes().to_vec());
            }
            core::ops::ControlFlow::Continue(())
        });
        defmt::info!("Total files found: {}", names.len());

        let _ = vol_mgr.close_dir(music_dir);
        let _ = vol_mgr.close_dir(root_dir);

        if names.is_empty() {
            defmt::warn!("⚠️ No files in /Music");
            return None;
        }

        let candidates: Vec<&[u8]> = names.iter().map(|v| v.as_slice()).collect();
        let (best_bytes, score) = barely_fuzzy::best_fuz(query.as_bytes(), &candidates, 5);
        defmt::debug!("best score: {}, best bytes: {:?}", score, best_bytes);


        let best_name = core::str::from_utf8(best_bytes).ok()?.to_string();
        defmt::info!("🏆 Best match: {} ({}%)", best_name.as_str(), score);
        Some((best_name, score))
 
    })
}

pub fn read_file(path: &str, buffer: &mut [u8]) -> Result<usize, SdError> {
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
        Ok(bytes_read)
    })
}



// STREAMING FILE READER – HOLDS THE VOLUME MANAGER LOCK WHILE THE FILE IS OPEN.
pub struct SdFileStream {
    raw_file: embedded_sdmmc::RawFile,
}

// READS UP TO `buf.len()` BYTES FROM THE FILE INTO `buf`.
// RETURNS THE NUMBER OF BYTES READ (0 AT EOF)
impl SdFileStream {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, SdError> {
        critical_section::with(|cs| {
            let mut guard = VOL_MGR.borrow(cs).borrow_mut();
            let vol_mgr = guard.as_mut().ok_or(SdError::NotInitialized)?;
            vol_mgr.read(self.raw_file, buf).map_err(|_| SdError::Read)
        })
    }
}

// OPENS A FILE FOR STREAMING READS
// TO AVOID HAVING TO LOAD ENTIRE FILE INTO MEMORY
pub fn open_file_stream(path: &str) -> Result<SdFileStream, SdError> {
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
            root_dir
        } else {
            vol_mgr
                .open_dir(root_dir, dir_name.as_str())
                .map_err(|_| SdError::Directory)?
        };

        let raw_file = vol_mgr
            .open_long_name_file_in_dir(dir, &file_name, embedded_sdmmc::Mode::ReadOnly)
            .map_err(|_| SdError::File)?;

        // DIRECTORIES NO LONGER NEEDED
        let _ = vol_mgr.close_dir(dir);
        let _ = vol_mgr.close_dir(root_dir);

        Ok(SdFileStream { raw_file })
    })
}

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

#[derive(Debug, defmt::Format)] 
pub enum SdError {
    NotInitialized,
    Volume,
    RootDir,
    Directory,
    File,
    Read,
}
