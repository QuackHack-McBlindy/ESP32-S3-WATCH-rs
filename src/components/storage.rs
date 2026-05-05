// MICRO SECURE DIGITAL CARD DRIVER (SPI)
// FILESYSTEM: FAT

//use embedded_hal::spi::SpiDevice;

/// CARD STATE
pub enum SdState {
    NotInserted,
    Mounted,
    Error,
}
