// BASE/KEYBOARD

use embedded_io_async::{Write, Read};

const MAX_LINE_LEN: usize = 256;
const HISTORY_SIZE: usize = 16;

pub async fn run_console<IO: embedded_io_async::Read + embedded_io_async::Write>(
    io: IO,
    _greeting: &str,
    mut on_line: impl core::ops::FnMut(&str),
    on_cancel: impl core::ops::FnOnce(),
) {
    let mut editor = embedded_keyboard::LineEditor::<IO, MAX_LINE_LEN, HISTORY_SIZE>::new(
        embedded_keyboard::Keyboard::new(io),
    );

    loop {
        match editor.read_line().await {
            core::result::Result::Ok(core::option::Option::Some(line)) => {
                defmt::info!("keyboard: {}", line.as_str());
                on_line(&line);
            }
            core::result::Result::Ok(core::option::Option::None) => {
                defmt::info!("keyboard: Ctrl‑C, exiting");
                on_cancel();
                break;
            }
            core::result::Result::Err(_e) => {
                defmt::info!("keyboard: error, exiting");
                break;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// KEYBOARD TASK
#[embassy_executor::task]
async fn ssh_keyboard_task(
    handle: sunset::ChanHandle,
    server: &'static sunset_async::SSHServer<'static>,
) {
    let mut chan = server.stdio(handle).await.unwrap();
    defmt::info!("SSH: live keyboard started");

    // START DISPLAY & OPEN TEXT INPUT GUI PAGE
    crate::DISPLAY_CMD.send(crate::DisplayCommand::Start).await;
    crate::store!(crate::state::DISPLAY_STATE, true);
    crate::gui::input::open(" ", "", |_| {}, crate::gui::pages::Page::Clock);

    let mut buf = [0u8; 1];
    loop {
        match chan.read(&mut buf).await {
            Ok(0) => {
                defmt::info!("SSH: EOF – exiting");
                break;
            } // WE READ ONE AT TIME
            Ok(1) => { // FOR A INSTANT LIVE KEYBOARD FEED 
                let b = buf[0];
                if b == b'\r' || b == b'\n' {
                    // ENTER KEY PRESSED – SUBMIT THE INPUT DATA COLLECTED
                    crate::gui::input::hit_ok();
                    // RE-OPEN THE PAGE - MAYBE MORE LINES (?)
                    crate::gui::input::open("SSH:", "", |_| {}, crate::gui::pages::Page::Clock);
                } else if b == 0x03 {
                    // CTRL+C – CANCEL & EXIT!
                    crate::gui::input::hit_cancel();
                    break;
                } else if b.is_ascii_graphic() || b == b' ' {
                    // ASCII CHARACTER - ADD TO THE TEXT FIELD
                    crate::gui::input::push_char(b as char);
                    crate::dirty!(); 
                    // PLEASE DON'T GO DARK WHEN WE TYPE!!
                    crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                }
                // IGNORE REST FOR NOW
            }
            Err(e) => {
                defmt::info!("SSH: read error – exiting");
                break;
            }
            _ => {}
        }
    }
    defmt::info!("SSH: keyboard task ended");
}

