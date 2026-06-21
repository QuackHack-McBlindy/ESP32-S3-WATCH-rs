#![no_std]
//! `embedded-keyboard` – bare metal, async keyboard input and line editing for any byte stream.
//!
//! - Transport‑agnostic: works with SSH, UART, USB‑CDC, TCP, etc.
//! - Raw key events (arrow keys, Ctrl‑C, …)
//! - Full‑featured line editor with insert, backspace, cursor movement,
//!   terminal redraw, and **history** (Up/Down).
//! - Echo and cursor positioning are entirely editor‑controlled;
//!   the `Keyboard` simply translates bytes into `KeyEvent`s.

use core::fmt::Write as FmtWrite;
use defmt;
use embedded_io_async::{Read, Write};
use heapless::{HistoryBuffer, String, Vec};

// ─────────────────────────────────────────────────────────────────
// Error types

#[derive(Debug)]
pub enum KeyboardError<E> {
    Io(E),
    Eof,
}

impl<E> From<E> for KeyboardError<E> {
    fn from(e: E) -> Self {
        KeyboardError::Io(e)
    }
}

// ─────────────────────────────────────────────────────────────────
// Key events

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyEvent {
    Char(char),
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    CtrlC,
}

// ─────────────────────────────────────────────────────────────────
// Keyboard – raw byte stream to KeyEvent

pub struct Keyboard<R: Read + Write> {
    io: R,
    pending_lf_ignore: bool,
}

impl<R: Read + Write> Keyboard<R> {
    pub fn new(io: R) -> Self {
        Self {
            io,
            pending_lf_ignore: false,
        }
    }

    async fn read_byte(&mut self) -> Result<u8, KeyboardError<R::Error>> {
        let mut buf = [0u8; 1];
        loop {
            match self.io.read(&mut buf).await {
                Ok(0) => return Err(KeyboardError::Eof),
                Ok(1) => return Ok(buf[0]),
                Err(e) => return Err(KeyboardError::Io(e)),
                _ => continue,
            }
        }
    }

    pub async fn read_key(&mut self) -> Result<KeyEvent, KeyboardError<R::Error>> {
        loop {
            let b = self.read_byte().await?;

            if b == 0x0D {
                self.pending_lf_ignore = true;
                return Ok(KeyEvent::Enter);
            }
            if b == 0x0A {
                if self.pending_lf_ignore {
                    self.pending_lf_ignore = false;
                    continue;
                }
                return Ok(KeyEvent::Enter);
            }

            if b == 0x08 || b == 0x7F {
                return Ok(KeyEvent::Backspace);
            }
            if b == 0x09 {
                return Ok(KeyEvent::Tab);
            }
            if b == 0x03 {
                return Ok(KeyEvent::CtrlC);
            }

            if b == 0x1B {
                let b2 = match self.read_byte().await {
                    Ok(b) => b,
                    Err(_) => return Ok(KeyEvent::Escape),
                };
                if b2 == b'[' {
                    let mut params = Vec::<u8, 4>::new();
                    let final_byte = loop {
                        let b = self.read_byte().await?;
                        if (0x40..=0x7E).contains(&b) {
                            break b;
                        } else if b == 0x1B {
                            return Ok(KeyEvent::Escape);
                        } else {
                            if params.len() < params.capacity() {
                                params.push(b).ok();
                            }
                        }
                    };

                    match final_byte {
                        b'A' => return Ok(KeyEvent::Up),
                        b'B' => return Ok(KeyEvent::Down),
                        b'C' => return Ok(KeyEvent::Right),
                        b'D' => return Ok(KeyEvent::Left),
                        b'H' => return Ok(KeyEvent::Escape),
                        b'F' => return Ok(KeyEvent::Escape),
                        b'~' => {
                            if params.len() == 1 && params[0] == b'3' {
                                return Ok(KeyEvent::Backspace);
                            }
                        }
                        _ => return Ok(KeyEvent::Escape),
                    }
                } else {
                    return Ok(KeyEvent::Escape);
                }
            }

            if b >= 0x20 && b <= 0x7E {
                return Ok(KeyEvent::Char(b as char));
            }

            defmt::warn!("Keyboard: ignored byte 0x{:02X}", b);
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Line editor – key events > editable line with terminal redraw
// and history support (Up/Down)

pub struct LineEditor<R: Read + Write, const MAX: usize = 256, const HIST: usize = 8> {
    keyboard: Keyboard<R>,
    buffer: Vec<u8, MAX>,
    cursor: usize,
    prompt_len: usize,
    history: HistoryBuffer<String<MAX>, HIST>,
    history_index: Option<usize>,
}

impl<R: Read + Write, const MAX: usize, const HIST: usize> LineEditor<R, MAX, HIST> {
    pub fn new(keyboard: Keyboard<R>) -> Self {
        Self {
            keyboard,
            buffer: Vec::new(),
            cursor: 0,
            prompt_len: 0,
            history: HistoryBuffer::new(),
            history_index: None,
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt_len = prompt.len();
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history_index = None;
    }

    pub async fn read_line(
        &mut self,
    ) -> Result<Option<String<MAX>>, KeyboardError<R::Error>> {
        self.history_index = None;

        loop {
            let key = self.keyboard.read_key().await?;
            match key {
                KeyEvent::Char(c) => {
                    self.history_index = None;
                    self.insert_char(c).await?;
                }
                KeyEvent::Backspace => {
                    self.history_index = None;
                    self.delete_char().await?;
                }
                KeyEvent::Enter => {
                    self.write_all(b"\r\n").await?;
                    let line = core::str::from_utf8(&self.buffer)
                        .unwrap_or("(invalid UTF-8)");
                    let mut result = String::new();
                    result.push_str(line).ok();
                    if !result.is_empty() {
                        self.history.write(result.clone());
                    }
                    self.buffer.clear();
                    self.cursor = 0;
                    return Ok(Some(result));
                }
                KeyEvent::CtrlC => {
                    self.write_all(b"\r\n").await?;
                    self.buffer.clear();
                    self.cursor = 0;
                    return Ok(None);
                }
                KeyEvent::Left => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                        self.move_cursor_left().await?;
                    }
                }
                KeyEvent::Right => {
                    if self.cursor < self.buffer.len() {
                        self.cursor += 1;
                        self.move_cursor_right().await?;
                    }
                }
                KeyEvent::Up => {
                    if self.history.is_empty() {
                        continue;
                    }
                    let new_idx = match self.history_index {
                        None => Some(self.history.len().saturating_sub(1)),
                        Some(i) if i > 0 => Some(i - 1),
                        _ => continue,
                    };
                    self.history_index = new_idx;
                    self.load_history_entry().await?;
                }
                KeyEvent::Down => {
                    match self.history_index {
                        None => continue,
                        Some(i) if i + 1 < self.history.len() => {
                            self.history_index = Some(i + 1);
                            self.load_history_entry().await?;
                        }
                        _ => {
                            self.history_index = None;
                            self.clear_line_and_buffer().await?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Internal helpers
    async fn write_all(&mut self, data: &[u8]) -> Result<(), KeyboardError<R::Error>> {
        let mut remaining = data;
        while !remaining.is_empty() {
            let n = self.keyboard.io.write(remaining).await?;
            if n == 0 {
                return Err(KeyboardError::Eof);
            }
            remaining = &remaining[n..];
        }
        Ok(())
    }

    async fn move_cursor_left(&mut self) -> Result<(), KeyboardError<R::Error>> {
        self.write_all(b"\x1B[D").await
    }

    async fn move_cursor_right(&mut self) -> Result<(), KeyboardError<R::Error>> {
        self.write_all(b"\x1B[C").await
    }

    async fn erase_to_end(&mut self) -> Result<(), KeyboardError<R::Error>> {
        self.write_all(b"\x1B[K").await
    }

    async fn insert_char(&mut self, c: char) -> Result<(), KeyboardError<R::Error>> {
        if self.buffer.len() >= MAX {
            return Ok(());
        }

        if self.cursor < self.buffer.len() {
            self.buffer.push(0).ok();
            for i in (self.cursor..self.buffer.len() - 1).rev() {
                self.buffer[i + 1] = self.buffer[i];
            }
        } else {
            self.buffer.push(c as u8).ok();
        }
        self.buffer[self.cursor] = c as u8;
        self.cursor += 1;

        self.write_all(&[c as u8]).await?;
        let tail: Vec<u8, MAX> = self.buffer[self.cursor..].iter().copied().collect();
        self.write_all(&tail).await?;
        let tail_len = tail.len();
        if tail_len > 0 {
            for _ in 0..tail_len {
                self.move_cursor_left().await?;
            }
        }
        Ok(())
    }

    async fn delete_char(&mut self) -> Result<(), KeyboardError<R::Error>> {
        if self.cursor == 0 {
            return Ok(());
        }
        self.cursor -= 1;
        self.buffer.remove(self.cursor);

        self.move_cursor_left().await?;
        self.erase_to_end().await?;
        let tail: Vec<u8, MAX> = self.buffer[self.cursor..].iter().copied().collect();
        self.write_all(&tail).await?;
        let tail_len = tail.len();
        if tail_len > 0 {
            for _ in 0..tail_len {
                self.move_cursor_left().await?;
            }
        }
        Ok(())
    }

    async fn load_history_entry(&mut self) -> Result<(), KeyboardError<R::Error>> {
        let entry_bytes: Vec<u8, MAX> = if let Some(idx) = self.history_index {
            if let Some(entry) = self.history.get(idx) {
                entry.as_bytes().iter().copied().collect()
            } else {
                return self.clear_line_and_buffer().await;
            }
        } else {
            return self.clear_line_and_buffer().await;
        };

        self.move_cursor_left_n(self.cursor).await?;
        self.erase_to_end().await?;
        self.buffer.clear();
        self.buffer.extend_from_slice(&entry_bytes).ok();
        self.cursor = self.buffer.len();
        let line: Vec<u8, MAX> = self.buffer.iter().copied().collect();
        self.write_all(&line).await?;
        Ok(())
    }

    async fn clear_line_and_buffer(&mut self) -> Result<(), KeyboardError<R::Error>> {
        self.move_cursor_left_n(self.cursor).await?;
        self.erase_to_end().await?;
        self.buffer.clear();
        self.cursor = 0;
        Ok(())
    }

    async fn move_cursor_left_n(&mut self, n: usize) -> Result<(), KeyboardError<R::Error>> {
        if n == 0 {
            return Ok(());
        }
        let mut buf = Vec::<u8, 16>::new();
        write!(buf, "\x1B[{}D", n).ok();
        self.write_all(&buf).await
    }
}
