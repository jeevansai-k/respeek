/* icons.rs — resolve app icons to local files the UI can load.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::collections::HashMap;
use std::path::PathBuf;

use crate::magpie::types::apps::icon::Icon as IconKind;

#[derive(Debug, Default)]
pub struct IconCache {
    /// app id → resolved path (`None` = looked up, nothing found).
    resolved: HashMap<String, Option<String>>,
    theme: Option<String>,
}

fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("respeek").join("icons")
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect()
}

/// Theme lookup for a named icon; prefers scalable/large raster icons.
pub fn lookup_named(name: &str, theme: Option<&str>) -> Option<String> {
    if name.starts_with('/') {
        return std::path::Path::new(name).exists().then(|| name.to_owned());
    }
    let mut q = freedesktop_icons::lookup(name).with_size(128).with_cache();
    if let Some(t) = theme {
        q = q.with_theme(t);
    }
    q.find()
        .or_else(|| freedesktop_icons::lookup(name).with_size(64).find())
        .or_else(|| freedesktop_icons::lookup(name).find())
        .map(|p| p.to_string_lossy().into_owned())
}

impl IconCache {
    pub fn new() -> Self {
        Self { resolved: HashMap::new(), theme: freedesktop_icons::default_theme_gtk() }
    }

    pub fn get(&self, app_id: &str) -> Option<String> {
        self.resolved.get(app_id).cloned().flatten()
    }

    /// App ids that still need a Magpie icon request.
    pub fn missing<'a>(&self, ids: impl Iterator<Item = &'a String>) -> Vec<String> {
        ids.filter(|id| !self.resolved.contains_key(*id)).cloned().collect()
    }

    pub fn insert(&mut self, icons: HashMap<String, IconKind>, requested: &[String]) {
        for id in requested {
            let path = match icons.get(id) {
                Some(IconKind::Id(name)) => lookup_named(name, self.theme.as_deref()),
                Some(IconKind::Data(bytes)) => {
                    let dir = cache_dir();
                    let ext = if bytes.starts_with(b"<") || bytes.starts_with(b"<?xml") {
                        "svg"
                    } else {
                        "png"
                    };
                    let path = dir.join(format!("{}.{ext}", sanitize(id)));
                    std::fs::create_dir_all(&dir)
                        .and_then(|_| std::fs::write(&path, bytes))
                        .ok()
                        .map(|_| path.to_string_lossy().into_owned())
                }
                _ => None,
            };
            self.resolved.insert(id.clone(), path);
        }
    }
}
