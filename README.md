# **ESP32-S3-WATCH-rs**

[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)


## **ESP32-S3-WATCH**

> [!NOTE]
> **Personal project!**
> **🧑‍🦯 As I am blind this firmware is mostly focused on accessibility of the voice assistant functionality of the watch.**  
> **It will have touch and a graphical user interface -- but it will be BIG.**  
<br>

**Bare Metal** *(no_std)* **ESP32-S3** firmware written in Rust without the `esp-idf` API.   
Designed to be used as a personal voice assistant watch with media player and a notification system.     

Its up to [yo](https://github.com/QuackHack-McBlindy/yo) to write your own voice commands.  
as a reference: I have **57** [yo](https://github.com/QuackHack-McBlindy/dotfiles) scripts meaning my watch has **272684913** phrases available as voice commands.  

    
> [!CAUTION]
> __Project is under active development!__ <br>
> **Breaking changes will be frequent.**  
<br>


## **Table Of Contents**

- [Overview](#overview)
- [Roadmap](#roadmap)
- [Demo](#demo)
- [Installation](#installation)
- [Usage](#usage)
  - [Frontend](#frontend)
  - [API](#api)
  - [Media Player](#media-player)
- [Voice Assistant](#voice-assistant)
- [Hardware](#hardware)
  - [Peripherals](#peripherals)
    - [I2S](#i2s)
    - [I2C](#i2c)
- [Graphical User Interface](#graphical-user-interface)
- [Applications](#applications)
- [Power Management/Optimization](#power-management--optimizations)
- [Sponsor](#sponsor)
- [License](#license)

<br>

## **Overview**

`ESP32-S3-WATCH-rs` is a `no_std` Rust firmware for the ESP32-S3 based smartwatch. The primary goal is to create a fully voice‑controlled assistant that is highly accessible for blind and visually impaired users. All interactions can be performed via voice, and the graphical interface is designed with large, high‑contrast elements.  

The watch streams audio to a companion backend service called [`yo`](https://github.com/QuackHack-McBlindy/yo), which handles wake word detection, speech‑to‑text, intent recognition and execution, and text‑to‑speech synthesis. The watch itself streams microphone audio, serves TCP server for audio streaming to the speaker, manages notifications, plays media, and serves a web frontend for configuration and status which also can be used as a GET API.  

## **Roadmap**

- [x] Voice Command Execution (Wake word, STT, intent recognition & execution)  
- [x] On-Device API  
- [x] On-Device WebServer (UI frontend)  
- [x] Backend: `yo`  
- [ ] Power optimised for battery operation  
- [ ] Touch GUI (in progress)  
- [x] Media player (streaming)  
- [ ] Notification system  



## **Demo**

A demo video/pictures will be added soon.    

[🎥 Demo video coming soon]


## **Installation**

<details><summary><strong>
❄️ Using flakes (TODO)
</strong></summary>

*not yet...*

</details>


<details><summary><strong>
📦 Building from source
</strong></summary>


Configure WiFi and other required seetings in the example `.env` file.  

```bash
$ mv .env.example .env
$ nano .env
```


## **Build and flash!**

```bash
cargo run --release
```


</details>


<details><summary><strong>
🐋 Docker (recommended)
</strong></summary>

```bash
$ git clone https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs
$ cd ESP32-S3-WATCH-rs
```

Configure WiFi and other required seetings in the example `.env` file.  

```bash
$ mv .env.example .env
$ nano .env
```

`docker-compose.yaml` may require you to change the defined serial port.  
To locate the serial port for use with the `docker-compose.yaml` file you can run the following command:  

```bash
$ ls -l /dev/serial/by-id/
```

**Build and Flash!**

```bash
$ docker compose build
$ docker compose up
```


</details>


<br><br>

## **Usage**


### **Frontend**  


### **API**    



### **Media Player** 


> [!NOTE]
> __🎵 Media Player__ <br>
> **Media player supports any file format, and can play any file or playlist.**  
> **You can use the provided `scripts/play-esp.sh` helper script to stream audioo to the device speaker.**
<br>

 
## **Voice Assistant**


## **Hardware**

<details><summary><strong>
Specs and GPIO
</strong></summary>

**Display: CO5300**  

Screen width: 33.09 mm  
Screen height: 40.51 mm  

QSPI_SIO0 → GPIO4  
QSPI_SI1 → GPIO5  
QSPI_SI2 → GPIO6  
QSPI_SI3 → GPIO7  
QSPI_SCL → GPIO11  

LCD_CS → GPIO12  
LCD_RESET → GPIO8  
LCD_TE → GPIO13  
  
**Touch: FT3168**  
  
RESET → GPIO9  
Interrupt → GPIO38  
I2C_SDA → GPIO15  
I2C_SCL → GPIO14  
  
**PMU: AXP2101**  

I2C_SDA → GPIO15  
I2C_SCL → GPIO14  

(Controlled by DSI_PWR_EN)  

**6-Axis IMU: QMI8658**  

Interrupt → GPIO21  
I2C_SDA → GPIO15  
I2C_SCL → GPIO14  
  
**RTC: PCF85063**  

Interrupt → GPIO39  
I2C_SDA → GPIO15  
I2C_SCL → GPIO14  
  
**Audio**  

I2C for configuration:  
I2C_SDA → GPIO15  
I2C_SCL → GPIO14  
  
**Speaker: ES8311**  
  
I2S_ASDOUT → GPIO42  
I2S_MCLK → GPIO16  
I2S_SCLK → GPIO41  

**Microphone: ES7210**  
     
I2S_LRCK → GPIO45  
I2S_DSDIN → GPIO40  
  
**Storage**  
  
32MB Flash + 8MB PSRAM  
  
**Micro SD Card**  
  
MOSI → GPIO1  
SCK → GPIO2  
MISO → GPIO3  
SDCS → GPIO? (partially obscured in image, likely GPIO?—appears cut off)  
  
**Buttons / Control**  
  
BOOT → GPIO0  
PWR → GPIO10  
PA_CTRL → GPIO46  
  
</details>

<details><summary><strong>
DEVICE DIMENSIONS (unit: mm)
</strong></summary>

**Front View**  
  
Overall width: 42.00 mm  
Overall height: 50.80 mm  
Screen width (inner): 33.09 mm  
Screen height (inner): 40.51 mm  
Corner radius: R9.2  

**Side View**  

Thickness (main body): 12.90 mm  
Maximum thickness: 13.60 mm  

**Back View**  
  
Top section width: 22.00 mm  
Bottom section width: 22.00 mm  
Label: ESP32-S3-Touch-AMOLED-2.06  

**Strap**  
  
Total length: 250.00 mm  
Strap width: 22.00 mm  
  
</details>


<br><br>

### **Peripherals**

#### **I2S**

I2S is used as single peripheral.  
**I2S TX** is configured as Master, while **I2S RX** is set to slave mode.  
Audio codecs (ES7210/ES8311) are configured via I2C, and are used as slaves.   


#### **I2C**

```
[INFO ] Scanning I2C bus on GPIO15(SDA)/GPIO14(SCL) (ESP32_S3_WATCH app/src/main.rs:125)
[INFO ] Found device at address 0x18 (ESP32_S3_WATCH app/src/main.rs:132)
[INFO ] Found device at address 0x34 (ESP32_S3_WATCH app/src/main.rs:132)
[INFO ] Found device at address 0x40 (ESP32_S3_WATCH app/src/main.rs:132)
[INFO ] Found device at address 0x51 (ESP32_S3_WATCH app/src/main.rs:132)
[INFO ] Found device at address 0x6B (ESP32_S3_WATCH app/src/main.rs:132)
[INFO ] Found device at address 0x7E (ESP32_S3_WATCH app/src/main.rs:132)
```

0x18 === ES8311 (SPEAKER)  
0x34 === AXP2101 (PMU)  
0x38	 === FT3168 (TOUCH)  
0x40 === ES7210 (MICROPHONE)  
0x51 === PCF85063A (RTC)  
0x6B	 === QMI8658 (6-axis IMU)   


## **Graphical User Interface**

## **Applications**  


## **Power Management & Optimizations**  


<br><br>


## **Sponsor**

[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)

### **☕**

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
