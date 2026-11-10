//! Embedded terminal: one pseudo-terminal per open terminal tab. Output is streamed to the
//! frontend as `pty-data` events; input, resizes and closes come in as commands.
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Default)]
pub struct Terminals {
    next: Mutex<u32>,
    sessions: Mutex<HashMap<u32, Session>>,
}

#[derive(Serialize, Clone)]
struct DataEvent {
    id: u32,
    data: String,
}

#[derive(Serialize, Clone)]
struct ExitEvent {
    id: u32,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// Length of the longest prefix of `buf` that is complete UTF-8 (an incomplete character at
/// the end is held back until its remaining bytes arrive).
fn complete_len(buf: &[u8]) -> usize {
    match std::str::from_utf8(buf) {
        Ok(_) => buf.len(),
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        Err(_) => buf.len(), // genuinely invalid bytes: let the lossy conversion replace them
    }
}

impl Terminals {
    pub fn open(
        &self,
        app: AppHandle,
        dir: &Path,
        shell: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<u32, String> {
        let pair = native_pty_system()
            .openpty(size(cols, rows))
            .map_err(|e| e.to_string())?;
        let program = shell
            .map(str::to_owned)
            .or_else(|| std::env::var("SHELL").ok())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "/bin/sh".to_owned());
        let mut cmd = CommandBuilder::new(program);
        cmd.cwd(dir);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Could not start the shell: {e}"))?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let id = {
            let mut n = self.next.lock().map_err(|_| "state poisoned")?;
            *n += 1;
            *n
        };
        self.sessions.lock().map_err(|_| "state poisoned")?.insert(
            id,
            Session {
                master: pair.master,
                writer,
                child,
            },
        );

        std::thread::spawn(move || {
            let mut pending: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 8192];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        pending.extend_from_slice(&chunk[..n]);
                        let upto = complete_len(&pending);
                        if upto == 0 {
                            continue;
                        }
                        let data = String::from_utf8_lossy(&pending[..upto]).into_owned();
                        pending.drain(..upto);
                        let _ = app.emit("pty-data", DataEvent { id, data });
                    }
                }
            }
            let _ = app.emit("pty-exit", ExitEvent { id });
        });
        Ok(id)
    }

    pub fn write(&self, id: u32, data: &str) -> Result<(), String> {
        let mut s = self.sessions.lock().map_err(|_| "state poisoned")?;
        let session = s.get_mut(&id).ok_or("The terminal is closed.")?;
        session
            .writer
            .write_all(data.as_bytes())
            .and_then(|_| session.writer.flush())
            .map_err(|e| e.to_string())
    }

    pub fn resize(&self, id: u32, cols: u16, rows: u16) -> Result<(), String> {
        let s = self.sessions.lock().map_err(|_| "state poisoned")?;
        match s.get(&id) {
            Some(session) => session
                .master
                .resize(size(cols, rows))
                .map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn close(&self, id: u32) {
        if let Ok(mut s) = self.sessions.lock() {
            if let Some(mut session) = s.remove(&id) {
                let _ = session.child.kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_back_a_split_character() {
        let bytes = "é".as_bytes(); // 2 bytes
        assert_eq!(complete_len(&bytes[..1]), 0);
        assert_eq!(complete_len(bytes), 2);
        let mixed = [b'a', 0xE2, 0x82]; // 'a' + first 2 bytes of '€'
        assert_eq!(complete_len(&mixed), 1);
    }

    #[test]
    fn invalid_bytes_are_not_held_forever() {
        assert_eq!(complete_len(&[0xFF, b'a']), 2);
    }
}
