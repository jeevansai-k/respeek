/* magpie/shim.rs
 *
 * Copyright 2026 Respeek contributors
 * Derived from Mission Center (Copyright 2026 Mission Center Developers)
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Replaces the GLib/GTK and Flatpak hooks that the upstream Magpie client
 * depends on, so the client can run in a UI-agnostic Rust core.
 */

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

macro_rules! g_critical {
    ($domain:expr, $($arg:tt)*) => { ::log::error!(target: $domain, $($arg)*) };
}
#[allow(unused_macros)]
macro_rules! g_warning {
    ($domain:expr, $($arg:tt)*) => { ::log::warn!(target: $domain, $($arg)*) };
}
macro_rules! g_debug {
    ($domain:expr, $($arg:tt)*) => { ::log::debug!(target: $domain, $($arg)*) };
}
macro_rules! cmd_flatpak_host {
    ($cmd: expr) => {{
        let mut cmd = std::process::Command::new("/usr/bin/flatpak-spawn");
        cmd.arg("--host").arg("sh").arg("-c");
        cmd.arg($cmd);
        cmd
    }};
}

#[allow(unused_imports)]
#[allow(unused_macros)]
pub(crate) use {cmd_flatpak_host, g_critical, g_debug, g_warning};

/// Env var to override the Magpie binary location.
pub const ENV_MAGPIE_PATH: &str = "RESPEEK_MAGPIE_PATH";

/// Respeek is shipped natively (deb/AppImage), never as a Flatpak.
pub fn is_flatpak() -> bool {
    false
}

pub fn flatpak_app_path() -> &'static str {
    ""
}

pub fn flatpak_data_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| std::env::temp_dir()).as_path()
}

/// Locate the Magpie gatherer:
/// 1. `$RESPEEK_MAGPIE_PATH`
/// 2. `respeek-magpie` next to the running executable (Tauri sidecar layout)
/// 3. `respeek-magpie` / `missioncenter-magpie` on `$PATH`
pub fn magpie_executable() -> String {
    if let Ok(p) = std::env::var(ENV_MAGPIE_PATH) {
        return p;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["respeek-magpie", "missioncenter-magpie"] {
                let candidate = dir.join(name);
                if candidate.exists() {
                    return candidate.to_string_lossy().into_owned();
                }
            }
        }
    }

    for name in ["respeek-magpie", "missioncenter-magpie"] {
        if let Ok(path) = which(name) {
            return path;
        }
    }

    "respeek-magpie".to_owned()
}

fn which(name: &str) -> Result<String, ()> {
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    Err(())
}

type FatalHandler = Box<dyn Fn(&str) + Send + Sync>;
static FATAL_HANDLER: OnceLock<FatalHandler> = OnceLock::new();

/// Register a callback the shell uses to show a fatal error before exit.
pub fn set_fatal_handler(handler: impl Fn(&str) + Send + Sync + 'static) {
    let _ = FATAL_HANDLER.set(Box::new(handler));
}

pub fn show_error_dialog_and_exit(message: &str) -> ! {
    log::error!(target: "Respeek::Gatherer", "Fatal: {message}");
    if let Some(handler) = FATAL_HANDLER.get() {
        handler(message);
    }
    std::process::exit(1);
}
