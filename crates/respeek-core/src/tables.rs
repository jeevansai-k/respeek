/* tables.rs — Apps and Services tables: tree building, search, sort, flatten, selection.
 *
 * Port of Mission Center's table_view/models.rs + apps_page logic.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::devices::mem_used;
use crate::icons::IconCache;
use crate::magpie::types::processes::{ProcessState, ProcessUsageStats};
use crate::magpie::{App, Process, Service};
use crate::model::*;
use crate::readings::Readings;
use crate::settings::{Settings, SortDirection};
use crate::units::{self, ByteStyle};

#[derive(Debug, Default, Clone)]
pub struct TableState {
    /// Keys of rows the user expanded (apps, processes, services).
    pub expanded: HashSet<String>,
    /// Keys of default-expanded rows the user collapsed (sections).
    pub collapsed: HashSet<String>,
    pub search: String,
    pub selected: Option<String>,
}

impl TableState {
    pub fn toggle(&mut self, key: &str, default_expanded: bool) {
        let set = if default_expanded { &mut self.collapsed } else { &mut self.expanded };
        if !set.remove(key) {
            set.insert(key.to_owned());
        }
    }
}

pub const APP_COLUMNS: &[(&str, &str, bool)] = &[
    ("name", "Name", false),
    ("pid", "PID", true),
    ("cpu", "CPU", true),
    ("memory", "Memory", true),
    ("swap", "Swap", true),
    ("drive", "Drive", true),
    ("network", "Network", true),
    ("gpu", "GPU", true),
    ("gpu_memory", "GPU Mem", true),
];

pub const SERVICE_COLUMNS: &[(&str, &str, bool)] = &[
    ("name", "Name", false),
    ("status", "Status", false),
    ("startup", "Startup", false),
    ("pid", "PID", true),
    ("user", "User", false),
    ("group", "Group", false),
    ("description", "Description", false),
];

// ----- helpers ported from upstream ------------------------------------------------------------

pub fn pretty_process_name(process: &Process) -> String {
    if process.exe.is_empty() {
        if let Some(cmd) = process.cmd.first() {
            let mut cmd = cmd
                .split_ascii_whitespace()
                .next()
                .and_then(|s| s.split('/').next_back())
                .unwrap_or(&process.name);
            if let Some(s) = cmd.strip_suffix(':') {
                cmd = s;
            }
            cmd.trim().to_owned()
        } else {
            process.name.trim().to_owned()
        }
    } else {
        let exe_name = process.exe.split('/').next_back().unwrap_or(&process.name);
        if exe_name.starts_with("wine") {
            if process.cmd.is_empty() {
                process.name.trim().to_owned()
            } else {
                process.cmd[0]
                    .split('\\')
                    .next_back()
                    .unwrap_or(&process.name)
                    .split('/')
                    .next_back()
                    .unwrap_or(&process.name)
                    .trim()
                    .to_owned()
            }
        } else {
            exe_name.trim().to_owned()
        }
    }
}

pub fn primary_processes(app: &App, process_map: &HashMap<u32, Process>) -> Vec<u32> {
    let mut secondary = HashSet::new();
    for app_pid in &app.pids {
        if let Some(process) = process_map.get(app_pid) {
            for child in &process.children {
                if app.pids.contains(child) {
                    secondary.insert(*child);
                }
            }
        }
    }

    let mut primary: Vec<u32> =
        app.pids.iter().copied().filter(|p| !secondary.contains(p)).collect();

    if primary.is_empty() {
        for (index, pid) in app.pids.iter().enumerate() {
            if let Some(process) = process_map.get(pid) {
                if !process.children.is_empty() || index == app.pids.len() - 1 {
                    primary.push(*pid);
                    break;
                }
            }
        }
    }
    primary.retain(|p| process_map.contains_key(p));
    primary
}

fn find_stoppable_process(process: &Process, process_map: &HashMap<u32, Process>) -> Option<u32> {
    if process.name != "bwrap" {
        return Some(process.pid);
    }
    process
        .children
        .iter()
        .filter_map(|c| process_map.get(c))
        .find_map(|p| find_stoppable_process(p, process_map))
}

pub fn stoppable_pids(primary: &[u32], process_map: &HashMap<u32, Process>) -> Vec<u32> {
    primary
        .iter()
        .filter_map(|pid| process_map.get(pid).and_then(|p| find_stoppable_process(p, process_map)))
        .collect()
}

pub fn collect_process_subtree(
    process_map: &HashMap<u32, Process>,
    pid: u32,
    pids: &mut HashSet<u32>,
) {
    let mut stack = vec![pid];
    while let Some(pid) = stack.pop() {
        if !pids.insert(pid) {
            continue;
        }
        if let Some(p) = process_map.get(&pid) {
            stack.extend(p.children.iter().copied());
        }
    }
}

/// Every pid that belongs to an app inherits the app's icon (upstream `propagate_app_icons`).
fn icons_by_pid(r: &Readings, icons: &IconCache) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    for app in r.apps.values() {
        let Some(icon) = icons.get(&app.id) else { continue };
        let mut pids = HashSet::new();
        for pid in &app.pids {
            collect_process_subtree(&r.processes, *pid, &mut pids);
        }
        for pid in pids {
            out.entry(pid).or_insert_with(|| icon.clone());
        }
    }
    out
}

fn state_status(p: &Process) -> Option<&'static str> {
    match p.state() {
        ProcessState::Stopped | ProcessState::Tracing => Some("suspended"),
        ProcessState::Zombie | ProcessState::Dead => Some("zombie"),
        _ => None,
    }
}

// ----- generic tree --------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct Node {
    key: String,
    kind: Option<RowKind>,
    name: String,
    subtitle: Option<String>,
    search_text: String,
    icon: Option<String>,
    glyph: &'static str,
    pid: Option<u32>,
    status: Option<&'static str>,
    stats: ProcessUsageStats,
    /// Text values for non-numeric service columns.
    text: HashMap<&'static str, String>,
    children: Vec<Node>,
    default_expanded: bool,
}

fn process_node(
    pid: u32,
    parent_key: &str,
    r: &Readings,
    icons: &HashMap<u32, String>,
    merged: bool,
    recurse: bool,
    visited: &mut HashSet<u32>,
) -> Option<Node> {
    let p = r.processes.get(&pid)?;
    if !visited.insert(pid) {
        return None;
    }
    let key = format!("{parent_key}/{pid}");
    let children = if recurse {
        p.children
            .iter()
            .filter_map(|c| process_node(*c, &key, r, icons, merged, true, visited))
            .collect()
    } else {
        vec![]
    };
    let name = pretty_process_name(p);
    let cmd = p.cmd.join(" ");
    let mut text = HashMap::new();
    text.insert("status", format!("{:?}", p.state()));
    Some(Node {
        search_text: format!("{} {} {}", name.to_lowercase(), pid, cmd.to_lowercase()),
        subtitle: (!cmd.is_empty()).then_some(cmd),
        key,
        kind: Some(RowKind::Process),
        name,
        icon: icons.get(&pid).cloned(),
        glyph: "process",
        pid: Some(pid),
        status: state_status(p),
        stats: if merged { p.merged_usage_stats(&r.processes) } else { p.usage_stats.clone() },
        text,
        children,
        default_expanded: false,
    })
}

fn cmp_nodes(a: &Node, b: &Node, column: &str) -> Ordering {
    let f = |x: f32, y: f32| x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    let by_name = || a.name.to_lowercase().cmp(&b.name.to_lowercase());
    let ord = match column {
        "pid" => a.pid.unwrap_or(0).cmp(&b.pid.unwrap_or(0)),
        "cpu" => f(a.stats.cpu_usage, b.stats.cpu_usage),
        "memory" => a.stats.memory_usage.cmp(&b.stats.memory_usage),
        "swap" => a.stats.swap_usage.cmp(&b.stats.swap_usage),
        "drive" => f(a.stats.disk_usage, b.stats.disk_usage),
        "network" => f(a.stats.network_usage, b.stats.network_usage),
        "gpu" => f(a.stats.gpu_usage, b.stats.gpu_usage),
        "gpu_memory" => a.stats.gpu_memory_usage.cmp(&b.stats.gpu_memory_usage),
        "name" => by_name(),
        other => a
            .text
            .get(other)
            .map(|s| s.to_lowercase())
            .cmp(&b.text.get(other).map(|s| s.to_lowercase())),
    };
    ord.then_with(by_name).then_with(|| a.key.cmp(&b.key))
}

fn sort_tree(nodes: &mut [Node], column: &str, asc: bool) {
    nodes.sort_by(|a, b| {
        let o = cmp_nodes(a, b, column);
        if asc { o } else { o.reverse() }
    });
    for n in nodes {
        sort_tree(&mut n.children, column, asc);
    }
}

/// Keep nodes that match (with their whole subtree) or that have matching descendants.
/// Returns the keys that must be force-expanded to reveal matches.
fn filter_tree(nodes: &mut Vec<Node>, q: &str, reveal: &mut HashSet<String>) {
    nodes.retain_mut(|n| {
        if n.kind.is_some() && n.search_text.contains(q) {
            return true;
        }
        filter_tree(&mut n.children, q, reveal);
        if n.children.is_empty() {
            false
        } else {
            reveal.insert(n.key.clone());
            true
        }
    });
}

struct Flattener<'a> {
    state: &'a TableState,
    reveal: &'a HashSet<String>,
    columns: &'a [&'static str],
    settings: &'a Settings,
    total_mem: f64,
    rows: Vec<Row>,
}

impl Flattener<'_> {
    fn cell(&self, n: &Node, column: &str) -> Cell {
        let s = &n.stats;
        let hide0 = self.settings.hide_is_zero;
        let blank = |zero: bool, text: String| if hide0 && zero { String::new() } else { text };
        let is_section = n.kind == Some(RowKind::Section);
        if is_section {
            return Cell { text: String::new(), heat: None };
        }
        match column {
            "pid" => Cell { text: n.pid.map(|p| p.to_string()).unwrap_or_default(), heat: None },
            "cpu" => Cell {
                text: blank(s.cpu_usage < 0.05, units::percent(s.cpu_usage)),
                heat: Some((s.cpu_usage / 100.0).clamp(0.0, 1.0)),
            },
            "memory" => Cell {
                text: blank(
                    s.memory_usage == 0,
                    units::bytes(s.memory_usage as f64, self.settings.memory_style()),
                ),
                heat: Some((s.memory_usage as f64 / self.total_mem).clamp(0.0, 1.0) as f32 * 4.0)
                    .map(|h| h.min(1.0)),
            },
            "swap" => Cell {
                text: blank(
                    s.swap_usage == 0,
                    units::bytes(s.swap_usage as f64, self.settings.memory_style()),
                ),
                heat: None,
            },
            "drive" => Cell {
                text: blank(
                    s.disk_usage < 1.0,
                    units::rate(s.disk_usage as f64, self.settings.drive_style()),
                ),
                heat: Some((s.disk_usage / (50.0 * 1024.0 * 1024.0)).clamp(0.0, 1.0)),
            },
            "network" => Cell {
                text: blank(
                    s.network_usage < 1.0,
                    units::rate(s.network_usage as f64, self.settings.network_style()),
                ),
                heat: Some((s.network_usage / (10.0 * 1024.0 * 1024.0)).clamp(0.0, 1.0)),
            },
            "gpu" => Cell {
                text: blank(s.gpu_usage < 0.05, units::percent(s.gpu_usage)),
                heat: Some((s.gpu_usage / 100.0).clamp(0.0, 1.0)),
            },
            "gpu_memory" => Cell {
                text: blank(
                    s.gpu_memory_usage == 0,
                    units::bytes(s.gpu_memory_usage as f64, ByteStyle::MEMORY),
                ),
                heat: None,
            },
            other => Cell { text: n.text.get(other).cloned().unwrap_or_default(), heat: None },
        }
    }

    fn walk(&mut self, nodes: &[Node], depth: u8) {
        for n in nodes {
            let expandable = !n.children.is_empty();
            let expanded = expandable
                && (self.reveal.contains(&n.key)
                    || if n.default_expanded {
                        !self.state.collapsed.contains(&n.key)
                    } else {
                        self.state.expanded.contains(&n.key)
                    });
            let cells = self
                .columns
                .iter()
                .filter(|c| **c != "name")
                .map(|c| self.cell(n, c))
                .collect();
            self.rows.push(Row {
                key: n.key.clone(),
                kind: n.kind.unwrap_or(RowKind::Section),
                depth,
                name: n.name.clone(),
                subtitle: n.subtitle.clone(),
                icon: n.icon.clone(),
                glyph: n.glyph,
                pid: n.pid,
                expandable,
                expanded,
                status: n.status,
                cells,
            });
            if expanded {
                self.walk(&n.children, depth + 1);
            }
        }
    }
}

fn find<'a>(nodes: &'a [Node], key: &str) -> Option<&'a Node> {
    for n in nodes {
        if n.key == key {
            return Some(n);
        }
        if let Some(f) = find(&n.children, key) {
            return Some(f);
        }
    }
    None
}

fn column_order(
    spec: &[(&'static str, &str, bool)],
    order: &[String],
    hidden: &[String],
) -> Vec<(&'static str, String, bool, bool)> {
    let mut cols: Vec<_> = spec
        .iter()
        .map(|(id, title, numeric)| (*id, title.to_string(), *numeric, !hidden.iter().any(|h| h == id)))
        .collect();
    if !order.is_empty() {
        let pos = |id: &str| order.iter().position(|o| o == id).unwrap_or(usize::MAX);
        cols.sort_by_key(|c| if c.0 == "name" { 0 } else { pos(c.0).saturating_add(1) });
    }
    cols[0].3 = true; // name is always visible
    cols
}

// ----- Apps ----------------------------------------------------------------------------------------

pub fn apps_table(r: &Readings, s: &Settings, state: &TableState, icons: &IconCache) -> TableView {
    let tree = s.tree_view;
    let merged = tree && s.merged_process_stats;
    let pid_icons = icons_by_pid(r, icons);

    // Apps section
    let mut app_nodes = vec![];
    let mut app_pids: HashMap<String, Vec<u32>> = HashMap::new();
    for app in r.apps.values() {
        let primary = primary_processes(app, &r.processes);
        if primary.is_empty() {
            continue;
        }
        let key = format!("app:{}", app.id);
        let mut stats = ProcessUsageStats::default();
        for pid in &primary {
            if let Some(p) = r.processes.get(pid) {
                stats.merge(&p.merged_usage_stats(&r.processes));
            }
        }
        let mut visited = HashSet::new();
        let children: Vec<Node> = if tree {
            primary
                .iter()
                .filter_map(|pid| process_node(*pid, &key, r, &pid_icons, merged, true, &mut visited))
                .collect()
        } else {
            let mut pids = HashSet::new();
            for pid in &primary {
                collect_process_subtree(&r.processes, *pid, &mut pids);
            }
            pids.into_iter()
                .filter_map(|pid| process_node(pid, &key, r, &pid_icons, false, false, &mut visited))
                .collect()
        };
        app_pids.insert(key.clone(), stoppable_pids(&primary, &r.processes));
        let suspended = primary
            .iter()
            .filter_map(|p| r.processes.get(p))
            .all(|p| matches!(p.state(), ProcessState::Stopped));
        app_nodes.push(Node {
            search_text: format!("{} {}", app.name.to_lowercase(), app.id.to_lowercase()),
            key,
            kind: Some(RowKind::App),
            name: app.name.clone(),
            subtitle: None,
            icon: icons.get(&app.id),
            glyph: "app",
            pid: None,
            status: suspended.then_some("suspended"),
            stats,
            text: HashMap::new(),
            children,
            default_expanded: false,
        });
    }

    // Processes section
    let root = r.processes.keys().min().copied().unwrap_or(1);
    let mut visited = HashSet::new();
    let proc_nodes: Vec<Node> = match r.processes.get(&root) {
        None => vec![],
        Some(init) if tree => init
            .children
            .iter()
            .filter_map(|c| process_node(*c, "procs", r, &pid_icons, merged, true, &mut visited))
            .collect(),
        Some(init) => {
            let mut pids = HashSet::new();
            for c in &init.children {
                collect_process_subtree(&r.processes, *c, &mut pids);
            }
            pids.into_iter()
                .filter_map(|pid| process_node(pid, "procs", r, &pid_icons, false, false, &mut visited))
                .collect()
        }
    };

    let section = |key: &str, name: &str, children: Vec<Node>| Node {
        key: key.into(),
        kind: Some(RowKind::Section),
        name: name.into(),
        glyph: "section",
        children,
        default_expanded: true,
        ..Default::default()
    };
    let mut roots = vec![section("apps", "Apps", app_nodes), section("procs", "Processes", proc_nodes)];
    for n in roots.iter_mut() {
        n.kind = Some(RowKind::Section);
        // Sections never match searches themselves.
        n.search_text.clear();
    }

    finish_table(
        roots,
        r,
        s,
        state,
        APP_COLUMNS,
        (&s.apps_sort_column, s.apps_sort_direction),
        (&s.apps_column_order, &s.apps_hidden_columns),
        |sel| {
            let is_app = sel.kind == Some(RowKind::App);
            let pids = if is_app {
                app_pids.get(&sel.key).cloned().unwrap_or_default()
            } else {
                sel.pid.into_iter().collect()
            };
            Selection {
                key: sel.key.clone(),
                title: sel.name.clone(),
                pids,
                service_id: None,
                is_app,
                service_running: false,
                service_enabled: false,
                suspended: sel.status == Some("suspended"),
            }
        },
        true,
        format!(
            "{} Running {}",
            r.apps.len(),
            if r.apps.len() == 1 { "App" } else { "Apps" }
        ),
        format!(
            "{} {}",
            units::count(r.processes.len() as u64),
            if r.processes.len() == 1 { "Process" } else { "Processes" }
        ),
        column_totals(r),
    )
}

fn column_totals(r: &Readings) -> HashMap<&'static str, String> {
    let mut t = HashMap::new();
    t.insert("cpu", units::percent_int(r.cpu.total_usage_percent));
    t.insert(
        "memory",
        units::percent_int(mem_used(r) as f32 / r.memory.mem_total.max(1) as f32 * 100.0),
    );
    let busy = r.disks.iter().map(|d| d.busy_percent).fold(0.0, f32::max);
    t.insert("drive", units::percent_int(busy));
    let gpu = r.gpus.values().filter_map(|g| g.utilization_percent).fold(0.0, f32::max);
    if !r.gpus.is_empty() {
        t.insert("gpu", units::percent_int(gpu));
    }
    if r.memory.swap_total > 0 {
        let swap = r.memory.swap_total.saturating_sub(r.memory.swap_free);
        t.insert("swap", units::percent_int(swap as f32 / r.memory.swap_total as f32 * 100.0));
    }
    t
}

#[allow(clippy::too_many_arguments)]
fn finish_table(
    mut roots: Vec<Node>,
    r: &Readings,
    s: &Settings,
    state: &TableState,
    spec: &[(&'static str, &str, bool)],
    (sort_col, dir): (&String, SortDirection),
    (order, hidden): (&Vec<String>, &Vec<String>),
    make_selection: impl Fn(&Node) -> Selection,
    show_network_notice: bool,
    headline: String,
    subheadline: String,
    totals: HashMap<&'static str, String>,
) -> TableView {
    let sort_col = if spec.iter().any(|c| c.0 == sort_col.as_str()) { sort_col.as_str() } else { "name" };
    let asc = dir == SortDirection::Ascending;
    // Sort children of the fixed roots, not the roots themselves when they are sections.
    if roots.iter().all(|n| n.kind == Some(RowKind::Section)) {
        for n in roots.iter_mut() {
            sort_tree(&mut n.children, sort_col, asc);
        }
    } else {
        sort_tree(&mut roots, sort_col, asc);
    }

    let selection = state.selected.as_ref().and_then(|k| find(&roots, k)).map(&make_selection);

    let mut reveal = HashSet::new();
    let q = state.search.trim().to_lowercase();
    if !q.is_empty() {
        filter_tree(&mut roots, &q, &mut reveal);
    }

    let cols = column_order(spec, order, hidden);
    let visible: Vec<&'static str> = cols.iter().filter(|c| c.3).map(|c| c.0).collect();
    let mut f = Flattener {
        state,
        reveal: &reveal,
        columns: &visible,
        settings: s,
        total_mem: r.memory.mem_total.max(1) as f64,
        rows: vec![],
    };
    f.walk(&roots, 0);

    let notice = r.network_stats_error.as_ref().filter(|_| show_network_notice).map(|_| {
        "Per-process network usage needs a one-time setup. Open Preferences › Apps & Services to enable it."
            .to_owned()
    });

    TableView {
        headline,
        subheadline,
        columns: cols
            .into_iter()
            .map(|(id, title, numeric, visible)| Column {
                total: totals.get(id).cloned(),
                id,
                title,
                numeric,
                visible,
            })
            .collect(),
        rows: f.rows,
        sort_column: sort_col.to_owned(),
        sort_ascending: asc,
        selection,
        notice,
    }
}

// ----- Services ------------------------------------------------------------------------------------

fn service_status(s: &Service) -> &'static str {
    if s.running {
        "running"
    } else if s.failed {
        "failed"
    } else if s.enabled {
        "stopped"
    } else {
        "disabled"
    }
}

pub fn services_table(
    r: &Readings,
    s: &Settings,
    state: &TableState,
    user: bool,
    icons: &IconCache,
) -> TableView {
    let services = if user { &r.user_services } else { &r.system_services };
    let pid_icons = icons_by_pid(r, icons);
    let mut visited = HashSet::new();

    let roots: Vec<Node> = services
        .values()
        .map(|svc| {
            let key = format!("svc:{}", svc.id);
            let status = service_status(svc);
            let children = match svc.pid.filter(|_| s.tree_view) {
                Some(pid) => process_node(pid, &key, r, &pid_icons, false, true, &mut visited)
                    .into_iter()
                    .collect(),
                None => vec![],
            };
            let mut text = HashMap::new();
            text.insert(
                "status",
                match status {
                    "running" => "Running",
                    "failed" => "Failed",
                    "stopped" => "Stopped",
                    _ => "Inactive",
                }
                .to_owned(),
            );
            text.insert("startup", if svc.enabled { "Enabled" } else { "Disabled" }.to_owned());
            text.insert("user", svc.user.clone().unwrap_or_default());
            text.insert("group", svc.group.clone().unwrap_or_default());
            text.insert("description", svc.description.clone().unwrap_or_default());
            let name = svc.name.trim_end_matches(".service").to_owned();
            Node {
                search_text: format!(
                    "{} {}",
                    svc.name.to_lowercase(),
                    svc.description.clone().unwrap_or_default().to_lowercase()
                ),
                key,
                kind: Some(RowKind::Service),
                subtitle: svc.description.clone(),
                name,
                icon: None,
                glyph: "service",
                pid: svc.pid,
                status: Some(status),
                stats: svc
                    .pid
                    .and_then(|p| r.processes.get(&p))
                    .map(|p| p.merged_usage_stats(&r.processes))
                    .unwrap_or_default(),
                text,
                children,
                default_expanded: false,
            }
        })
        .collect();

    let by_id: HashMap<String, &Service> =
        services.values().map(|svc| (format!("svc:{}", svc.id), svc)).collect();
    let running = services.values().filter(|s| s.running).count();
    let failed = services.values().filter(|s| s.failed).count();

    finish_table(
        roots,
        r,
        s,
        state,
        SERVICE_COLUMNS,
        (&s.services_sort_column, s.services_sort_direction),
        (&s.services_column_order, &s.services_hidden_columns),
        |sel| match by_id.get(&sel.key) {
            Some(svc) => Selection {
                key: sel.key.clone(),
                title: sel.name.clone(),
                pids: svc.pid.into_iter().collect(),
                service_id: Some(svc.id),
                is_app: false,
                service_running: svc.running,
                service_enabled: svc.enabled,
                suspended: false,
            },
            None => Selection {
                key: sel.key.clone(),
                title: sel.name.clone(),
                pids: sel.pid.into_iter().collect(),
                service_id: None,
                is_app: false,
                service_running: false,
                service_enabled: false,
                suspended: sel.status == Some("suspended"),
            },
        },
        false,
        format!("{} {} Services", services.len(), if user { "User" } else { "System" }),
        if failed > 0 {
            format!("{running} running · {failed} failed")
        } else {
            format!("{running} running")
        },
        HashMap::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magpie::types::processes::Process;

    fn proc(pid: u32, parent: u32, name: &str, children: &[u32], cpu: f32) -> Process {
        Process {
            name: name.into(),
            exe: format!("/usr/bin/{name}"),
            pid,
            parent,
            children: children.to_vec(),
            usage_stats: ProcessUsageStats { cpu_usage: cpu, memory_usage: 1024, ..Default::default() },
            ..Default::default()
        }
    }

    fn readings() -> Readings {
        let mut r = Readings::default();
        for p in [
            proc(1, 0, "systemd", &[10, 20, 30], 0.0),
            proc(10, 1, "firefox", &[11, 12], 5.0),
            proc(11, 10, "firefox", &[], 10.0),
            proc(12, 10, "firefox", &[], 1.0),
            proc(20, 1, "bash", &[21], 0.5),
            proc(21, 20, "top", &[], 2.0),
            proc(30, 1, "bwrap", &[31], 0.0),
            proc(31, 30, "flatpak-app", &[], 3.0),
        ] {
            r.processes.insert(p.pid, p);
        }
        r.apps.insert(
            "firefox".into(),
            App { id: "firefox".into(), name: "Firefox".into(), command: "firefox".into(), pids: vec![10, 11, 12] },
        );
        r.apps.insert(
            "flat".into(),
            App { id: "flat".into(), name: "Flat App".into(), command: "x".into(), pids: vec![30, 31] },
        );
        r.memory.mem_total = 1 << 30;
        r
    }

    #[test]
    fn primary_and_stoppable() {
        let r = readings();
        assert_eq!(primary_processes(&r.apps["firefox"], &r.processes), vec![10]);
        let flat = primary_processes(&r.apps["flat"], &r.processes);
        assert_eq!(flat, vec![30]);
        assert_eq!(stoppable_pids(&flat, &r.processes), vec![31]);
    }

    #[test]
    fn apps_table_tree_sort_and_search() {
        let r = readings();
        let s = Settings::default();
        let mut st = TableState::default();
        let t = apps_table(&r, &s, &st, &IconCache::default());
        // sections expanded, apps collapsed; sorted by cpu desc → Firefox (16%) before Flat App (3%)
        let names: Vec<_> = t.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names[0], "Apps");
        assert_eq!(names[1], "Firefox");
        assert_eq!(names[2], "Flat App");
        assert_eq!(names[3], "Processes");
        assert!((t.rows[1].cells[1].heat.unwrap() - 0.16).abs() < 0.01);

        st.toggle("app:firefox", false);
        st.selected = Some("app:firefox".into());
        let t = apps_table(&r, &s, &st, &IconCache::default());
        assert_eq!(t.rows[2].key, "app:firefox/10");
        assert_eq!(t.rows[2].depth, 2);
        assert_eq!(t.selection.as_ref().unwrap().pids, vec![10]);

        st.search = "top".into();
        let t = apps_table(&r, &s, &st, &IconCache::default());
        let names: Vec<_> = t.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Processes", "bash", "top"]);
    }
}
