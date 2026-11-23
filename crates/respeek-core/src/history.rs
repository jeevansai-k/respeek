/* history.rs — fixed-capacity ring buffers for graph series.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Default)]
pub struct Ring {
    data: VecDeque<f32>,
}

impl Ring {
    pub fn push(&mut self, v: f32, cap: usize) {
        self.data.push_back(if v.is_finite() { v } else { 0.0 });
        while self.data.len() > cap {
            self.data.pop_front();
        }
    }

    /// Values oldest → newest, left-padded with zeros to `cap` so graphs fill from the right.
    pub fn padded(&self, cap: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(cap);
        let missing = cap.saturating_sub(self.data.len());
        out.extend(std::iter::repeat_n(0.0, missing));
        out.extend(self.data.iter().skip(self.data.len().saturating_sub(cap)));
        out
    }

    pub fn max(&self) -> f32 {
        self.data.iter().copied().fold(0.0, f32::max)
    }

    pub fn last(&self) -> f32 {
        self.data.back().copied().unwrap_or(0.0)
    }
}

/// All series for all devices keyed by "<device>/<series>".
#[derive(Debug, Default)]
pub struct Histories {
    rings: HashMap<String, Ring>,
    touched: std::collections::HashSet<String>,
    pub cap: usize,
}

impl Histories {
    pub fn new(cap: usize) -> Self {
        Self { cap, ..Default::default() }
    }

    pub fn push(&mut self, key: &str, v: f32) {
        let cap = self.cap;
        self.touched.insert(key.to_owned());
        self.rings.entry(key.to_owned()).or_default().push(v, cap);
    }

    pub fn get(&self, key: &str) -> Option<&Ring> {
        self.rings.get(key)
    }

    pub fn values(&self, key: &str) -> Vec<f32> {
        self.rings
            .get(key)
            .map(|r| r.padded(self.cap))
            .unwrap_or_else(|| vec![0.0; self.cap])
    }

    pub fn max(&self, key: &str) -> f32 {
        self.rings.get(key).map(Ring::max).unwrap_or(0.0)
    }

    /// Drop series of devices that disappeared (not pushed since the last call).
    pub fn end_tick(&mut self) {
        let touched = std::mem::take(&mut self.touched);
        self.rings.retain(|k, _| touched.contains(k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_pads_and_caps() {
        let mut r = Ring::default();
        for i in 0..5 {
            r.push(i as f32, 3);
        }
        assert_eq!(r.padded(3), vec![2.0, 3.0, 4.0]);
        assert_eq!(r.padded(5), vec![0.0, 0.0, 2.0, 3.0, 4.0]);
        assert_eq!(r.max(), 4.0);
    }

    #[test]
    fn drops_stale_series() {
        let mut h = Histories::new(4);
        h.push("a", 1.0);
        h.push("b", 1.0);
        h.end_tick();
        h.push("a", 2.0);
        h.end_tick();
        assert!(h.get("a").is_some());
        assert!(h.get("b").is_none());
    }
}
