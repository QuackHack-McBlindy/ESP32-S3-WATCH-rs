# **embedded-png**
 
[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)

## **embedded-png**


`embedded-png` is a **`no_std`, heap‑allocating** library that decodes PNG images and draws them on any display that implements [`embedded-graphics`](https://crates.io/crates/embedded-graphics)’ `DrawTarget<Color = Rgb565>`.

Supports common PNG pixel formats:  

- **Indexed colour**
- **RGB / RGBA**
- **Grayscale**
- **Grayscale + alpha**



## **Installation**

  
Add this to your `Cargo.toml`:  

```toml
[dependencies]
embedded-png = "0.1.0"
embedded-graphics = "0.8.2"
minipng = "1"
```  

If you want `defmt` logging (e.g. on ESP32 with `esp‑hal`), enable the feature:  


```
embedded-png = { version = "0.1.0", features = ["defmt"] }
```

<br>

## **Example usage**

**1. Draw a PNG from an embedded byte slice**  

```rust
use embedded_png::draw_png_bytes;

let my_png = include_bytes!("./../assets/splash.png");
draw_png_bytes(&mut display, my_png, 10, 20)?;
```

**2. Load, inspect, then draw (for centering)**  

```rust
use embedded_png::{Png, draw_png};

let png_data = include_bytes!("./../assets/icon.png");
if let Ok(png) = Png::load_from_bytes(png_data) {
    let fb_size = display.bounding_box().size;
    let x = (fb_size.width as i32 - png.width() as i32) / 2;
    let y = (fb_size.height as i32 - png.height() as i32) / 2;
    draw_png(&mut display, &png, x, y)?;
}
```

**3. Draw at the top‑left corner**  

```rust
use embedded_png::draw_png_bytes_at_origin;

let png_data = include_bytes!("./../assets/logo.png");
draw_png_bytes_at_origin(&mut display, png_data)?;
```


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
