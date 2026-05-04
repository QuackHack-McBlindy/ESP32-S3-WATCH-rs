# **ESP32-S3-WATCH-rs**

[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)


## **ESP32-S3-WATCH**

> [!NOTE]
> **🧑‍🦯 Personal project!**  
> **As I am blind this firmware is mostly focused on accessibility of the voice assistant functionality of the watch.**  
> **It will have touch and a graphical user interface -- but it will be BIG.**  
<br>

**Bare Metal** *(no_std)* **ESP32-S3** firmware written in Rust without the `esp-idf` API.   
Designed to be used as a personal voice assistant watch with media player and a notification system.     

Its up to [yo](https://github.com/QuackHack-McBlindy/yo) to write your own voice commands.  
as a reference: I have **57** [yo](https://github.com/QuackHack-McBlindy/dotfiles) scripts meaning my watch has **272684913** phrases available as voice commands.  

    


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
- [Sponsor](#sponsor)
- [License](#license)

<br><br>  


## **Demo**



https://github.com/user-attachments/assets/357bf377-0874-4f42-8d15-3532c31bc813

  
**Yes, it's fast!** *(TTS is bottleneck)*  

<br><br>


## **Overview**

`ESP32-S3-WATCH-rs` is a `no_std` Rust firmware for the ESP32-S3 based smartwatch. The primary goal is to create a fully voice‑controlled assistant that is highly accessible for blind and visually impaired users. All interactions can be performed via voice, and the graphical interface is designed with large, high‑contrast elements.  

The watch streams audio to a companion backend service called [`yo`](https://github.com/QuackHack-McBlindy/yo), which handles wake word detection, speech‑to‑text, intent recognition and execution, and text‑to‑speech synthesis. The watch itself streams microphone audio, serves TCP server for audio streaming to the speaker, manages notifications, plays media, and serves a web frontend for configuration and status which also can be used as a GET API.  

<br><br>

## **Project Structure & Design**  

I knew that if I did not do this properly - right away, that it would get really messy.  
I amm basically writing a complete voice-driven API on top of every available run-time option of `esp-hal`.   
Modular API and using only fully qualified paths everywhere, should help keep things as tiny as possible.      

```
📂 ESP32-S3-WATCH-rs
├── 📂 applications
├── 📂 base
│   ├── 📂 routes
│   │   └── 📂 ... 
│   │       └── 📂 ... 
│   ├── 📄 api.rs
│   ├── 📄 macros.rs
│   ├── 📄 uptime.rs
│   ├── 📄 wifi.rs
│   └── 📄 mod.rs
├── 📂 components
│   ├── 📄 axp2101.rs
│   ├── 📄 buttons.rs
│   ├── 📄 co5300.rs
│   ├── 📄 framebuffer.rs
│   ├── 📄 ft3168.rs
│   ├── 📄 pcf85063a.rs
│   ├── 📄 qspi_bus.rs
│   └── 📄 mod.rs
├── 📂 gui
│   ├── 📄 battery.rs
│   ├── 📄 homescreen.rs
│   ├── 📄 pages.rs
│   ├── 📄 rolex.rs
│   ├── 📄 time.rs
│   └── 📄 mod.rs
├── 📄 main.rs
└── 📄 state.rs
```


<br><br>


## **Roadmap**


Project roadmap - watch as `watch` grows and evolves along the road.  
Extend with more crazy ideas as they pop up.  

- [x] Async & WiFi
- [x] Handle multiple SSID/WiFi alternatives  
- [x] Buttons & Display (lights up on wake word detection)
- [x] i2s: RX Microphone  
- [x] i2s: TX Speaker  
- [x] i2s: Simultaneous RX & TX 
- [x] Voice Command Execution (Wake word, speech to shell command)
- [x] Media Player - Stream any audio to speaker (wav, mp3, flac, mp4, ...)
- [x] Intercom `ffmpeg -f alsa -i default -f s16le -ar 16000 -ac 2 - | nc <ESP_IP> 12345`
- [x] On-Device API
- [x] On-Device WebServer & Web Media Player   
- [ ] OTA (auto update from git repo?)
- [ ] Fully voice controlled. (Change any  option at run-time) 
- [x] Graphical User Interface
- [ ] Applications 
- [ ] Power optimised for battery operation  
- [ ] Notification system  
- [ ] Security & WireGuard?
- [ ] Phone calls/text message (Bluetooth)
- [x] Backend: `yo`

`yo` is not only the backend server service but it's also where you will write your voice commands.  
This is where your `ESP32-S3` microphone audio will be streamed.   

- [yo](https://github.com/QuackHack-McBlindy/yo)  
  - Wake Word Detection
  - Speech To Text
  - Text To Speech
  - Voice Command Execution
  - Control any device option with your voice!


<br><br>


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

If you prefer to handle your media manually in the web browser - good news!  
Your `ESP32-S3` is now serving a fully featured media player that can cast to your TV's and has built-in transcoding.  
You can go ahead and visit your device at:  

`http://<ESP_IP>:80/`  


https://github.com/user-attachments/assets/bdbd0250-b683-4ffa-b8cb-817d0589df1a 

https://github.com/user-attachments/assets/91760f4f-0f31-439e-bc6b-8d2960c62cd8


<br><br>


### **API**    

The API is designed to be easily expandable.       
*Fetch all your available endpoints at:* `curl http://<ESP_IP>:80/api`      
  
  
Using the internal API you can for example set the `ESP32-S3` display brightness to 75 percentage using:    

```bash
curl http://<ESP_IP>:80/api/settings/display/brightness/75 
```
  

| Endpoint | Description |
|----------|-------------|
| `/` | Serves the web frontend (HTML dashboard) |
| `/favicon.ico` | Serves the favicon (currently returns 404) |
| `/script.js` | Serves the JavaScript frontend logic |
| `/api` | Returns a plain‑text list of all available API endpoints |
| `/api/update` | Trigger OTA firmware update |
| `/api/settings/power/state/{value}` | Control device power: `on`, `off`, or `toggle` (default) |
| `/api/settings/display/state/{value}` | Control display on/off: `on`, `off`, or `toggle` |
| `/api/settings/display/brightness/{value}` | Set backlight brightness (0–80%). `{value}` as integer percent |
| `/api/settings/mic/volume/{value}` | Set microphone gain (0–100%). Returns current volume |
| `/api/settings/mic/mute/{value}` | Mute/unmute mic: `1`/`on`/`mute`, `0`/`off`/`unmute`, or `toggle` |
| `/api/settings/speaker/volume/{value}` | Set speaker volume (0–100%) |
| `/api/settings/speaker/mute/{value}` | Mute/unmute speaker: same options as mic mute |
| `/api/settings/voice/state/{value}` | Voice recording command: `start` or `stop` |
| `/api/voice/detected` | Called when voice is detected; sets brightness to 70% and returns `"OK"` |
| `/api/voice/executed` | Called after a voice command succeeds; sets brightness to 0% and returns `"OK"` |
| `/api/voice/failed` | Called after a voice command fails; sets brightness to 0% and returns `"OK"` |
| `/api/media/{action}` | Media control (e.g., `play`, `pause`, `next`, `prev`) |
| `/api/sensor/{value}` | Read a sensor or system value (see supported keys below) |

### Supported sensor keys for `/api/sensor/{value}`

| Key | Description |
|-----|-------------|
| `battery`, `battery_level`, `battery_percentage` | Battery charge % (e.g., `78`) |
| `battery_voltage`, `voltage` | Battery voltage in V (e.g., `3.84`) |
| `rssi`, `wifi_signal`, `wifi` | Wi‑Fi signal strength in dBm (e.g., `-54`) |
| `ip` | Device IP address (e.g., `192.168.1.122`) |
| `uptime` | System uptime (e.g., `3d 14h`) |
| `firmware`, `version` | Firmware version string (e.g., `v2.1.0`) |


<br><br>



### **Media Player** 


> [!NOTE]
> __🎵 Media Player__ <br>
> **Media player supports any file format, and can play any file or playlist.**  
> **You can use the provided `scripts/play-esp.sh` helper script to stream audioo to the device speaker.**
<br>

<br><br>

 
## **Voice Assistant**


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

<br><br>


### **My Voice Commands**

<!-- MY_VOICE_COMMANDS_START -->

My voice assistant can currently execuute **57** voice scripts.   
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

<br><br>

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

<br><br>

## **Graphical User Interface**

<br><br>


## **Applications**  

<br><br>


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


