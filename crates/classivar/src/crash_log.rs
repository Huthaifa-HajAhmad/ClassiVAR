//! A panic closes the window at once, so its message and backtrace are also
//! written to `crash.log` in the program's settings folder (the newest last):
//! `%APPDATA%\\ClassiVAR` on Windows, `~/Library/Application Support/ClassiVAR` on
//! macOS. The file holds the program's own message only, never the case on screen,
//! and starts again from empty once it passes `MAX_BYTES`.

use std::io::Write;
use std::path::PathBuf;

const MAX_BYTES: u64 = 256 * 1024;

fn folder() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(base).join("ClassiVAR"));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library").join("Application Support").join("ClassiVAR"))
}

pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!(
            "--- ClassiVAR {} panic ---\n{info}\n{}\n",
            env!("CARGO_PKG_VERSION"),
            std::backtrace::Backtrace::force_capture()
        );
        if let Some(dir) = folder() {
            let path = dir.join("crash.log");
            let too_big = std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES);
            let opened = std::fs::create_dir_all(&dir).and_then(|_| {
                std::fs::OpenOptions::new().create(true).append(!too_big).write(true).truncate(too_big).open(&path)
            });
            if let Ok(mut file) = opened {
                let _ = file.write_all(text.as_bytes());
            }
        }
        default(info);
    }));
}
