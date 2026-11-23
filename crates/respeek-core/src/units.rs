/* units.rs — every human-readable string the UI shows is produced here.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

/// How a byte quantity should be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteStyle {
    /// `true` = bytes (B, KiB…), `false` = bits (b, Kib…).
    pub bytes: bool,
    /// `true` = powers of 1024 (KiB), `false` = powers of 1000 (kB).
    pub base2: bool,
}

impl ByteStyle {
    pub const MEMORY: Self = Self { bytes: true, base2: true };
    pub const BYTES_SI: Self = Self { bytes: true, base2: false };
}

const BASE2_BYTES: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
const BASE10_BYTES: [&str; 7] = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];
const BASE2_BITS: [&str; 7] = ["b", "Kib", "Mib", "Gib", "Tib", "Pib", "Eib"];
const BASE10_BITS: [&str; 7] = ["b", "kb", "Mb", "Gb", "Tb", "Pb", "Eb"];

/// Split a value into (scaled number, unit) in the requested style.
pub fn scale(value_bytes: f64, style: ByteStyle) -> (f64, &'static str) {
    let mut v = if style.bytes { value_bytes } else { value_bytes * 8.0 };
    let div = if style.base2 { 1024.0 } else { 1000.0 };
    let units = match (style.bytes, style.base2) {
        (true, true) => &BASE2_BYTES,
        (true, false) => &BASE10_BYTES,
        (false, true) => &BASE2_BITS,
        (false, false) => &BASE10_BITS,
    };
    let mut i = 0;
    while v.abs() >= div && i < units.len() - 1 {
        v /= div;
        i += 1;
    }
    (v, units[i])
}

fn trim_number(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let digits = if v.abs() >= 100.0 {
        0
    } else if v.abs() >= 10.0 {
        1
    } else {
        2
    };
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

pub fn bytes(value: f64, style: ByteStyle) -> String {
    let (v, u) = scale(value, style);
    format!("{} {u}", trim_number(v))
}

pub fn rate(value_per_sec: f64, style: ByteStyle) -> String {
    format!("{}/s", bytes(value_per_sec, style))
}

pub fn percent(v: f32) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    if v > 0.0 && v < 0.1 {
        return "<0.1%".into();
    }
    format!("{v:.1}%").replace(".0%", "%")
}

pub fn percent_int(v: f32) -> String {
    format!("{}%", v.round().clamp(0.0, 100.0) as i32)
}

pub fn mhz(v: u64) -> String {
    if v >= 1000 {
        format!("{:.2} GHz", v as f64 / 1000.0)
    } else {
        format!("{v} MHz")
    }
}

pub fn khz(v: u64) -> String {
    mhz(v / 1000)
}

pub fn celsius(v: f32) -> String {
    format!("{v:.0} °C")
}

pub fn milli_kelvin(v: u32) -> String {
    celsius(v as f32 / 1000.0 - 273.15)
}

pub fn watts(v: f32) -> String {
    if v >= 100.0 {
        format!("{v:.0} W")
    } else {
        format!("{v:.1} W")
    }
}

pub fn count(v: u64) -> String {
    let s = v.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// "3:04:05" style clock duration (used for uptime and battery time).
pub fn uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let h = (seconds % 86_400) / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if days > 0 {
        format!("{days}d {h:02}:{m:02}:{s:02}")
    } else {
        format!("{h}:{m:02}:{s:02}")
    }
}

/// "2 h 15 min" style duration.
pub fn duration_words(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    match (h, m) {
        (0, 0) => format!("{seconds} s"),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

/// Port of upstream `to_long_human_readable_time` (used for SMART power-on time).
pub fn long_duration(seconds: u64) -> String {
    const YEAR: u64 = 60 * 60 * 6 * 1461;
    const MONTH: u64 = 60 * 30 * 1461;
    const DAY: u64 = 86_400;
    const HOUR: u64 = 3600;
    let mut parts = vec![];
    let mut t = seconds;
    for (unit, one, many) in [
        (YEAR, "year", "years"),
        (MONTH, "month", "months"),
        (DAY, "day", "days"),
        (HOUR, "hour", "hours"),
        (60, "minute", "minutes"),
    ] {
        let n = t / unit;
        if n > 0 {
            parts.push(format!("{n} {}", if n == 1 { one } else { many }));
            t %= unit;
        }
        if parts.len() == 2 {
            break;
        }
    }
    if parts.is_empty() {
        format!("{seconds} seconds")
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes() {
        assert_eq!(bytes(0.0, ByteStyle::MEMORY), "0 B");
        assert_eq!(bytes(1536.0, ByteStyle::MEMORY), "1.5 KiB");
        assert_eq!(bytes(16.0 * 1024.0 * 1024.0 * 1024.0, ByteStyle::MEMORY), "16 GiB");
        assert_eq!(bytes(1_250_000.0, ByteStyle::BYTES_SI), "1.25 MB");
        assert_eq!(
            rate(125_000.0, ByteStyle { bytes: false, base2: false }),
            "1 Mb/s"
        );
    }

    #[test]
    fn formats_misc() {
        assert_eq!(percent(12.0), "12%");
        assert_eq!(percent(12.34), "12.3%");
        assert_eq!(percent(0.04), "<0.1%");
        assert_eq!(mhz(3400), "3.40 GHz");
        assert_eq!(count(1234567), "1,234,567");
        assert_eq!(uptime(3_725), "1:02:05");
        assert_eq!(uptime(90_061), "1d 01:01:01");
        assert_eq!(duration_words(8100), "2 h 15 min");
    }
}
