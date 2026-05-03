// COMPONENTS/QSPI_BUS
// QSPI BUS DRIVER FOR CO5300 AMOLED DISPLAY - DMA VERSION
// USES `SpiDmaBus` FOR LARGE TRANSFERS VIA DMA

// MAX BYTES PER DMA TRANSFER (MUST FIT IN DMA TX BUFFER)
const DMA_CHUNK: usize = 8000;

pub struct QspiBus<'d> {
    spi: esp_hal::spi::master::SpiDmaBus<'d, esp_hal::Blocking>,
    cs: esp_hal::gpio::Output<'d>,
    scratch: alloc::vec::Vec<u8>, // HEAP-ALLOCATED SCRATCH BUFFER FOR PIXEL CONVERSION
}

impl<'d> QspiBus<'d> {
    pub fn new(
        spi: esp_hal::spi::master::SpiDmaBus<'d, esp_hal::Blocking>,
        cs: esp_hal::gpio::Output<'d>,
    ) -> Self {
        Self {
            spi,
            cs,
            scratch: alloc::vec![0u8; DMA_CHUNK],
        }
    }

    fn cs_low(&mut self) {
        self.cs.set_low();
    }
    fn cs_high(&mut self) {
        self.cs.set_high();
    }

    pub fn write_command(&mut self, reg: u8) {
        self.cs_low();
        let _ = self.spi.half_duplex_write(
            esp_hal::spi::master::DataMode::Single,
            esp_hal::spi::master::Command::_8Bit(0x02, esp_hal::spi::master::DataMode::Single),
            esp_hal::spi::master::Address::_24Bit(
                (reg as u32) << 8,
                esp_hal::spi::master::DataMode::Single,
            ),
            0,
            &[],
        );
        self.cs_high();
    }

    pub fn write_c8d8(&mut self, reg: u8, data: u8) {
        self.cs_low();
        let _ = self.spi.half_duplex_write(
            esp_hal::spi::master::DataMode::Single,
            esp_hal::spi::master::Command::_8Bit(0x02, esp_hal::spi::master::DataMode::Single),
            esp_hal::spi::master::Address::_24Bit(
                (reg as u32) << 8,
                esp_hal::spi::master::DataMode::Single,
            ),
            0,
            &[data],
        );
        self.cs_high();
    }

    pub fn write_c8d16d16(&mut self, reg: u8, d1: u16, d2: u16) {
        let data = [(d1 >> 8) as u8, d1 as u8, (d2 >> 8) as u8, d2 as u8];
        self.cs_low();
        let _ = self.spi.half_duplex_write(
            esp_hal::spi::master::DataMode::Single,
            esp_hal::spi::master::Command::_8Bit(0x02, esp_hal::spi::master::DataMode::Single),
            esp_hal::spi::master::Address::_24Bit(
                (reg as u32) << 8,
                esp_hal::spi::master::DataMode::Single,
            ),
            0,
            &data,
        );
        self.cs_high();
    }

    pub fn begin_pixels(&mut self) {
        self.cs_low();
        let _ = self.spi.half_duplex_write(
            esp_hal::spi::master::DataMode::Quad,
            esp_hal::spi::master::Command::_8Bit(0x32, esp_hal::spi::master::DataMode::Single),
            esp_hal::spi::master::Address::_24Bit(
                0x003C00,
                esp_hal::spi::master::DataMode::Single,
            ),
            0,
            &[],
        );
    }

    pub fn stream_pixels(&mut self, pixels: &[u16]) {
        if pixels.is_empty() {
            return;
        }
        let max_px = DMA_CHUNK / 2;
        let mut remaining = pixels;
        while !remaining.is_empty() {
            let n = remaining.len().min(max_px);
            for (i, &px) in remaining[..n].iter().enumerate() {
                self.scratch[i * 2] = (px >> 8) as u8;
                self.scratch[i * 2 + 1] = px as u8;
            }
            let _ = self.spi.half_duplex_write(
                esp_hal::spi::master::DataMode::Quad,
                esp_hal::spi::master::Command::None,
                esp_hal::spi::master::Address::None,
                0,
                &self.scratch[..n * 2],
            );
            remaining = &remaining[n..];
        }
    }

    pub fn end_pixels(&mut self) {
        self.cs_high();
    }

    pub fn write_pixels(&mut self, pixels: &[u16]) {
        if pixels.is_empty() {
            return;
        }
        self.cs_low();
        let max_px = DMA_CHUNK / 2;
        let mut remaining = pixels;
        let mut first = true;
        while !remaining.is_empty() {
            let n = remaining.len().min(max_px);
            for (i, &px) in remaining[..n].iter().enumerate() {
                self.scratch[i * 2] = (px >> 8) as u8;
                self.scratch[i * 2 + 1] = px as u8;
            }
            if first {
                let _ = self.spi.half_duplex_write(
                    esp_hal::spi::master::DataMode::Quad,
                    esp_hal::spi::master::Command::_8Bit(
                        0x32,
                        esp_hal::spi::master::DataMode::Single,
                    ),
                    esp_hal::spi::master::Address::_24Bit(
                        0x003C00,
                        esp_hal::spi::master::DataMode::Single,
                    ),
                    0,
                    &self.scratch[..n * 2],
                );
                first = false;
            } else {
                let _ = self.spi.half_duplex_write(
                    esp_hal::spi::master::DataMode::Quad,
                    esp_hal::spi::master::Command::None,
                    esp_hal::spi::master::Address::None,
                    0,
                    &self.scratch[..n * 2],
                );
            }
            remaining = &remaining[n..];
        }
        self.cs_high();
    }

    pub fn write_repeat(&mut self, color: u16, count: u32) {
        if count == 0 {
            return;
        }
        let hi = (color >> 8) as u8;
        let lo = color as u8;
        let max_px = DMA_CHUNK / 2;
        // FILL SCRATCH WITH REPEATED COLOR
        for i in 0..max_px {
            self.scratch[i * 2] = hi;
            self.scratch[i * 2 + 1] = lo;
        }
        self.cs_low();
        let mut remaining = count;
        let mut first = true;
        while remaining > 0 {
            let n = remaining.min(max_px as u32);
            let bytes = (n as usize) * 2;
            if first {
                let _ = self.spi.half_duplex_write(
                    esp_hal::spi::master::DataMode::Quad,
                    esp_hal::spi::master::Command::_8Bit(
                        0x32,
                        esp_hal::spi::master::DataMode::Single,
                    ),
                    esp_hal::spi::master::Address::_24Bit(
                        0x003C00,
                        esp_hal::spi::master::DataMode::Single,
                    ),
                    0,
                    &self.scratch[..bytes],
                );
                first = false;
            } else {
                let _ = self.spi.half_duplex_write(
                    esp_hal::spi::master::DataMode::Quad,
                    esp_hal::spi::master::Command::None,
                    esp_hal::spi::master::Address::None,
                    0,
                    &self.scratch[..bytes],
                );
            }
            remaining -= n;
        }
        self.cs_high();
    }
}
