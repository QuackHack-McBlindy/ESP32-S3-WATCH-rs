// BASE/SHELL


pub use menu::{Menu, Item, ItemType, Parameter, MenuCallbackFn, ItemCallbackFn, argument_finder};

use embedded_io::Write as EioWrite;
use embedded_io_async::{Write as AsyncWrite, Read as AsyncRead};
use core::fmt::Write as _;
use alloc::format;
use alloc::vec::Vec;

use crate::base::routes::api::shell::{resolve, resolve_abs, CWD};

// ───────────────────────────────────────────────────────────────────────
// STUB FUNC
pub fn get_battery_percent() -> u8 { 80 }
pub fn build_all_sensors_json() -> &'static str { r#"{"battery":80}"# }
pub fn media_play() {}
pub fn media_pause() {}
pub fn media_next() {}
pub fn media_prev() {}
pub fn media_heart() {}
pub fn media_search(_q: &str) -> &'static str { "No results" }
pub fn set_display_brightness(_val: u8) {}
pub fn display_on() {}
pub fn display_off() {}

// ───────────────────────────────────────────────────────────────────────
// APPSTATE
pub struct AppState { /* FIELDS */ }

impl AppState {
    pub fn new() -> Self { Self {} }
}


// ───────────────────────────────────────────────────────────────────────
// HELPERS

// ICON LOGIC - RETURNS A &'static str
fn icon(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        if name == "assets" { " " } else { " " }
    } else {
        if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg")
            || name.ends_with(".gif") || name.ends_with(".bmp") || name.ends_with(".webp")
            || name.ends_with(".svg")
        {
            " "
        } else if name.ends_with(".mp3") || name.ends_with(".wav") || name.ends_with(".flac")
            || name.ends_with(".ogg") || name.ends_with(".m4a") || name.ends_with(".aac")
        {
            "🎵 "
        } else {
            " "
        }
    }
}

// RECURSIVE TREE PRINTER – WRITES DIRECTLY TO THE GENERIC WRITER
fn tree_rec<W: EioWrite>(
    path: &str,
    prefix: &str,
    w: &mut W,
    is_last: bool,
) {
    let entries = match crate::components::storage::list_dir(path) {
        Ok(e) => e,
        Err(_) => return,
    };

    // SEPARATE DIRS & FILES, SORT EACH
    let mut dirs: Vec<(&str, bool, u32)> = entries.iter()
        .filter(|(_, is_dir, _)| *is_dir)
        .map(|(n, d, s)| (n.as_str(), *d, *s))
        .collect();
    let mut files: Vec<(&str, bool, u32)> = entries.iter()
        .filter(|(_, is_dir, _)| !*is_dir)
        .map(|(n, d, s)| (n.as_str(), *d, *s))
        .collect();
    dirs.sort_by_key(|(name, _, _)| *name);
    files.sort_by_key(|(name, _, _)| *name);
    let mut all = dirs;
    all.append(&mut files);

    let total = all.len();
    for (i, (name, is_dir, _)) in all.into_iter().enumerate() {
        let is_last_entry = i == total - 1;
        let connector = if is_last_entry { "└── " } else { "├── " };

        let _ = write!(w, "{}{}{}{}\r\n", prefix, connector, icon(name, is_dir), name);

        if is_dir {
            let new_prefix = if is_last_entry {
                alloc::format!("{}    ", prefix)
            } else {
                alloc::format!("{}│   ", prefix)
            };
            let sub_path = alloc::format!("{}/{}", path.trim_end_matches('/'), name);
            tree_rec(&sub_path, &new_prefix, w, is_last_entry);
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// ASYNCMENUBUF – BYTE BUFFER FOR SHELL OUTPUT
#[derive(Default)]
pub struct AsyncMenuBuf {
    pub buf: heapless::Vec<u8, 1024>,
}

impl AsyncMenuBuf {
    pub async fn flush<W>(&mut self, w: &mut W) -> sunset::Result<()>
    where
        W: AsyncWrite<Error = sunset::Error>,
    {
        let mut b: &[u8] = &self.buf;
        while !b.is_empty() {
            let l = w.write(b).await?;
            b = &b[l..];
        }
        self.buf.clear();
        Ok(())
    }
}

// embedded_io::Write IMPLEMENTATION (NO fmt::Write NEEDED)
impl embedded_io::ErrorType for AsyncMenuBuf {
    type Error = core::convert::Infallible;
}

impl embedded_io::Write for AsyncMenuBuf {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut written = 0;
        for &b in buf {
            // CARRIAGE ( `\r` ) RETURN BEFORE EVERY LINE ( `\n` )
            if b == b'\n' {
                if self.buf.push(b'\r').is_err() {
                    return Ok(written);
                }
            }
            if self.buf.push(b).is_err() {
                return Ok(written);
            }
            written += 1;
        }
        Ok(written)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────────
// SHELL COMMANDS FUNCTIONS (CALLBACKS)

// HELP CALLBACK
fn help_callback(
    menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    if let Some(cmd) = args.get(0) {
        for item in menu.items {
            if item.command == *cmd {
                writeln!(w, "{} - {}", item.command, item.help.unwrap_or("No description")).ok();
                if let ItemType::Callback { parameters, .. } = &item.item_type {
                    for param in *parameters {
                        match param {
                            Parameter::Mandatory { parameter_name, help } => {
                                writeln!(w, "  <{}>  {}", parameter_name, help.unwrap_or("")).ok();
                            }
                            Parameter::Optional { parameter_name, help } => {
                                writeln!(w, "  [{}]  {}", parameter_name, help.unwrap_or("")).ok();
                            }
                            Parameter::Named { parameter_name, help } => {
                                writeln!(w, "  --{}  {}", parameter_name, help.unwrap_or("")).ok();
                            }
                            Parameter::NamedValue { parameter_name, help, argument_name } => {
                                writeln!(w, "  --{} <{}>  {}",
                                    parameter_name,
                                    argument_name,
                                    help.unwrap_or("")
                                ).ok();
                            }
                        }
                    }
                }
                return;
            }
        }
        writeln!(w, "Unknown command: {}", cmd).ok();
    } else {
        writeln!(w, "Available commands:").ok();
        for item in menu.items {
            writeln!(w, "  {}  {}", item.command, item.help.unwrap_or("")).ok();
        }
        writeln!(w, "Type 'help <command>' for more details.").ok();
    }
}


pub fn sensor_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    if let Some(sensor_name) = args.get(0) {
        let value = match *sensor_name {
            "battery" | "battery_level" | "battery_percentage" => {
                format!("{}%", get_battery_percent())
            }
            _ => format!("Unknown sensor: {}", sensor_name),
        };
        writeln!(w, "{}", value).ok();
    } else { writeln!(w, "Usage: sensor <name>").ok(); }
}

pub fn sensors_all_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    _args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    let json = build_all_sensors_json();
    writeln!(w, "{}", json).ok();
}

pub fn media_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    let cmd = args.get(0);
    match cmd {
        Some(&"play") => { media_play(); writeln!(w, "Playing").ok(); }
        Some(&"pause") => { media_pause(); writeln!(w, "Paused").ok(); }
        Some(&"next") => { media_next(); writeln!(w, "Next track").ok(); }
        Some(&"prev") => { media_prev(); writeln!(w, "Previous track").ok(); }
        Some(&"heart") => { media_heart(); writeln!(w, "Added to favourites").ok(); }
        Some(&"search") => {
            if let Some(query) = args.get(1) {
                let results = media_search(query);
                writeln!(w, "{}", results).ok();
            } else { writeln!(w, "Usage: media search <query>").ok(); }
        }
        _ => { writeln!(w, "Unknown media command").ok(); }
    }
}

pub fn settings_callback(
    menu: &Menu<impl EioWrite, AppState>,
    item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    state: &mut AppState,
) {
    let sub = args.get(0);
    match sub {
        Some(&"display") => {
            let sub2 = args.get(1);
            match sub2 {
                Some(&"brightness") => {
                    if let Ok(val) = argument_finder(item, args, "value") {
                        if let Some(v) = val {
                            set_display_brightness(v.parse().unwrap_or(50));
                            writeln!(w, "Brightness set to {}", v).ok();
                        } else { writeln!(w, "Missing value").ok(); }
                    }
                }
                Some(&"state") => {
                    if let Ok(val) = argument_finder(item, args, "value") {
                        match val {
                            Some("on") | Some("1") | Some("true") => {
                                display_on(); writeln!(w, "Display on").ok();
                            }
                            _ => {
                                display_off(); writeln!(w, "Display off").ok();
                            }
                        }
                    }
                }
                _ => { writeln!(w, "Unknown display setting").ok(); }
            }
        }
        Some(&"speaker") => { }
        Some(&"wifi") => { }
        Some(&"power") => { }
        _ => { writeln!(w, "Usage: settings <domain> [options]").ok(); }
    }
}

// LS CALLBACK
fn ls_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    let path = resolve(args.get(0).copied());
    match crate::components::storage::list_dir(&path) {
        Ok(entries) => {
            let _ = writeln!(w, "Directory of {}:", path);
            for (n, d, sz) in entries {
                if d {
                    let _ = writeln!(w, "  [DIR]  {}", n);
                } else { let _ = writeln!(w, "  {:>8}  {}", sz, n); }
            }
        }
        Err(e) => { let _ = writeln!(w, "ls error: {:?}", e); }
    }
}

// CD CALLBACK
fn cd_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    let arg = args.get(0).copied();
    let new_path = match arg {
        None => alloc::string::String::from("/"),
        Some(a) if a.chars().all(|c| c == '.') => {
            let levels_up = a.len().saturating_sub(1);
            let current = critical_section::with(|cs| CWD.borrow(cs).borrow().clone());
            if levels_up == 0 {
                current
            } else {
                let mut parts: alloc::vec::Vec<&str> =
                    current.split('/').filter(|p| !p.is_empty()).collect();
                for _ in 0..levels_up {
                    parts.pop();
                }
                if parts.is_empty() {
                    alloc::string::String::from("/")
                } else { alloc::format!("/{}", parts.join("/")) }
            }
        }
        Some(a) => resolve_abs(a),
    };

    match crate::components::storage::list_dir(&new_path) {
        Ok(_) => {
            critical_section::with(|cs| *CWD.borrow(cs).borrow_mut() = new_path.clone());
            let _ = writeln!(w, "Changed to {}", new_path);
        }
        Err(_) => { let _ = writeln!(w, "cd: no such directory: {}", new_path); }
    }
}

// PRINT WORKING DIRECTORY CALLBACK
fn pwd_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    _args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    let cwd = critical_section::with(|cs| CWD.borrow(cs).borrow().clone());
    let _ = writeln!(w, "{}", cwd);
}

// CAT CALLBACK
fn cat_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    if let Some(file) = args.get(0) {
        let path = resolve(Some(file));
        match crate::components::storage::read_file_to_vec(&path) {
            Ok(data) => {
                let text = core::str::from_utf8(&data).unwrap_or("(binary)");
                let _ = writeln!(w, "{}", text);
            }
            Err(e) => {
                let _ = writeln!(w, "cat error: {:?}", e);
                defmt::info!("cat error!");
            }
        }
    } else { let _ = writeln!(w, "cat: missing file"); }
}

// HEXDUMP
fn hexdump_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    if let Some(file) = args.get(0) {
        let path = resolve(Some(file));
        match crate::components::storage::read_file_to_vec(&path) {
            Ok(data) => {
                for (i, chunk) in data.chunks(16).enumerate() {
                    let _ = write!(w, "{:08x}: ", i * 16);
                    for b in chunk { let _ = write!(w, "{:02x} ", b); }
                    let _ = writeln!(w);
                    // OPTIONAL EARLY STOP
                    if i > 30 {
                        let _ = writeln!(w, "...");
                        break;
                    }
                }
            }
            Err(e) => { let _ = writeln!(w, "hexdump error: {:?}", e); }
        }
    } else { let _ = writeln!(w, "hexdump: missing file"); }
}

// REMOVE FILE CALLBACK
fn rm_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    if let Some(file) = args.get(0) {
        let path = resolve(Some(file));
        match crate::components::storage::delete_file(&path) {
            Ok(()) => {
                let _ = writeln!(w, "Deleted {}", path);
            }
            Err(e) => { let _ = writeln!(w, "rm error: {:?}", e); }
        }
    } else { let _ = writeln!(w, "rm: missing file"); }
}

// TREE CALLBACK
fn tree_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    _item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    let start = resolve(args.get(0).copied());
    let root_display = if start == "/" { "." } else { start.as_str() };
    let _ = writeln!(w, "{}{}\r\n", icon(&start, true), root_display);
    tree_rec(&start, "", w, true);
}


// DISPLAY CALLBACK
fn display_callback(
    _menu: &Menu<impl EioWrite, AppState>,
    item: &Item<impl EioWrite, AppState>,
    args: &[&str],
    w: &mut impl EioWrite,
    _state: &mut AppState,
) {
    if let Ok(Some(val)) = argument_finder(item, args, "brightness") {
        if let Ok(percent) = val.parse::<u8>() {
            let percent = percent.clamp(0, 80);
            crate::store!(crate::state::DISPLAY_BRIGHTNESS, percent);
            writeln!(w, "Brightness set to {}%", percent).ok();
        } else { writeln!(w, "Invalid brightness value (0‑80)").ok(); }
    } else { writeln!(w, "Usage: display --brightness <0‑80>").ok(); }
}


// ───────────────────────────────────────────────────────────────────────
// SHELL COMMAND DEFINITIONS
pub static ROOT_MENU: Menu<AsyncMenuBuf, AppState> = Menu {
    label: "root",
    items: &[
        // HELP
        &Item {
            command: "help",
            help: Some("Show this help"),
            item_type: ItemType::Callback {
                function: help_callback,
                parameters: &[
                    Parameter::Optional {
                        parameter_name: "command",
                        help: Some("Command to get detailed help for"),
                    },
                ],
            },
        },      
        // NLP
        &Item {
            command: "nlp",
            help: Some("Process a natural language sentence and translate to a shell command for execution. Usage: nlp <input> [fuzzy_threashhold]"),
            item_type: ItemType::Callback {
                //function: nlp_callback,
                function: help_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "input",
                        help: Some("Human language sentence to process."),
                    },
                    Parameter::Optional {
                        parameter_name: "fuzzy_treshhold",
                        help: Some("Fuzzy threashhold (1-100) for when a sentence should be concidered a match. (default: 40)"),
                    },
                ],
            },
        },
        // DISPLAY
        &Item {
            command: "display",
            help: Some("Control display. Usage: display --brightness <0-80>"),
            item_type: ItemType::Callback {
                function: display_callback,
                parameters: &[
                    Parameter::Optional {
                        parameter_name: "brightness",
                        help: Some("Set brightness percentage (0‑80)"),
                    },
                ],
            },
        },
        // SENSOR
        &Item {
            command: "sensor",
            help: Some("Read a sensor value. Usage: sensor <key>"),
            item_type: ItemType::Callback {
                function: sensor_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "key",
                        help: Some("Sensor key (battery, rssi, ip, …)"),
                    },
                ],
            },
        },
        // SENSORS
        &Item {
            command: "sensors",
            help: Some("Show all sensor values as JSON"),
            item_type: ItemType::Callback {
                function: sensors_all_callback,
                parameters: &[],
            },
        },
        // MEDIA
        &Item {
            command: "media",
            help: Some("Control media player. Usage: media <play|pause|next|prev|heart|search> [query]"),
            item_type: ItemType::Callback {
                function: media_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "action",
                        help: Some("One of: play, pause, next, prev, heart, search"),
                    },
                    Parameter::Optional {
                        parameter_name: "query",
                        help: Some("Search query (only for 'search')"),
                    },
                ],
            },
        },
        // SETTINGS
        &Item {
            command: "settings",
            help: Some("Change device settings. Usage: settings <domain> [key] [value]"),
            item_type: ItemType::Callback {
                function: settings_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "domain",
                        help: Some("display, speaker, mic, wifi, power, voice, ..."),
                    },
                    Parameter::Optional {
                        parameter_name: "key",
                        help: Some("Setting name, e.g. brightness, volume"),
                    },
                    Parameter::Optional {
                        parameter_name: "value",
                        help: Some("New value"),
                    },
                ],
            },
        },        
        // LS
        &Item {
            command: "ls",
            help: Some("List directory contents. Usage: ls [path]"),
            item_type: ItemType::Callback {
                function: ls_callback,
                parameters: &[
                    Parameter::Optional {
                        parameter_name: "path",
                        help: Some("Directory path (absolute or relative)"),
                    },
                ],
            },
        },
        // CD
        &Item {
            command: "cd",
            help: Some("Change working directory. Usage: cd [path | dots]"),
            item_type: ItemType::Callback {
                function: cd_callback,
                parameters: &[
                    Parameter::Optional {
                        parameter_name: "path",
                        help: Some("Target directory (or . / .. / ... for up)"),
                    },
                ],
            },
        },
        // PWD
        &Item {
            command: "pwd",
            help: Some("Print current working directory"),
            item_type: ItemType::Callback {
                function: pwd_callback,
                parameters: &[],
            },
        },
        // CAT
        &Item {
            command: "cat",
            help: Some("Display file content. Usage: cat <file>"),
            item_type: ItemType::Callback {
                function: cat_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "file",
                        help: Some("File to display"),
                    },
                ],
            },
        },
        // HEXDUMP
        &Item {
            command: "hexdump",
            help: Some("Hex dump of a file. Usage: hexdump <file>"),
            item_type: ItemType::Callback {
                function: hexdump_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "file",
                        help: Some("File to dump"),
                    },
                ],
            },
        },
        // RM
        &Item {
            command: "rm",
            help: Some("Delete a file (no confirmation!). Usage: rm <file>"),
            item_type: ItemType::Callback {
                function: rm_callback,
                parameters: &[
                    Parameter::Mandatory {
                        parameter_name: "file",
                        help: Some("File to remove"),
                    },
                ],
            },
        },
        // TREE
        &Item {
            command: "tree",
            help: Some("Recursive directory tree. Usage: tree [path]"),
            item_type: ItemType::Callback {
                function: tree_callback,
                parameters: &[
                    Parameter::Optional {
                        parameter_name: "path",
                        help: Some("Starting directory"),
                    },
                ],
            },
        },                
    ], 
    
    // WELCOME MESSAGE
    entry: Some(|_, w, _| {
        let _ = write!(w, "Welcome to the ESP32 Shell!\r\n");
        let _ = write!(w, "Type 'help' for available commands.\r\n");
    }),
    exit: None,
};

// DISPATCHER FOR ROOT_MENU
pub fn menu_dispatch<W: EioWrite>(
    menu: &'static Menu<W, AppState>,
    tokens: &[&str],
    w: &mut W,
    state: &mut AppState,
) {
    if tokens.is_empty() {
        return;
    }
    let cmd = tokens[0];
    let args = &tokens[1..];
    for item in menu.items {
        if item.command == cmd {
            if let ItemType::Callback { function, .. } = &item.item_type {
                function(menu, *item, args, w, state);
            }
            return;
        }
    }
    let _ = write!(w, "Unknown command: {}\r\n", cmd);
}

// LIVE KEYBOARD FEED
pub async fn keyboard_live_feed(
    chan: &mut (impl AsyncRead<Error = sunset::Error> + AsyncWrite<Error = sunset::Error>),
) {
    defmt::info!("SSH: live keyboard started");
    // DISPLAY ON!
    crate::DISPLAY_CMD.send(crate::DisplayCommand::Start).await;
    crate::store!(crate::state::DISPLAY_STATE, true);
    // OPEN THE KEYBOARD INPUT PAGE
    crate::gui::input::open(" ", "", |_| {}, crate::gui::pages::Page::Clock); // SEND TO CLOCK PAGE AFTER SUBMIT

    let mut buf = [0u8; 1];
    loop {
        match chan.read(&mut buf).await {
            Ok(0) => {
                defmt::info!("SSH: keyboard EOF – exiting");
                break;
            }
            Ok(1) => {
                let b = buf[0];
                // ENTER - SEND INPUT
                if b == b'\r' || b == b'\n' {
                    crate::gui::input::hit_ok();
                    crate::gui::input::open("SSH:", "", |_| {}, crate::gui::pages::Page::Clock);
                // CANCEL
                } else if b == 0x03 {
                    crate::gui::input::hit_cancel();
                    break;
                // PUSH CHARACTER TO THE DEVICE INPUT
                } else if b.is_ascii_graphic() || b == b' ' {
                    crate::gui::input::push_char(b as char);
                    crate::dirty!();
                    // PLEASE DON'T TURN OFF DISPLAY WHILE I TYPE!
                    crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                }
            }
            Err(_) => {
                defmt::info!("SSH: keyboard read error – exiting");
                break;
            }
            _ => {}
        }
    }
    defmt::info!("SSH: keyboard task ended");
}

// FULL SHELL SESSION (USED BY SSH)
pub async fn shell_session(
    chan: &mut (impl AsyncRead<Error = sunset::Error> + AsyncWrite<Error = sunset::Error>),
) -> sunset::Result<()> {
    let mut out_buf = AsyncMenuBuf::default();
    let mut state = AppState::new();

    // WELCOME
    if let Some(entry_fn) = ROOT_MENU.entry {
        entry_fn(&ROOT_MENU, &mut out_buf, &mut state);
        out_buf.flush(chan).await.ok();
    }

    let _ = chan.write(b"$ ").await;
    let _ = chan.flush().await;

    let mut line_buf: heapless::Vec<u8, 256> = heapless::Vec::new();
    let mut in_byte = [0u8; 1];

    loop {
        match chan.read(&mut in_byte).await {
            Ok(0) => {
                defmt::info!("SSH: EOF – exiting shell");
                break;
            }
            Ok(1) => {
                let b = in_byte[0];
                match b {
                    b'\r' | b'\n' => {
                        let _ = chan.write(b"\r\n").await;
                        if let Ok(line_str) = core::str::from_utf8(&line_buf) {
                            let trimmed = line_str.trim();
                            if !trimmed.is_empty() {
                                let tokens: heapless::Vec<&str, 10> =
                                    trimmed.split_whitespace().collect();
                                if tokens[0] == "keyboard" {
                                    keyboard_live_feed(chan).await;
                                } else {
                                    menu_dispatch(&ROOT_MENU, &tokens, &mut out_buf, &mut state);
                                    out_buf.flush(chan).await.ok();
                                }
                            }
                        }
                        let _ = chan.write(b"$ ").await;
                        let _ = chan.flush().await;
                        line_buf.clear();
                    }
                    b'\x03' => {
                        let _ = chan.write(b"^C\r\n").await;
                        defmt::info!("SSH: shell terminated by Ctrl+C");
                        break;
                    }
                    b'\x08' | b'\x7f' => {
                        if !line_buf.is_empty() {
                            line_buf.pop();
                            let _ = chan.write(b"\x08 \x08").await;
                        }
                    }
                    _ => {
                        if b.is_ascii_graphic() || b == b' ' {
                            if line_buf.push(b).is_ok() {
                                let _ = chan.write(&[b]).await;
                            }
                        }
                    }
                }
            }
            Ok(n) => {
                defmt::warn!("SSH: unexpected read of {} bytes", n);
            }
            Err(e) => {
                defmt::info!("SSH: read error – exiting shell");
                break;
            }
        }
    }

    Ok(())
}
