/* model.rs — serde view-models sent to the UI. The UI renders these verbatim.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::Serialize;

use crate::settings::Settings;

/// Semantic color role; the design system maps it to a palette color.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tint {
    Cpu,
    Memory,
    Swap,
    Disk,
    DiskWrite,
    Network,
    NetworkSend,
    Gpu,
    GpuMemory,
    Encoder,
    Decoder,
    Fan,
    Battery,
    Kernel,
    Neutral,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Cpu,
    Memory,
    Disk,
    Network,
    Gpu,
    Fan,
    Battery,
}

#[derive(Debug, Clone, Serialize)]
pub struct Series {
    pub label: String,
    pub tint: Tint,
    /// Oldest → newest, already normalized to 0..=1 against `max`.
    pub values: Vec<f32>,
    pub dashed: bool,
    /// Formatted latest value ("12%", "1.2 MB/s").
    pub current: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Graph {
    pub id: String,
    pub title: String,
    /// Formatted top-of-scale label ("100%", "8 MB/s").
    pub max_label: String,
    pub series: Vec<Series>,
    /// "hero" (large), "half" (side-by-side), "mini" (per-core grid cell).
    pub size: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stat {
    pub label: String,
    pub value: String,
    /// Primary stats render large with a ticker animation.
    pub primary: bool,
    pub tint: Option<Tint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatGroup {
    pub title: Option<String>,
    pub stats: Vec<Stat>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub label: String,
    pub value: String,
    /// Fraction of the whole bar, 0..=1.
    pub fraction: f32,
    pub tint: Tint,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageBar {
    pub title: String,
    pub subtitle: String,
    pub fraction: f32,
    pub tint: Tint,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Extra {
    /// Memory composition bar (In use / Modified / Standby / Free).
    Composition { title: String, segments: Vec<Segment> },
    /// Partition / mount usage list (disks).
    Partitions { items: Vec<UsageBar> },
    /// Key-value details shown behind an "i" popover (memory slots, GPU APIs…).
    Details { title: String, groups: Vec<StatGroup> },
    /// Actions available for this device.
    Actions { smart: bool, eject: bool },
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub id: String,
    pub kind: DeviceKind,
    pub title: String,
    pub subtitle: String,
    /// Headline value 0..=1 (drives the mini gauge).
    pub level: f32,
    /// `level` formatted for display ("42%").
    pub level_label: String,
    pub tint: Tint,
    /// Mini sparkline, normalized 0..=1.
    pub spark: Vec<f32>,
    /// Optional second sparkline (dashed): write / send / kernel.
    pub spark2: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceDetail {
    pub id: String,
    pub kind: DeviceKind,
    pub title: String,
    pub model: String,
    pub graphs: Vec<Graph>,
    /// Per-core mini graphs (CPU in per-core mode).
    pub grid: Vec<Graph>,
    pub stats: Vec<StatGroup>,
    pub extras: Vec<Extra>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Column {
    pub id: &'static str,
    pub title: String,
    /// Aggregate shown under the header ("34%" total CPU).
    pub total: Option<String>,
    pub numeric: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    Section,
    App,
    Process,
    Service,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub text: String,
    /// 0..=1 load used to tint the cell like a heat-map; `None` = no tint.
    pub heat: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    /// Stable key across ticks ("app:firefox", "pid:1234", "section:apps").
    pub key: String,
    pub kind: RowKind,
    pub depth: u8,
    pub name: String,
    pub subtitle: Option<String>,
    /// Local filesystem path of the icon (UI converts it to an asset URL).
    pub icon: Option<String>,
    /// Fallback glyph name when no icon is available.
    pub glyph: &'static str,
    pub pid: Option<u32>,
    pub expandable: bool,
    pub expanded: bool,
    /// Status badge for services/processes ("running", "failed", "stopped", "suspended"…).
    pub status: Option<&'static str>,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Selection {
    pub key: String,
    pub title: String,
    /// PIDs an action applies to.
    pub pids: Vec<u32>,
    pub service_id: Option<u64>,
    pub is_app: bool,
    pub service_running: bool,
    pub service_enabled: bool,
    pub suspended: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableView {
    pub headline: String,
    pub subheadline: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub sort_column: String,
    pub sort_ascending: bool,
    pub selection: Option<Selection>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "page", rename_all = "snake_case")]
pub enum PageData {
    Performance { detail: Option<DeviceDetail>, overview: bool },
    Apps { table: TableView },
    Services { table: TableView, user: bool },
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub seq: u64,
    pub interval_ms: u64,
    pub sidebar: Vec<DeviceSummary>,
    pub page: PageData,
    pub settings: Settings,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyValue {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessDetails {
    pub pid: u32,
    pub name: String,
    pub icon: Option<String>,
    pub state: String,
    pub groups: Vec<StatGroup>,
    pub command_line: String,
    pub exe: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceDetails {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub groups: Vec<StatGroup>,
    pub logs: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmartView {
    pub title: String,
    pub available: bool,
    pub groups: Vec<StatGroup>,
    pub attributes_header: Vec<String>,
    pub attributes: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AboutSystem {
    pub logo: Option<String>,
    pub title: String,
    pub groups: Vec<StatGroup>,
}

pub fn stat(label: impl Into<String>, value: impl Into<String>) -> Stat {
    Stat { label: label.into(), value: value.into(), primary: false, tint: None }
}

pub fn primary(label: impl Into<String>, value: impl Into<String>, tint: Tint) -> Stat {
    Stat { label: label.into(), value: value.into(), primary: true, tint: Some(tint) }
}

pub fn group(title: Option<&str>, stats: Vec<Stat>) -> StatGroup {
    StatGroup { title: title.map(str::to_owned), stats }
}
