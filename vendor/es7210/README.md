# **ES7210**
 
[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)

## **ES7210**


`es7210` is a `no_std`, platform‑independent driver for the **Everest Semi ES7210** quad‑channel audio ADC.  
It communicates over I²C using the [`embedded-hal`](https://crates.io/crates/embedded-hal) abstractions and requires **no additional dependencies**.  

> **Supported features**  
> * I²S, Left‑Justified, DSP‑A / DSP‑B data formats  
> * Sample rates from 8 kHz to 96 kHz  
> * TDM (time‑division multiplexing) mode  
> * Programmable microphone bias voltage (2.18 V … 2.87 V)  
> * 15‑step microphone preamplifier gain (0 dB … +37.5 dB)  
> * Direct digital volume control (–95 dB to +32 dB)  
> * Mute / unmute, power‑up / power‑down sequencing  



## **Installation**

  
Add `es7210` as a dependency in `Cargo.toml`.

```toml
[dependencies]
es7210 = "0.1.0"
```
  


<br>

## **Example usage**

```rust
use es7210::{Es7210, CodecConfig, I2sFormat, I2sBits, MicBias, MicGain};

// You can use any I²C implementation that implements `embedded_hal::i2c::I2c`.
// Replace `MyHalI2c` with the I²C peripheral type from your HAL.
fn example<I2C: embedded_hal::i2c::I2c>(i2c: &mut I2C) -> Result<(), es7210::Error<I2C::Error>> {
    // The ES7210 is typically at address 0x40
    let codec = Es7210::new(0x40);

    // Define the desired configuration
    let cfg = CodecConfig {
        sample_rate_hz: 48_000,
        mclk_ratio: 256,               // MCLK = 12.288 MHz
        i2s_format: I2sFormat::I2S,
        bit_width: I2sBits::Bits24,
        mic_bias: MicBias::V2_55,
        mic_gain: MicGain::Gain18dB,
        tdm_enable: false,
    };

    // Run the full initialisation sequence
    codec.config_codec(i2c, &cfg)?;

    // The ADC is now streaming audio.
    // You may later call `codec.disable(i2c)?` and `codec.enable(i2c)?` to
    // stop / resume conversion without a full re‑initialisation.

    Ok(())
}
```

> [!NOTE]
> **Note: The driver is completely generic over the I²C bus and delay provider.**  
> **It works with any HAL that supplies an embedded-hal compatible I²C interface (ESP‑32, STM32, nRF, RP2040, …).**  

<br>



## **Features**

- **True `no_std` – runs on bare‑metal, no operating system required.**  

- **Minimal dependencies – only embedded-hal (version 1.0)**  

- **No logging framework – the published code contains no defmt or log calls, keeping the dependency tree tiny**  

- **Exhaustive clock coefficient table – supports common MCLK / sample rate combinations for 8 k, 11.025 k, 12 k, 16 k, 22.05 k, 24 k, 32 k, 44.1 k, 48 k, 64 k, 88.2 k, and 96 kHz**  

- **Simple, ergonomic API – a single configuration struct and a handful of methods cover the most common use cases**  


<br><br>

## **☕**

[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)
> 🦆🧑‍🦯 says ⮞ Hi! I'm QuackHack-McBlindy!  
> Like my work?  
> Buy me a coffee, or become a sponsor.  
> Thanks for supporting open source/hungry developers ♥️🦆!   

♥️₿ *Wallet:* `pungkula.x`  
<a href="https://www.buymeacoffee.com/quackhackmcblindy" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" style="height: 60px !important;width: 217px !important;" ></a>

<br>

## **License**

This project is licensed under the terms of the MIT license.  
See the `LICENSE` file in the repository for full details.  
