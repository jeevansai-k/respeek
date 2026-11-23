/* settings.rs — persisted preferences (mirrors Mission Center's GSettings schema).
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::units::ByteStyle;

/// Seconds per update step; upstream `INTERVAL_STEP`.
pub const INTERVAL_STEP: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    // Window
    pub window_width: i32,
    pub window_height: i32,
    pub is_maximized: bool,
    pub selected_page: String,
    pub appearance: Appearance,
    pub accent: String,
    pub reduce_transparency: bool,

    // General
    /// Update interval in steps of `INTERVAL_STEP` seconds (10..=200, default 20 = 1 s).
    pub update_interval: u64,
    /// Number of graph data points kept (10..=600).
    pub data_points: u32,

    // Units
    pub memory_use_bytes: bool,
    pub memory_use_base2: bool,
    pub drive_use_bytes: bool,
    pub drive_use_base2: bool,
    pub network_use_bytes: bool,
    pub network_use_base2: bool,
    pub network_dynamic_scaling: bool,

    // Apps & Services
    pub merged_process_stats: bool,
    pub remember_sorting: bool,
    pub remember_column_order: bool,
    pub core_count_affects_percentages: bool,
    pub show_column_separators: bool,
    pub hide_is_zero: bool,
    pub tree_view: bool,
    pub apps_sort_column: String,
    pub apps_sort_direction: SortDirection,
    pub apps_column_order: Vec<String>,
    pub apps_hidden_columns: Vec<String>,
    pub services_sort_column: String,
    pub services_sort_direction: SortDirection,
    pub services_column_order: Vec<String>,
    pub services_hidden_columns: Vec<String>,

    // Performance
    /// 1 = overall, 2 = per core (upstream `performance-page-cpu-graph`).
    pub cpu_graph: u8,
    pub kernel_times: bool,
    pub selected_device: String,
    pub smooth_graphs: bool,
    pub sliding_graphs: bool,
    pub show_cpu: bool,
    pub show_memory: bool,
    pub show_disks: bool,
    pub show_network: bool,
    pub show_network_wired: bool,
    pub show_network_wireless: bool,
    pub show_network_vpn: bool,
    pub show_network_virtual: bool,
    pub show_network_other: bool,
    pub show_gpus: bool,
    pub show_fans: bool,
    pub show_batteries: bool,
    pub memory_composition_visible: bool,
    pub memory_swap_visible: bool,
    pub gpu_graph_middle: u8,
    pub gpu_graph_bottom: u8,
    pub sidebar_order: Vec<String>,
    pub sidebar_hidden_graphs: Vec<String>,

    pub first_time_running: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_width: 1180,
            window_height: 760,
            is_maximized: false,
            selected_page: "performance".into(),
            appearance: Appearance::System,
            accent: "blue".into(),
            reduce_transparency: false,

            update_interval: 20,
            data_points: 60,

            memory_use_bytes: true,
            memory_use_base2: true,
            drive_use_bytes: true,
            drive_use_base2: true,
            network_use_bytes: false,
            network_use_base2: false,
            network_dynamic_scaling: true,

            merged_process_stats: false,
            remember_sorting: false,
            remember_column_order: false,
            core_count_affects_percentages: false,
            show_column_separators: false,
            hide_is_zero: false,
            tree_view: true,
            apps_sort_column: "cpu".into(),
            apps_sort_direction: SortDirection::Descending,
            apps_column_order: vec![],
            apps_hidden_columns: vec!["swap".into(), "gpu_memory".into()],
            services_sort_column: "name".into(),
            services_sort_direction: SortDirection::Ascending,
            services_column_order: vec![],
            services_hidden_columns: vec![],

            cpu_graph: 1,
            kernel_times: false,
            selected_device: "cpu".into(),
            smooth_graphs: true,
            sliding_graphs: true,
            show_cpu: true,
            show_memory: true,
            show_disks: true,
            show_network: true,
            show_network_wired: true,
            show_network_wireless: true,
            show_network_vpn: true,
            show_network_virtual: true,
            show_network_other: true,
            show_gpus: true,
            show_fans: true,
            show_batteries: true,
            memory_composition_visible: true,
            memory_swap_visible: true,
            gpu_graph_middle: 1,
            gpu_graph_bottom: 2,
            sidebar_order: vec![],
            sidebar_hidden_graphs: vec![],

            first_time_running: true,
        }
    }
}

impl Settings {
    pub fn interval_secs(&self) -> f64 {
        self.update_interval.clamp(10, 200) as f64 * INTERVAL_STEP
    }

    pub fn memory_style(&self) -> ByteStyle {
        ByteStyle { bytes: self.memory_use_bytes, base2: self.memory_use_base2 }
    }

    pub fn drive_style(&self) -> ByteStyle {
        ByteStyle { bytes: self.drive_use_bytes, base2: self.drive_use_base2 }
    }

    pub fn network_style(&self) -> ByteStyle {
        ByteStyle { bytes: self.network_use_bytes, base2: self.network_use_base2 }
    }

    /// Clamp values the UI could have pushed outside their valid ranges.
    pub fn sanitize(&mut self) {
        self.update_interval = self.update_interval.clamp(10, 200);
        self.data_points = self.data_points.clamp(10, 600);
        self.cpu_graph = self.cpu_graph.clamp(1, 2);
        self.gpu_graph_middle = self.gpu_graph_middle.min(5);
        self.gpu_graph_bottom = self.gpu_graph_bottom.min(5);
        self.window_width = self.window_width.max(720);
        self.window_height = self.window_height.max(480);
    }

    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("respeek").join("settings.toml")
    }

    pub fn load() -> Self {
        let mut s: Settings = std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default();
        s.sanitize();
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
    }

    /// Apply a partial JSON patch coming from the UI (`{"tree_view": false}`).
    pub fn apply_patch(&mut self, patch: serde_json::Value) -> Result<(), String> {
        let mut current = serde_json::to_value(&*self).map_err(|e| e.to_string())?;
        let (Some(obj), serde_json::Value::Object(p)) = (current.as_object_mut(), patch) else {
            return Err("settings patch must be an object".into());
        };
        for (k, v) in p {
            if !obj.contains_key(&k) {
                return Err(format!("unknown setting `{k}`"));
            }
            obj.insert(k, v);
        }
        let mut next: Settings = serde_json::from_value(current).map_err(|e| e.to_string())?;
        next.sanitize();
        *self = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_patches() {
        let s = Settings::default();
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(s, back);

        let mut s = Settings::default();
        s.apply_patch(serde_json::json!({"update_interval": 5000, "tree_view": false}))
            .unwrap();
        assert_eq!(s.update_interval, 200);
        assert!(!s.tree_view);
        assert!(s.apply_patch(serde_json::json!({"nope": 1})).is_err());
        assert!((Settings::default().interval_secs() - 1.0).abs() < 1e-9);
    }
}
