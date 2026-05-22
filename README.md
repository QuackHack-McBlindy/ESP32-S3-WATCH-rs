# **ESP32-S3-WATCH-rs**

[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)


## **ESP32-S3-WATCH**

> [!NOTE]
> **🧑‍🦯 Personal project!**  
> **As I am blind this OS is mostly focused on accessibility through the voice assistant functionality of the watch.**  
> **It has a full graphical user interface -- but it's designed for a blind man, so it's BIG.**  
<br>


**Bare Metal** *(no-std)* voice-driven wearable OS for **ESP32-S3** smartwatch with a high focus on accessibility and smart home and media control. Written in Rust, using `esp-hal` as it's hardware abstraction layer.  
There is no usage of the `ESP-IDF` API in this firmware.   
  
Designed to be used as a personal voice assistant watch with **EXTENSIVE** custom voice commands & **much much more!** *(I flush the toilet with this thing..)*  

*“Source code is the best documentation.“*   

<br>
Be sure to check out the demo usage videos/pictures down below.  
<br>

Its up to [yo](https://github.com/QuackHack-McBlindy/yo) to write your own voice commands.  
My watch can execute **57** different [scripts](https://github.com/QuackHack-McBlindy/dotfiles), and understands **272684913**  different phrases as voice commands **-- with a average processing time of 2,72 ms per command**.  
  
The top-tier performance come from a **deterministic** voice architecture and smart caching inside the voice intents.  
  
 

## **Table Of Contents**

- [Demo](#demo)
- [Overview](#overview)
- [Roadmap](#roadmap)
- [Installation](#installation)
- [Usage](#usage)
  - [Frontend](#frontend)
  - [API](#api)
  - [Media Player](#media-player)
- [Voice Assistant](#voice-assistant)
  - [Architecture](#architecture)
  - [My Voice Commands](#my-voice-commands)
- [Hardware](#hardware)
  - [Peripherals](#peripherals)
    - [I2S](#i2s)
    - [I2C](#i2c)
- [Graphical User Interface](#graphical-user-interface)
- [Applications](#applications)
- [Power Management/Optimization](#power-management--optimizations)
- [♥️ Sponsor](#sponsor)
- [License](#license)

<br>  


## **Demo**

#### **Voice Assistant Video**  


Turn up volume & hit play.  

[Play voice assistant wake word demo video](https://github.com/user-attachments/assets/357bf377-0874-4f42-8d15-3532c31bc813)

  
**Yes, it's fast!** *(TTS generation is the bottleneck)*  


<br>

[Play demo video of media player](https://github.com/user-attachments/assets/6880aa18-7a8a-437c-bca0-b99c03bb1682)


<br>

#### **Pictures**

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/clock.jpeg">
  <img src="resource/demo/clock.jpeg" alt="Clock" width="148">
</a>

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/battery.jpeg">
  <img src="resource/demo/battery.jpeg" alt="Battery" width="148">
</a>

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/charging.jpeg">
  <img src="resource/demo/charging.jpeg" alt="Charging" width="148">
</a> <br>

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/launcher.jpeg">
  <img src="resource/demo/launcher.jpeg" alt="Launcher" width="148">
</a>

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/qwackify.jpeg">
  <img src="resource/demo/qwackify.jpeg" alt="Media Player -Qwackify" width="148">
</a>

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/settings.jpeg">
  <img src="resource/demo/settings.jpeg" alt="Settings page" width="148">
</a>

<a href="https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/main/resource/demo/calling.jpeg">
  <img src="resource/demo/calling.jpeg" alt="Dad is calling" width="148">
</a>


  
**Sorry, I have been told I am the master of blurry pictures.**  
*(how would I know...)*  

There is a video demo from the web-based Qwackify too down below.  

<br>


## **Overview**


*“A powerful voice assistant can make a huge difference for blind people.”*  
*“Imagine yourself stumbling blindly across the room looking for the remote — meanwhile, I cam call it using only my voice.”*  


`ESP32-S3-WATCH-rs` is a `no_std` Rust firmware for the ESP32-S3 based smartwatch. The primary goal is to create a fully voice‑controlled assistant that is highly accessible for blind and visually impaired users. All interactions can be performed via voice, and the graphical interface is designed with large, high‑contrast elements.  

The watch streams audio to a companion backend service called [`yo`](https://github.com/QuackHack-McBlindy/yo), which handles wake word detection, speech‑to‑text, intent recognition and execution, and text‑to‑speech synthesis. The watch itself streams microphone audio, serves TCP server for audio streaming to the speaker, manages notifications, plays media, and serves a web frontend for web based media playback in the browser called **Qwackify**, which is like Jellyfin on steroids - but much more good looking and served directly from a watch.  <br>

The watch also has an internal API serving GET endpoints for controlling options and media playback -- playing local music straight from the micro SD card is as easy as:  <br>
**From your desktop:** `curl http://<ESP_IP>/api/media/search/songs/mysong`.  <br>
**From the watch:** tap the boot button to open the app launcher, swipe down to the app `Qwackify` and double tap it to open then press play.  <br>
**Using your voice, say:** `yo bitch! play the duck song`.  

<br>

## **Project Structure & Design**  

I knew that if I did not do this properly like I want it - right away, that it would get really messy, and never get done.  
I amm basically writing a complete voice-driven API on top of every available run-time option of `esp-hal`.   
Modular API and using use statements only when needed, otherwise fully qualified paths everywhere, should help keep things as tiny as possible.      
I also prefer to have some of the extensive code as library crates, it can be useful for other people or simplify usage for myself on my other devices.  

```
📂 ESP32-S3-WATCH-rs
├── 📂 applications
│   ├── 📄 media_player.rs
│   ├── 📄 house.rs
│   └── 📄 mod.rs
├── 📂 base
│   ├── 📂 routes
│   │   └── 📂 ... 
│   │       └── 📂 ... 
│   ├── 📄 assets.rs
│   ├── 📄 api.rs
│   ├── 📄 bluetooth.rs
│   ├── 📄 macros.rs
│   ├── 📄 uptime.rs
│   ├── 📄 wifi.rs
│   └── 📄 mod.rs
├── 📂 components
│   ├── 📄 axp2101.rs
│   ├── 📄 buttons.rs
│   ├── 📄 co5300.rs
│   ├── 📄 frequency.rs
│   ├── 📄 framebuffer.rs
│   ├── 📄 ft3168.rs
│   ├── 📄 pcf85063a.rs
│   ├── 📄 qmi8658.rs
│   ├── 📄 qspi_bus.rs
│   ├── 📄 storage.rs
│   └── 📄 mod.rs
├── 📂 crates
│   ├── 📂 barely-fuzzy
│   ├── 📂 embedded-png
│   ├── 📂 es7210
│   ├── 📂 es8311
│   ├── 📂 tinyapi
│   └── 📂 yo-esp
├── 📂 gui
│   ├── 📄 animations.rs
│   ├── 📄 apps.rs
│   ├── 📄 battery.rs
│   ├── 📄 call.rs
│   ├── 📄 colors.rs
│   ├── 📄 house.rs
│   ├── 📄 media_player.rs
│   ├── 📄 pages.rs
│   ├── 📄 settings.rs
│   ├── 📄 text.rs
│   ├── 📄 time.rs
│   └── 📄 mod.rs
├── 📄 main.rs
└── 📄 state.rs
```


<br>


## **Roadmap**


Project roadmap - watch as `watch` grows and evolves along the road.  
Extend with more crazy ideas as they pop up. `ESP32-S3-WATCH-rs` is still under active development, and will be as long as this list is not complete.  
**Feel free to contribute with any cool ideas you might have for the `ESP32-S3`! - Nothing is impossible!**  

- [x] Async & WiFi
- [x] Handle multiple SSID/WiFi alternatives  
- [x] Buttons & Deep Sleep (power-save mode)
- [x] Interactive Shell terminal (Bash syntax over HTTP)
- [x] Remote file upload to the SD card (streamed chunks, POST)
- [x] Remote file download from the SD card (streamed chunks)
- [x] i2s: RX Microphone  
- [x] i2s: TX Speaker  
- [x] i2s: Simultaneous RX & TX  (Full-Duplex)
- [x] Voice Command Execution (Wake word, speech to shell command)
- [x] Push-to-talk feature (more battery effecient)
- [x] Media Player - Stream any audio to speaker (wav, mp3, flac, mp4, ...)
- [x] Fuzzy search & play local media from the SD card. (with downsampling) 
- [x] Intercom `ffmpeg -f alsa -i default -f s16le -ar 16000 -ac 2 - | nc <ESP_IP> 12345`
- [x] On-Device API
- [x] On-Device WebServer & Web Media Player (with casting to Android TV)
- [ ] Draw graphs on watch from input data 
- [ ] Generate on-device QR codes. (need TLS for secure secret sharing via QR)
- [x] Broadcasting all text-to-speech to every ESP32 device.   
- [ ] Phone calls/text message (Bluetooth HandsFree Protocol)
- [x] Backend: `yo`

`yo` is not only the backend server service but it's also where you will write your voice commands.  
This is where your `ESP32-S3` microphone audio will be streamed.   

- [yo](https://github.com/QuackHack-McBlindy/yo)  
  - Wake Word Detection
  - Speech To Text
  - Text To Speech
  - Voice Command Execution
  - Control any device option with your voice!


<br>


## **Installation**

<details><summary><strong>
❄️ Using flakes (TODO)
</strong></summary>

*sorry, not yet...*

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


<br>

## **Usage**


### **Frontend**  

If you prefer to handle your media manually in the web browser - good news!  
Your `ESP32-S3` is now serving a fully featured media player that can cast to your TV's and has built-in transcoding (transcoding on backend).  
You can go ahead and visit your device at:  

`http://<ESP_IP>/`  


https://github.com/user-attachments/assets/bdbd0250-b683-4ffa-b8cb-817d0589df1a 

https://github.com/user-attachments/assets/91760f4f-0f31-439e-bc6b-8d2960c62cd8


<br>


### **API**    

The API is designed to be easily expandable.       
*Fetch all your available endpoints at:* `curl http://<ESP_IP>/api`      
  
  
Using the internal API you can for example set the `ESP32-S3` display brightness to 75 percentage using:    

```bash
curl http://<ESP_IP>/api/settings/display/brightness/75 
```
  

| Endpoint | Description |
|----------|-------------|
| `/` | Serves the web frontend (HTML dashboard) |
| `/favicon.ico` | Serves the favicon |
| `/api` | Returns a plain‑text list of all available API endpoints |
| `/api/shell/{cmd}` | Send a Shell command (see supported commands below) |
| `/api/upload/file/{file}` | Upload any file to the root of the SD card **Note: POST** |
| `/api/upload/file/music/{file}` | Upload a song to the SD cards `/Music` directory **Note: POST** |
| `/api/download/file/{file}` | Download any file from the `/share` directory of the SD card |
| `/api/download/file/music/{file}` | Download a song from the SD cards `/Music` directory |
| `/api/sensor/{value}` | Read a single sensor/system value (see supported keys below) |
| `/api/sensors` | Returns all sensor/system values as JSON |
| `/api/media/play` | Sends `play` command to the media player. Starts the playback |
| `/api/media/pause` | Sends `pause` command to the media player. Pauses the playback |
| `/api/media/previous` | Sends `previous` command to the media player. Plays previous track |
| `/api/media/next` | Sends `next` command to the media player. Plays next track |
| `/api/media/search/songs/{song}` | Fuzzy search & play MP3 files from SD card. Returns matching song names and starts playback |
| `/api/settings/bluetooth/{value}` | Set bluetooth state (on/off). |
| `/api/settings/mic/volume/{value}` | Set microphone gain (0–100%). `{value}` as integer percent |
| `/api/settings/mic/mute/{value}` | Mute/unmute mic: `1`/`on`/`mute`, `0`/`off`/`unmute`, or `toggle` |
| `/api/settings/speaker/volume/{value}` | Set speaker volume (0–100%) |
| `/api/settings/speaker/mute/{value}` | Mute/unmute speaker: same options as mic mute |
| `/api/settings/voice/wakeword/{value}` | GET | Enable/disable wake‑word streaming (`on`, `off`, `enable`, `disable`) |
| `/api/settings/display/brightness/{value}` | Set backlight brightness (0–100%). `{value}` as integer percent |
| `/api/settings/display/state/{value}` | Set display state (on/off). |
| `/api/settings/display/call/{value}` | Run this endpoiint with the callers name from iPhone when you receieve a phone call to display the calling page on the watch. This page let's user accept/decline the call. |
| `/api/settings/display/page/{value}` | Change display page. `{value}` integer: 0=clock,1=battery,2=apps,10=media player, etc. |
| `/api/settings/display/text/{value}` | Displays the provided value as a large text on the display. |
| `/api/settings/wifi/set/ssid/{ssid}/password/{password}` | Saves a WiFi SSID to the WiFI connection list |
| `/api/settings/wifi/off` | Turns off the WiFi **Note: turning on WiFi via API call not possible!** |


### Supported sensor keys for `/api/sensor/{value}`

| Key(s)                                                       | Description                         |
|--------------------------------------------------------------|-------------------------------------|
| `battery`, `battery_level`, `battery_percentage`             | Battery charge in percent           |
| `battery_voltage`, `voltage`                                 | Battery voltage in millivolts       |
| `battery_charging`                                           | Charging status (0 or 1)            |
| `battery_need_charging`                                      | Low battery warning (0 or 1)        |
| `battery_full`                                               | Battery full flag (0 or 1)          |
| `battery_usb_connected`                                      | USB connection status (0 or 1)      |
| `brightness`, `display`                                      | Display brightness (0–100)          |
| `display_state`                                              | Display power state (0 = off, 1 = on)|
| `rssi`, `wifi_signal`, `wifi`                                | Wi‑Fi signal strength in dBm        |
| `ip`                                                         | Device IPv4 address                 |
| `speaker`                                                    | Speaker volume (0–100)              |
| `mic`                                                        | Microphone gain (0–100)             |
| `uptime`                                                     | System uptime (e.g., "02h 15m 30s") |
| `time`                                                       | Current time in HH:MM:SS            |
| `firmware`, `version`                                        | Firmware version string             |


### **Shell**

A Bash‑like interactive shell over HTTP.  
Commands are issued via `GET /api/shell/{command}`.  
Spaces must be encoded as `%20`, and slashes inside paths as `%2F`.  

**Examples:**  

```bash
# `ls /Music`
curl "http://<ESP_IP>/api/shell/ls%20%2FMusic"
```

Or check what is on the SD card from browser, visit: `http://<ESP_IP>/api/shell/tree`  
You should see something like:  

```
📂 .
├── 📂 Music
│   ├── 🎵 ducksong-1.mp3  
│   └── 🎵 ducksong-gangstarap.mp3
└── 📂 share
    └──  image.jpeg
```


| Command | Description |
|---------|-------------|
| `--help` or `help` | Print shell documentation and examples |
| `ls [path]` | List directory contents (relative or absolute) |
| `cd [path]` | Change working directory (`.` = stay, `..` = up one, `...` = up two, no arg = root) |
| `pwd` | Print current working directory |
| `nano <file>` | Creates a text file (TODO) |
| `cat <file>` | Display text file content |
| `hexdump <file>` | Show binary file in hexadecimal (truncated to ~500 chars) |
| `rm <file>` | Delete a file (no confirmation) |
| `mv <file> <path>` | Moves a file **Note: moving of very large is not possible** |
| `jq <file>` | Parse JSON file (TODO) |
| `tree [path]` | Recursive directory listing |



> **Persistence:** The shell keeps state – `cd` into a directory, then `ls` and `cat` will use that directory for relative paths.  
> Use `pwd` to see where you are.  


<br>



### **Media Player** 


> [!NOTE]
> __🎵 Media Player__ <br>
> **Media player supports any file format, and can play any file or playlist.**  
> **You can use the provided `scripts/play-esp.sh` helper script to stream audioo to the device speaker.**
<br>

<br>

 
## **Voice Assistant**

Wake word detection is disabled by default to save battery.  
Hold the `BOOT` button to send voice commands, release it when done talking.   

### **Architecture**

Check out the official [yo repository](https://github.com/QuackHack-Mcblindy/yo).  

<br>

The **yo** voice assistant employs a dual-language architecture that separates grammar compilation from runtime interpretation.  
This design allows for a **rapid** fast, deterministic, and privacy-first offline-capable system.  
  
The architecture is fundamentally split into two parts:  

- **Compile-Time (Nix):** Acts as a grammar compiler. It takes declarative sentence templates from configuration files, expands them into all possible variants, and pre-compiles them into optimized regular expressions.  

- **Runtime (Rust):** Functions as a deterministic interpreter. It takes audio input, matches it against the pre-compiled patterns, extracts any defined parameters, and executes the corresponding script with those arguments.  

<br>

So - no more bad, expensive strategies like:  

> * speech ➤ LLM ➤ guess intent  

Instead the flow looks like:   

> * speech ➤ deterministic intent match ➤ script  

<br>

**Result:** Designed for speeed and safety. All heavy lifting is done at build-time.   

<br>


### **My Voice Commands**

<!-- MY_VOICE_COMMANDS_START -->

My voice assistant can currently execute **57** voice scripts.   
That is **2503** regex patterns and makes a total of **272684913** understandable phrases available as voice commands.  

| Command Syntax | Description | Example | Voice Ready |
|----------------|-------------|---------|--------------|
| Command Syntax               | Aliases    | Description | VoiceReady |
|------------------------------|------------|-------------|--|
| [yo deploy](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/system/deploy.nix) --host [--flake] [--user] [--repo] [--port] [--test] |  | Build and deploy a NixOS configuration to a remote host. Bootstraps, builds locally, activates remotely, and auto-tags the generation. | ✅ |
| [yo reboot](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/system/reboot.nix) [--host] | restart | Force reboot and wait for host | ✅ |
| [yo services](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/system/services.nix) --operation --service --host [--user] [--port] [--!] |  | Systemd service handler. | ✅ |
| [yo switch](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/system/switch.nix) [--flake] [--!] | rb | Rebuild and switch Nix OS system configuration. ('!' to test) | ✅ |
| [yo call](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/call.nix) --contactName --contactFile |  | Calls phone number from contact list | ✅ |
| [yo text](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/text.nix) --contactName --contactFile |  | Text message a phone number from contact list | ✅ |
| [yo calculator](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/productivity/calculator.nix) --expression | calc | Calculate math expressions | ✅ |
| [yo calendar](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/productivity/calendar.nix) [--operation] [--calenders] | kal | Calendar assistant. Provides easy calendar access. Interactive terminal calendar, or manage the calendar through yo commands or with voice. | ✅ |
| [yo clip2phone](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/productivity/clip2phone.nix) --copy |  | Send clipboard to an iPhone, for quick copy paste | ✅ |
| [yo hitta](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/productivity/hitta.nix) --search |  | Locate a persons address with help of Hitta.se | ✅ |
| [yo pull](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/productivity/pull.nix) [--flake] [--host] |  | Pull the latest changes from your dotfiles repo. Resets tracked files to origin/main but keeps local extras. | ✅ |
| [yo search](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/productivity/search.nix) --search [--token-file] [--num-results] |  | Perform web search using Kagi with Quick Answer | ✅ |
| [yo stores](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/stores.nix) --store_name [--location] [--radius] | store, shop | Finds nearby stores using OpenStreetMap data with fuzzy name matching. Returns results with opening hours. | ✅ |
| [yo travel](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/travel.nix) [--to] [--from] [--type] [--apikeyPath] |  | Public transportation helper. Fetches current bus, boat, train and air travel schedules. (Sweden) | ✅ |
| [yo weather](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/weather.nix) [--location] [--day] [--condition] [--locationPath] | weat | Weather Assistant. Ask anything weather related (3 day forecast) | ✅ |
| [yo ip-updater](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/network/ip-updater.nix) [--token1] [--token2] [--token3] |  | DDNS updater | ✅ |
| [yo shareWiFi](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/network/shareWiFi.nix) [--ssidFile] [--passwordFile] |  | creates a QR code of guest WiFi and push image to iPhone | ✅ |
| [yo speed](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/network/speed.nix) | st | Test internet download speed | ✅ |
| [yo call-remote](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/media/call-remote.nix) |  | Used to call the tv remote, for easy localization. | ✅ |
| [yo news](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/media/news.nix) [--apis] [--clear] |  | API caller and playlist manager for latest Swedish news from SR. | ✅ |
| [yo tv](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/media/tv.nix) [--typ] [--search] [--device] [--season] [--shuffle] [--tvshowsDir] [--moviesDir] [--musicDir] [--musicvideoDir] [--videosDir] [--podcastDir] [--audiobookDir] [--youtubeAPIkeyFile] [--webserver] [--defaultPlaylist] [--favoritesPlaylist] [--max_items] [--mqttUser] [--mqttPWFile] | remote | Android TV Controller. Fuzzy search all media types and creates playlist and serves over webserver for casting. Fully conttrollable. | ✅ |
| [yo tv-guide](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/media/tv-guide.nix) [--search] [--channel] [--jsonFilePath] | tvg | TV-guide assistant.. | ✅ |
| [yo copy](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/files/copy.nix) --from --to | cp | Copy a file or directory to a new location | ✅ |
| [yo list](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/files/list.nix) [--path] | ls | List directory contents with details | ✅ |
| [yo makedir](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/files/makedir.nix) --path | mkd | Create a new directory with parents if needed | ✅ |
| [yo move](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/files/move.nix) --from --to | mv | Move a file or directory to a new location | ✅ |
| [yo nano](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/files/nano.nix) --file --content |  | Write content to filepath | ✅ |
| [yo remove](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/files/remove.nix) --target | rm, delete | Remove files or directories safely | ✅ |
| [yo alarm](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/alarm.nix) --hours --minutes [--list] [--sound] | wakeup | Set an alarm for a specified time | ✅ |
| [yo battery](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/battery.nix) [--device] |  | Fetch battery level for specified device. | ✅ |
| [yo bed](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/bed.nix) [--part] [--state] |  | Bed controller | ✅ |
| [yo blinds](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/blinds.nix) [--state] |  | Turn blinds up/down | ✅ |
| [yo chair](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/chair.nix) [--part] [--state] |  | Chair controller | ✅ |
| [yo display](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/display.nix) --path |  | Creates a HTML image that can be displayed on the chat frontend. | ✅ |
| [yo findPhone](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/findPhone.nix) |  | Helper for locating Phone | ✅ |
| [yo house](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/house.nix) [--device] [--state] [--brightness] [--color] [--temperature] [--scene] [--all-lights] [--room] [--json] [--hue-key-file] |  | High-performance unified CLI for controlling all smart home devices. | ✅ |
| [yo kitchenFan](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/kitchenFan.nix) [--state] |  | Turns kitchen fan on/off | ✅ |
| [yo lights](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/lights.nix) [--state] |  | Lights toggle | ✅ |
| [yo state](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/state.nix) [--device] |  | Fetches the state of the specified device. | ✅ |
| [yo temperatures](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/temperatures.nix) |  | Get all temperature values from sensors and return a average value. | ✅ |
| [yo tibber](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/tibber.nix) [--mode] [--homeIDFile] [--APIKeyFile] [--filePath] [--user] [--pwfile] | el | Fetches home electricity price data | ✅ |
| [yo timer](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/timer.nix) [--minutes] [--seconds] [--hours] [--list] [--sound] |  | Set a timer | ✅ |
| [yo toilet](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/home/toilet.nix) |  | Flush the toilet | ✅ |
| [yo btc](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/btc.nix) [--filePath] [--user] [--pwfile] |  | Crypto currency BTC price tracker | ✅ |
| [yo chat](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/chat.nix) --text |  | No fwendz? Let's chat yo! | ✅ |
| [yo duckPUCK](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/duckPUCK.nix) [--mode] [--team] [--stat] [--count] [--dataDir] | puck | [🏒🦆] - Your Personal Hockey Assistant! - Expert commentary and analyzer specialized on Hockey Allsvenskan (SWE). Analyzing games, scraping scoreboards and keeping track of all dates annd numbers. | ✅ |
| [yo hockeyGames](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/hockeyGames.nix) [--type] [--days] [--team] [--dataDir] [--debug] | hag | Hockey Assistant. Provides Hockey Allsvenskan data and deliver analyzed natural language responses (TTS). | ✅ |
| [yo invokeai](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/invokeai.nix) --prompt [--host] [--port] [--outputDir] [--width] [--height] [--steps] [--cfgScale] [--seed] [--model] | genimg | AI generated images powered by InvokeAI | ✅ |
| [yo joke](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/joke.nix) [--jokeFile] |  | Duck says s funny joke. | ✅ |
| [yo post](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/post.nix) [--postalCodeFile] [--postalCode] |  | Check for the next postal delivery day. (Sweden) | ✅ |
| [yo reminder](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/reminder.nix) [--about] [--list] [--clear] [--user] [--pwfile] | remind | Reminder Assistant | ✅ |
| [yo shop-list](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/shop-list.nix) [--operation] [--item] [--list] [--mqttUser] [--mqttPWFile] |  | Shopping list management | ✅ |
| [yo suno](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/suno.nix) --prompt [--genre] | mg | AI generated lyrics and music files powered by Suno | ✅ |
| [yo timee](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/timee.nix) |  | Tells time, day, date & week | ✅ |
| [yo xmr](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/misc/xmr.nix) [--filePath] [--user] [--pwfile] |  | Crypto currency XMR price tracker | ✅ |
| [yo duckTrace](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/maintenance/duckTrace.nix) [--script] [--host] [--errors] [--monitor] | log | View duckTrace logs quick and quack, unified logging system | ✅ |
| [yo health](https://github.com/QuackHack-McBlindy/dotfiles/blob/main/bin/maintenance/health.nix) | hc | Check system health status across your machines. Returns JSON structured responses. | ✅ |
<!-- MY_VOICE_COMMANDS_END -->

<br>  


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
SDCS → GPIO   
  
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


<br>

### **Peripherals**

#### **I2S**

I2S is used as single peripheral.  
**I2S TX** is configured as Master, while **I2S RX** is set to slave mode.  
Audio codecs (ES7210/ES8311) are configured via I2C, and are used as slaves.   

<br>

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

<br>

## **Graphical User Interface**

The GUI is currently pretty basic. It has a default "home" page with a digital clock, and when swiping left it will display battery status.  
Pressing the boot button will display the application launcher which let's the user scroll smoothly through the applications, double tapping an image will open the app. 
A upwards swipe gesture will close the open application and display the digital clock again.  


<br>


## **Applications**  

**Qwackify** - a media player with play/pause & previous/next track buttons, title & progress bar.  
  
**House** - Smart Home application with some quick action buttons etc, most of the home control is done by voice anyway.  
  
Will extend with more applications as I think of any useful ones.  

<br>


## **Power Management & Optimizations**  

The watch is basically running the CPU on max all the time because of all the heavy lifting.   
Battery optimization will be done after some more usage.  
The biggest battery saving win you can do right now is holding down the power button for 5 seconds when it's not going to be actively used - which will put the device into deep sleep.  
Holding the  power button again will wake it up again.   

As battery is not really optimized yet, it drains pretty quickly but it recharges very fast too (rougly 10-15 minutes).  

<br>


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

