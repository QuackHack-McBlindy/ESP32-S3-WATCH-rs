# **ES8311**
 
[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)

## **ES8311**

`es8311` is a `no_std`, platform‑independent driver for the **Everest Semi ES8311** low‑power audio ADC/DAC codec.  
It communicates over I²C using the [`embedded-hal`](https://crates.io/crates/embedded-hal) abstractions and requires **no additional dependencies**.  

> **Supported features**  
> * 24‑bit audio ADC and DAC with configurable resolution (16‑32 bit)  
> * Sample rates from 8 kHz to 96 kHz  
> * Programmable microphone gain (0 dB … +42.5 dB) with selectable fade  
> * Digital volume control (0‑100%) with mute  
> * Analogue or digital microphone input  
> * Clock source from dedicated MCLK or derived from SCLK  
> * Smooth volume fades to avoid clicks and pops  


## **Installation**

  
Add `es8311` as a dependency in `Cargo.toml`.

```toml
[dependencies]
es8311 = "0.1.0"
```
  


<br>

## **Example usage**

```rust
use es8311::{Es8311, ClockConfig, Resolution, MicGain, Fade};
// Use any I²C and delay implementations that implement embedded_hal traits.
// Replace `MyHalI2c` and `MyDelay` with your HAL's types.
fn example<I2C, D>(i2c: &mut I2C, delay: &mut D) -> Result<(), es8311::Error<I2C::Error>>
where
    I2C: embedded_hal::i2c::I2c,
    D: embedded_hal::delay::DelayNs,
{
    // ES8311 typically lives at address 0x18 (7‑bit)
    let codec = Es8311::new(0x18);

    // Clock configuration
    let clk_cfg = ClockConfig {
        mclk_inverted: false,
        sclk_inverted: false,
        mclk_from_mclk_pin: true,
        mclk_frequency: 12_288_000,   // 12.288 MHz
        sample_frequency: 48_000,
    };

    // Initialise the codec
    codec.init(i2c, &clk_cfg, Resolution::Bits16, Resolution::Bits16, delay)?;

    // Configure the microphone (analogue in this case)
    codec.microphone_config(i2c, false)?;  // false = analogue mic

    // Set microphone gain to +24 dB
    codec.microphone_gain_set(i2c, MicGain::Gain24dB)?;

    // Set output volume to 80%
    codec.volume_set(i2c, 80, None)?;

    // Unmute the DAC
    codec.mute(i2c, false)?;

    // (Optional) Set fade rate for smooth volume changes
    codec.fade(i2c, Fade::LRCK1024)?;

    Ok(())
}
```

> [!NOTE]
> **Note: The driver is completely generic over the I²C bus and delay provider.**  
> **It works with any HAL that supplies an embedded-hal compatible I²C interface and a delay (DelayNs), for example ESP‑32, STM32, nRF, RP2040, etc**  

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
