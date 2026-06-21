// BASE/ROUTES/API/SHELL
// BASH OVER THE SD CARD IMPLEMENDED AS HTTP
// COMMANDS: `--help`, `ls`, `cd`, `pwd`, `cat`, `hexdump`, `rm`, `tree`
// ───────────────────────────────────────────────────────────────────────

use crate::alloc::string::ToString;

pub(crate) static CWD: critical_section::Mutex<core::cell::RefCell<alloc::string::String>> =
    critical_section::Mutex::new(core::cell::RefCell::new(alloc::string::String::new()));

fn decode_percent20(s: &str) -> alloc::string::String {
    s.replace("%20", " ")
}

// ───────────────────────────────────────────────────────────────────────
// MAIN SHELL HANDLER
pub fn handle_shell(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let raw_cmd = req.param("value").unwrap_or("--help");
    let cmd = decode_percent20(raw_cmd);
    defmt::info!("Shell: {}", cmd.as_str());

    let mut parts = cmd.split_whitespace();
    let cmd = parts.next().unwrap_or("--help");
    let args: alloc::vec::Vec<&str> = parts.collect();
    let arg = args.get(0).copied();

    let out = match cmd {

        // ───────────────────────────────────────────────────────────────────────
        // `ls` - SHOW FILES IN DIRECTORY
        "ls" => {
            let path = resolve(arg);
            match crate::components::storage::list_dir(&path) {
                Ok(entries) => {
                    let mut s = alloc::format!("Directory of {}:\n", path);
                    for (n, d, sz) in entries {
                        if d { s.push_str(&alloc::format!("  [DIR]  {}\n", n)); }
                        else { s.push_str(&alloc::format!("  {:>8}  {}\n", sz, n)); }
                    }
                    s
                }
                Err(e) => alloc::format!("ls error: {:?}", e),
            }
        }

        // ───────────────────────────────────────────────────────────────────────
        // `cd` - CHANGE WORKING DIRECTORY        
        "cd" => {
            let new_path = match arg {
                None => alloc::string::String::from("/"),   // no arg → root
                Some(a) if a.chars().all(|c| c == '.') => {
                    // `cd .` === STAY, `cd ..` === UP ONE DIR, `cd ...` === UP TWO DIRS, ETC
                    let levels_up = a.len().saturating_sub(1);
                    let current = critical_section::with(|cs| CWD.borrow(cs).borrow().clone());
                    if levels_up == 0 {
                        current
                    } else {
                        let mut parts: alloc::vec::Vec<&str> = current.split('/').filter(|p| !p.is_empty()).collect();
                        for _ in 0..levels_up {
                            parts.pop();
                        }
                        if parts.is_empty() {
                            alloc::string::String::from("/")
                        } else {
                            alloc::format!("/{}", parts.join("/"))
                        }
                    }
                }
                Some(a) => resolve_abs(a),   // normal relative or absolute path
            };
        
            // VERIFY THE NEW PATH EXISTS (TRY TO LIST)
            match crate::components::storage::list_dir(&new_path) {
                Ok(_) => {
                    critical_section::with(|cs| *CWD.borrow(cs).borrow_mut() = new_path.clone());
                    alloc::format!("Changed to {}", new_path)
                }
                Err(_) => alloc::format!("cd: no such directory: {}", new_path),
            }
        }

        // ───────────────────────────────────────────────────────────────────────
        // `pwd` - SHOW WHERE WE ARE    
        "pwd" => critical_section::with(|cs| CWD.borrow(cs).borrow().clone()),

        // ───────────────────────────────────────────────────────────────────────
        // `cat` - PRINT FILE CONTENT (TEXT)       
        "cat" => match arg {
            Some(_) => {
                let path = resolve(arg);
                match crate::components::storage::read_file_to_vec(&path) {
                    Ok(data) => core::str::from_utf8(&data)
                        .unwrap_or("(binary)")
                        .to_string(),
                    Err(e) => alloc::format!("cat error: {:?}", e),
                }
            }
            None => alloc::string::String::from("cat: missing file"),
        },
          
        // ───────────────────────────────────────────────────────────────────────        
        // `hexdump` - SHOW BINARY FILE IN HEX
        "hexdump" => match arg {
            Some(_) => {
                let path = resolve(arg);
                match crate::components::storage::read_file_to_vec(&path) {
                    Ok(data) => {
                        let mut s = alloc::string::String::new();
                        for (i, chunk) in data.chunks(16).enumerate() {
                            s.push_str(&alloc::format!("{:08x}: ", i * 16));
                            for b in chunk { s.push_str(&alloc::format!("{:02x} ", b)); }
                            s.push('\n');
                            if s.len() > 500 { s.push_str("..."); break; }
                        }
                        s
                    }
                    Err(e) => alloc::format!("hexdump error: {:?}", e),
                }
            }
            None => alloc::string::String::from("hexdump: missing file"),
        },
      
        // ───────────────────────────────────────────────────────────────────────        
        // `--help` - SHOW HOW TO USE THIS ENDPOINT
        "--help" | "help" => alloc::string::String::from(
            "INTERACTIVE SHELL OVER HTTP\n\
             ──────────────────────────\n\
             Usage: curl \"http://<ip>/api/shell/<command>%20<arguments>\"\n\
             \n\
             COMMANDS:\n\
             \n\
             --help                   Show this help\n\
             ls [path]                List directory contents\n\
             cd [path]                Change working directory\n\
             pwd                      Print working directory\n\
             cat <file>               Display text file content\n\
             hexdump <file>           Show binary file in hexadecimal\n\
             rm <file>                Delete a file (no confirmation!)\n\
             tree [path]              Recursive directory tree\n\
             \n\
             PATH RULES:\n\
             \n\
             - Absolute paths start with a slash.  Because the URL router\n\
               interprets slashes, you must encode each slash as %2F.\n\
               Example:  ls%20%2FMusic  →  ls /Music\n\
             \n\
             - Relative paths are relative to the current working directory\n\
               (which persists across requests).\n\
               Example:  ls%20Music  →  ls Music  (if CWD is /)\n\
             \n\
             SPECIAL DOT NOTATION FOR cd:\n\
             \n\
             cd .                      Stay in the current directory\n\
             cd ..                     Go up one directory level\n\
             cd ...                    Go up two directory levels\n\
             (four dots = up three, etc.)\n\
             \n\
             EXAMPLES:\n\
             \n\
             curl \"http://<ip>/api/shell/ls%20%2FMusic\"\n\
             curl \"http://<ip>/api/shell/cd%20Music\"\n\
             curl \"http://<ip>/api/shell/pwd\"\n\
             curl \"http://<ip>/api/shell/cat%20readme.txt\"\n\
             curl \"http://<ip>/api/shell/hexdump%20%2FMusic%2Fsong.mp3\"\n\
             curl \"http://<ip>/api/shell/rm%20old.mp3\"\n\
             curl \"http://<ip>/api/shell/tree\"\n\
             \n\
             TIP:  Use `curl` without --data for GET requests.\n\
             The shell keeps state; use `cd` to navigate before other commands."
        ),

        // ───────────────────────────────────────────────────────────────────────
        // `jq` - PARSE JSON FILES (TODO)        
        //"jq" => match arg {
      
        // ───────────────────────────────────────────────────────────────────────
        // `rm` - REMOVE FILE (WARNING: NO CONFIRMATION)        
        "rm" => match arg {
            Some(_) => {
                let path = resolve(arg);
                match crate::components::storage::delete_file(&path) {
                    Ok(()) => alloc::format!("Deleted {}", path),
                    Err(e) => alloc::format!("rm error: {:?}", e),
                }
            }
            None => alloc::string::String::from("rm: missing file"),
        },
        
        // ───────────────────────────────────────────────────────────────────────
        // `tree` - TREE FORMED RECURSIVE DIRECTORY LISTING                        
        "tree" => {
            let start = resolve(arg);
            let mut out = alloc::string::String::new();
        
            // CHOOSE AN ICON BASED ON NAME/EXTENSION.
            fn icon(name: &str, is_dir: bool) -> &'static str {
                if is_dir {
                    if name == "assets" { " " } else { " " }
                } else {
                    // FILE ICONS BY EXTENSION.
                    // IMAGE FILES
                    if name.ends_with(".png") || name.ends_with(".jpg") ||
                       name.ends_with(".jpeg") || name.ends_with(".gif") ||
                       name.ends_with(".bmp") || name.ends_with(".webp") ||
                       name.ends_with(".svg") {
                        " "
                    }
                    // SOUND / AUDIO FILES
                    else if name.ends_with(".mp3") || name.ends_with(".wav") ||
                            name.ends_with(".flac") || name.ends_with(".ogg") ||
                            name.ends_with(".m4a") || name.ends_with(".aac") {
                        "🎵 "
                    }
                    // OTHER FILES (FALLBACK)
                    else { " " }
                }
            }
        
            // RECURSIVE TREE PRINTER
            fn tree_rec(
                path: &str,
                prefix: &str,
                out: &mut alloc::string::String,
                is_last: bool,
            ) {
                // READ DIRECTORY ENTRIES
                let entries = match crate::components::storage::list_dir(path) {
                    core::result::Result::Ok(e) => e,
                    core::result::Result::Err(_) => return,
                };
                // DIRECTORIES FIRST, THEN FILES
                // USE as_str() TO GET &str REFERENCES INTO THE ORIGINAL STRINGS
                let mut dirs: alloc::vec::Vec<(&str, bool, u32)> = entries.iter()
                    .filter(|(_, is_dir, _)| *is_dir)
                    .map(|(n, d, s)| (n.as_str(), *d, *s))
                    .collect();
                let mut files: alloc::vec::Vec<(&str, bool, u32)> = entries.iter()
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
        
                    // CHOOSE THE RIGHT TREE CONNECTOR
                    let connector = if is_last_entry { "└── " } else { "├── " };
                    out.push_str(prefix);
                    out.push_str(connector);
                    out.push_str(icon(name, is_dir));
                    out.push_str(name);
                    out.push('\n');
        
                    // RECURSE INTO SUBDIRECTORIES
                    if is_dir {
                        let new_prefix = if is_last_entry {
                            alloc::format!("{}    ", prefix)
                        } else {
                            alloc::format!("{}│   ", prefix)
                        };
                        let sub_path = alloc::format!("{}/{}", path.trim_end_matches('/'), name);
                        tree_rec(&sub_path, &new_prefix, out, is_last_entry);
                    }
        
                    // SAFETY LIMIT
                    if out.len() > 450 {
                        out.push_str("...\n");
                        return;
                    }
                }
            }
        
            // PRINT THE ROOT
            out.push_str(icon(&start, true));
            // IF THE ROOT PATH IS JUST "/" - SHOW "."
            let root_display = if start == "/" { "." } else { &start };
            out.push_str(root_display);
            out.push('\n');
            tree_rec(&start, "", &mut out, true);
        
            if out.is_empty() {
                alloc::string::String::from("(empty)")
            } else {
                out
            }
        }
        _ => alloc::string::String::from("Commands: ls, cd, pwd, cat, hexdump, rm, tree"),
    };

    tinyapi::Response::text(&out)
}

// ───────────────────────────────────────────────────────────────────────
// HELPERS

// JOIN THE CURRENT WORKING DIRECTORY WITH A RELATIVE PATH (OR RETURN ABSOLUTE)
pub(crate) fn resolve(arg: Option<&str>) -> alloc::string::String {
    let a = match arg {
        Some(a) => a,
        None => return critical_section::with(|cs| CWD.borrow(cs).borrow().clone()),
    };
    if a.starts_with('/') {
        a.to_string()
    } else {
        let cwd = critical_section::with(|cs| CWD.borrow(cs).borrow().clone());
        alloc::format!("{}/{}", cwd, a)
    }
}

// RESOLVES AN ABSOLUTE PATH DIRECTLY
pub(crate) fn resolve_abs(a: &str) -> alloc::string::String {
    if a.starts_with('/') {
        a.to_string()
    } else {
        let cwd = critical_section::with(|cs| CWD.borrow(cs).borrow().clone());
        alloc::format!("{}/{}", cwd, a)
    }
}
