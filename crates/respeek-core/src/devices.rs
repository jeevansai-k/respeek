/* devices.rs — performance data: history recording, sidebar summaries and detail pages.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::history::Histories;
use crate::magpie::types::network::ConnectionKind;
use crate::magpie::{Battery, Connection, Disk, Fan, Gpu, MemoryDevice};
use crate::model::*;
use crate::readings::Readings;
use crate::settings::Settings;
use crate::units::{self, ByteStyle};

// ----- ids ---------------------------------------------------------------------------------------

pub fn disk_id(d: &Disk) -> String {
    format!("disk:{}", d.id)
}
pub fn net_id(c: &Connection) -> String {
    format!("net:{}", c.id)
}
pub fn gpu_id(id: &str) -> String {
    format!("gpu:{id}")
}
pub fn fan_id(f: &Fan) -> String {
    format!("fan:{}-{}", f.hwmon_index, f.fan_index)
}
pub fn battery_id(b: &Battery) -> String {
    format!("battery:{}", b.name)
}

fn sorted_disks(r: &Readings) -> Vec<&Disk> {
    let mut v: Vec<&Disk> = r.disks.iter().collect();
    v.sort_by(|a, b| (!a.is_system, &a.id).cmp(&(!b.is_system, &b.id)));
    v
}

fn sorted_connections<'a>(r: &'a Readings, s: &Settings) -> Vec<&'a Connection> {
    let mut v: Vec<&Connection> = r
        .connections
        .values()
        .filter(|c| match c.kind() {
            ConnectionKind::Wired => s.show_network_wired,
            ConnectionKind::Wireless => s.show_network_wireless,
            ConnectionKind::Vpn => s.show_network_vpn,
            ConnectionKind::Virtual
            | ConnectionKind::Docker
            | ConnectionKind::Bridge
            | ConnectionKind::Multipass => s.show_network_virtual,
            _ => s.show_network_other,
        })
        .collect();
    v.sort_by_key(|c| (kind_rank(c.kind()), c.id.clone()));
    v
}

fn kind_rank(k: ConnectionKind) -> u8 {
    match k {
        ConnectionKind::Wired => 0,
        ConnectionKind::Wireless => 1,
        ConnectionKind::Wwan => 2,
        ConnectionKind::Vpn => 3,
        _ => 4,
    }
}

fn sorted_gpus(r: &Readings) -> Vec<&Gpu> {
    let mut v: Vec<&Gpu> = r.gpus.values().collect();
    v.sort_by(|a, b| a.id.cmp(&b.id));
    v
}

fn disk_kind_label(d: &Disk) -> &'static str {
    if d.kind.is_none() {
        return "Disk";
    }
    match d.kind().as_str_name() {
        "HDD" => "HDD",
        "SSD" => "SSD",
        "NVMe" => "NVMe SSD",
        "eMMC" => "eMMC",
        "SD" => "SD Card",
        "iSCSI" => "iSCSI",
        "Floppy" => "Floppy",
        "Optical" => "Optical Drive",
        "ThumbDrive" => "USB Drive",
        _ => "Disk",
    }
}

fn connection_label(c: &Connection) -> &'static str {
    match c.kind() {
        ConnectionKind::Wired => "Ethernet",
        ConnectionKind::Wireless => "Wi-Fi",
        ConnectionKind::Wwan => "Cellular",
        ConnectionKind::Vpn => "VPN",
        ConnectionKind::Bluetooth => "Bluetooth",
        ConnectionKind::Bridge => "Bridge",
        ConnectionKind::Docker => "Docker",
        ConnectionKind::Multipass => "Multipass",
        ConnectionKind::Virtual => "Virtual",
        ConnectionKind::InfiniBand => "InfiniBand",
        ConnectionKind::Other => "Network",
    }
}

pub fn mem_used(r: &Readings) -> u64 {
    let m = &r.memory;
    let avail = if m.mem_available > m.mem_total { m.mem_free } else { m.mem_available };
    m.mem_total.saturating_sub(avail)
}

// ----- scaling -----------------------------------------------------------------------------------

/// Round a rate up to a "nice" 1/2/5 × 10ⁿ ceiling so graph scales don't jitter.
pub fn nice_ceiling(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let exp = v.log10().floor();
    let base = 10f64.powf(exp);
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if v <= m * base {
            return m * base;
        }
    }
    10.0 * base
}

/// Normalize to 0..=1 and quantize to 1/1000 — far below a pixel at any graph size, and it
/// keeps each serialized sample to a few bytes ("0.42" instead of "0.41999998").
fn norm(values: Vec<f32>, max: f32) -> Vec<f32> {
    let max = if max > 0.0 { max } else { 1.0 };
    values
        .into_iter()
        .map(|v| ((v / max).clamp(0.0, 1.0) * 1000.0).round() / 1000.0)
        .collect()
}

fn pct_series(h: &Histories, key: &str, label: &str, tint: Tint, dashed: bool) -> Series {
    let last = h.get(key).map(|r| r.last()).unwrap_or(0.0);
    Series {
        label: label.into(),
        tint,
        values: norm(h.values(key), 100.0),
        dashed,
        current: units::percent(last),
    }
}

fn rate_series(
    h: &Histories,
    key: &str,
    label: &str,
    tint: Tint,
    dashed: bool,
    max: f32,
    style: ByteStyle,
) -> Series {
    let last = h.get(key).map(|r| r.last()).unwrap_or(0.0);
    Series {
        label: label.into(),
        tint,
        values: norm(h.values(key), max),
        dashed,
        current: units::rate(last as f64, style),
    }
}

// ----- recording ---------------------------------------------------------------------------------

pub fn record(h: &mut Histories, r: &Readings) {
    let cpu = &r.cpu;
    h.push("cpu/total", cpu.total_usage_percent);
    h.push("cpu/kernel", cpu.kernel_usage_percent);
    for (i, v) in cpu.core_usage_percent.iter().enumerate() {
        h.push(&format!("cpu/core{i}"), *v);
    }
    for (i, v) in cpu.core_kernel_usage_percent.iter().enumerate() {
        h.push(&format!("cpu/kcore{i}"), *v);
    }

    let m = &r.memory;
    let total = m.mem_total.max(1) as f32;
    h.push("memory/used", mem_used(r) as f32 / total * 100.0);
    let swap_used = m.swap_total.saturating_sub(m.swap_free) as f32;
    h.push(
        "memory/swap",
        if m.swap_total > 0 { swap_used / m.swap_total as f32 * 100.0 } else { 0.0 },
    );

    for d in &r.disks {
        let id = disk_id(d);
        h.push(&format!("{id}/busy"), d.busy_percent);
        h.push(&format!("{id}/read"), d.rx_speed_bytes_ps as f32);
        h.push(&format!("{id}/write"), d.tx_speed_bytes_ps as f32);
    }

    for c in r.connections.values() {
        let id = net_id(c);
        h.push(&format!("{id}/rx"), c.rx_rate_bytes_ps);
        h.push(&format!("{id}/tx"), c.tx_rate_bytes_ps);
    }

    for g in r.gpus.values() {
        let id = gpu_id(&g.id);
        h.push(&format!("{id}/usage"), g.utilization_percent.unwrap_or(0.0));
        let mem = match (g.used_memory, g.total_memory) {
            (Some(u), Some(t)) if t > 0 => u as f32 / t as f32 * 100.0,
            _ => 0.0,
        };
        h.push(&format!("{id}/memory"), mem);
        h.push(&format!("{id}/encode"), g.encoder_percent.unwrap_or(0.0));
        h.push(&format!("{id}/decode"), g.decoder_percent.unwrap_or(0.0));
        h.push(&format!("{id}/power"), g.power_draw_watts.unwrap_or(0.0));
        h.push(&format!("{id}/temp"), g.temperature_c.unwrap_or(0.0));
    }

    for f in &r.fans {
        let id = fan_id(f);
        h.push(&format!("{id}/rpm"), f.rpm as f32);
        h.push(&format!("{id}/temp"), f.temp_amount.map(|t| t as f32 / 1000.0 - 273.15).unwrap_or(0.0));
    }

    for b in &r.batteries {
        let id = battery_id(b);
        h.push(&format!("{id}/charge"), b.percentage);
        h.push(&format!("{id}/power"), b.power.unwrap_or(0.0).abs());
    }

    h.end_tick();
}

fn rate_max(h: &Histories, keys: &[String], s: &Settings, hard: Option<f64>) -> f32 {
    let observed = keys.iter().map(|k| h.max(k)).fold(0.0, f32::max) as f64;
    if !s.network_dynamic_scaling {
        if let Some(hard) = hard {
            return hard as f32;
        }
    }
    nice_ceiling(observed.max(1024.0)) as f32
}

// ----- sidebar -----------------------------------------------------------------------------------

pub fn summaries(r: &Readings, h: &Histories, s: &Settings) -> Vec<DeviceSummary> {
    let mut out = Vec::new();

    if s.show_cpu {
        let cpu = &r.cpu;
        out.push(DeviceSummary {
            id: "cpu".into(),
            kind: DeviceKind::Cpu,
            title: "CPU".into(),
            subtitle: format!(
                "{} · {}",
                units::percent_int(cpu.total_usage_percent),
                units::mhz(cpu.current_frequency_mhz)
            ),
            level: (cpu.total_usage_percent / 100.0).clamp(0.0, 1.0),
            tint: Tint::Cpu,
            spark: norm(h.values("cpu/total"), 100.0),
            level_label: String::new(),
            spark2: s.kernel_times.then(|| norm(h.values("cpu/kernel"), 100.0)),
        });
    }

    if s.show_memory {
        let m = &r.memory;
        let used = mem_used(r);
        out.push(DeviceSummary {
            id: "memory".into(),
            kind: DeviceKind::Memory,
            title: "Memory".into(),
            subtitle: format!(
                "{} / {}",
                units::bytes(used as f64, s.memory_style()),
                units::bytes(m.mem_total as f64, s.memory_style())
            ),
            level: used as f32 / m.mem_total.max(1) as f32,
            tint: Tint::Memory,
            spark: norm(h.values("memory/used"), 100.0),
            level_label: String::new(),
            spark2: None,
        });
    }

    if s.show_disks {
        for (i, d) in sorted_disks(r).into_iter().enumerate() {
            let id = disk_id(d);
            out.push(DeviceSummary {
                title: format!("Disk {i}"),
                subtitle: format!("{} · {}", disk_kind_label(d), units::percent_int(d.busy_percent)),
                level: d.busy_percent / 100.0,
                kind: DeviceKind::Disk,
                tint: Tint::Disk,
                spark: norm(h.values(&format!("{id}/busy")), 100.0),
                level_label: String::new(),
                spark2: None,
                id,
            });
        }
    }

    if s.show_network {
        for c in sorted_connections(r, s) {
            let id = net_id(c);
            let keys = [format!("{id}/rx"), format!("{id}/tx")];
            let max = rate_max(h, &keys, s, c.max_speed_bytes_ps.map(|v| v as f64));
            let style = s.network_style();
            out.push(DeviceSummary {
                title: connection_label(c).into(),
                subtitle: format!(
                    "↓ {}  ↑ {}",
                    units::rate(c.rx_rate_bytes_ps as f64, style),
                    units::rate(c.tx_rate_bytes_ps as f64, style)
                ),
                level: (c.rx_rate_bytes_ps.max(c.tx_rate_bytes_ps) / max).clamp(0.0, 1.0),
                kind: DeviceKind::Network,
                tint: Tint::Network,
                spark: norm(h.values(&keys[0]), max),
                level_label: String::new(),
                spark2: Some(norm(h.values(&keys[1]), max)),
                id,
            });
        }
    }

    if s.show_gpus {
        for (i, g) in sorted_gpus(r).into_iter().enumerate() {
            let id = gpu_id(&g.id);
            let usage = g.utilization_percent.unwrap_or(0.0);
            let mut subtitle = units::percent_int(usage);
            if let Some(t) = g.temperature_c {
                subtitle.push_str(&format!(" · {}", units::celsius(t)));
            }
            out.push(DeviceSummary {
                title: format!("GPU {i}"),
                subtitle,
                level: usage / 100.0,
                kind: DeviceKind::Gpu,
                tint: Tint::Gpu,
                spark: norm(h.values(&format!("{id}/usage")), 100.0),
                level_label: String::new(),
                spark2: None,
                id,
            });
        }
    }

    if s.show_fans {
        for f in &r.fans {
            let id = fan_id(f);
            let max = f.max_rpm.map(|m| m as f32).unwrap_or_else(|| {
                nice_ceiling(h.max(&format!("{id}/rpm")).max(1000.0) as f64) as f32
            });
            out.push(DeviceSummary {
                title: f.fan_label.clone().unwrap_or_else(|| format!("Fan {}", f.fan_index)),
                subtitle: format!("{} RPM", units::count(f.rpm as u64)),
                level: (f.rpm as f32 / max).clamp(0.0, 1.0),
                kind: DeviceKind::Fan,
                tint: Tint::Fan,
                spark: norm(h.values(&format!("{id}/rpm")), max),
                level_label: String::new(),
                spark2: None,
                id,
            });
        }
    }

    if s.show_batteries {
        for b in &r.batteries {
            let id = battery_id(b);
            out.push(DeviceSummary {
                title: "Battery".into(),
                subtitle: format!("{} · {}", units::percent_int(b.percentage), battery_state(b)),
                level: b.percentage / 100.0,
                kind: DeviceKind::Battery,
                tint: Tint::Battery,
                spark: norm(h.values(&format!("{id}/charge")), 100.0),
                level_label: String::new(),
                spark2: None,
                id,
            });
        }
    }

    for d in out.iter_mut() {
        d.level_label = units::percent_int(d.level * 100.0);
    }
    out.retain(|d| !s.sidebar_hidden_graphs.contains(&d.id));
    if !s.sidebar_order.is_empty() {
        // Stable: known ids follow the saved order, new devices keep their natural place after.
        let pos = |id: &str| s.sidebar_order.iter().position(|o| o == id).unwrap_or(usize::MAX);
        out.sort_by_key(|d| pos(&d.id));
    }
    out
}

fn battery_state(b: &Battery) -> &'static str {
    if b.state.is_none() {
        return "Unknown";
    }
    match b.state().as_str_name() {
        "Charging" => "Charging",
        "Discharging" => "On battery",
        "Empty" => "Empty",
        "FullyCharged" => "Charged",
        "PendingCharge" => "Not charging",
        "PendingDischarge" => "Waiting",
        _ => "Unknown",
    }
}

// ----- detail pages ------------------------------------------------------------------------------

pub fn detail(id: &str, r: &Readings, h: &Histories, s: &Settings) -> Option<DeviceDetail> {
    if id == "cpu" {
        return Some(cpu_detail(r, h, s));
    }
    if id == "memory" {
        return Some(memory_detail(r, h, s));
    }
    if let Some((i, d)) = sorted_disks(r).into_iter().enumerate().find(|(_, d)| disk_id(d) == id) {
        return Some(disk_detail(i, d, h, s));
    }
    if let Some(c) = r.connections.values().find(|c| net_id(c) == id) {
        return Some(network_detail(c, h, s));
    }
    if let Some((i, g)) = sorted_gpus(r).into_iter().enumerate().find(|(_, g)| gpu_id(&g.id) == id)
    {
        return Some(gpu_detail(i, g, h, s));
    }
    if let Some(f) = r.fans.iter().find(|f| fan_id(f) == id) {
        return Some(fan_detail(f, h));
    }
    if let Some(b) = r.batteries.iter().find(|b| battery_id(b) == id) {
        return Some(battery_detail(b, h));
    }
    None
}

fn cpu_detail(r: &Readings, h: &Histories, s: &Settings) -> DeviceDetail {
    let cpu = &r.cpu;
    let mut series = vec![pct_series(h, "cpu/total", "Utilization", Tint::Cpu, false)];
    if s.kernel_times {
        series.push(pct_series(h, "cpu/kernel", "Kernel", Tint::Kernel, true));
    }

    let grid = if s.cpu_graph == 2 {
        (0..cpu.core_usage_percent.len())
            .map(|i| {
                let mut series = vec![pct_series(
                    h,
                    &format!("cpu/core{i}"),
                    &format!("CPU {i}"),
                    Tint::Cpu,
                    false,
                )];
                if s.kernel_times {
                    series.push(pct_series(
                        h,
                        &format!("cpu/kcore{i}"),
                        "Kernel",
                        Tint::Kernel,
                        true,
                    ));
                }
                Graph {
                    id: format!("core{i}"),
                    title: format!("CPU {i}"),
                    max_label: "100%".into(),
                    series,
                    size: "mini",
                }
            })
            .collect()
    } else {
        vec![]
    };

    let mut live = vec![
        primary("Utilization", units::percent_int(cpu.total_usage_percent), Tint::Cpu),
        primary("Speed", units::mhz(cpu.current_frequency_mhz), Tint::Cpu),
        stat("Processes", units::count(cpu.total_process_count)),
        stat("Threads", units::count(cpu.total_thread_count)),
        stat("Handles", units::count(cpu.total_handle_count)),
        stat("Up time", units::uptime(cpu.uptime_seconds)),
    ];
    if let Some(t) = cpu.temperature_celsius {
        live.push(stat("Temperature", units::celsius(t)));
    }
    if let Some(p) = cpu.power_draw_w {
        live.push(stat("Power draw", units::watts(p)));
    }

    let mut info = vec![];
    if let Some(f) = cpu.base_freq_khz {
        info.push(stat("Base speed", units::khz(f)));
    }
    if let Some(n) = cpu.socket_count {
        info.push(stat("Sockets", n.to_string()));
    }
    info.push(stat("Logical processors", cpu.core_usage_percent.len().to_string()));
    if let Some(v) = &cpu.virtualization_technology {
        info.push(stat("Virtualization", v.clone()));
    }
    if let Some(vm) = cpu.is_virtual_machine {
        info.push(stat("Virtual machine", if vm { "Yes" } else { "No" }));
    }
    for (label, v) in [
        ("L1 cache", cpu.l1_combined_cache_bytes),
        ("L2 cache", cpu.l2_cache_bytes),
        ("L3 cache", cpu.l3_cache_bytes),
        ("L4 cache", cpu.l4_cache_bytes),
    ] {
        if let Some(v) = v.filter(|v| *v > 0) {
            info.push(stat(label, units::bytes(v as f64, ByteStyle::MEMORY)));
        }
    }
    if let Some(d) = &cpu.frequency_driver {
        info.push(stat("Frequency driver", d.clone()));
    }
    if let Some(g) = &cpu.frequency_governor {
        info.push(stat("Governor", g.clone()));
    }
    if let Some(p) = &cpu.power_preference {
        info.push(stat("Power preference", p.clone()));
    }

    DeviceDetail {
        id: "cpu".into(),
        kind: DeviceKind::Cpu,
        title: "CPU".into(),
        model: cpu.name.clone().unwrap_or_else(|| "Processor".into()),
        graphs: vec![Graph {
            id: "cpu".into(),
            title: if s.cpu_graph == 2 { "Overall utilization".into() } else { "Utilization".into() },
            max_label: "100%".into(),
            series,
            size: "hero",
        }],
        grid,
        stats: vec![group(None, live), group(Some("Processor"), info)],
        extras: vec![],
    }
}

fn memory_detail(r: &Readings, h: &Histories, s: &Settings) -> DeviceDetail {
    let m = &r.memory;
    let style = s.memory_style();
    let total = m.mem_total.max(1);
    let used = mem_used(r);
    let modified = m.dirty;
    let free = m.mem_free;
    let standby = total.saturating_sub(used + free);
    let swap_used = m.swap_total.saturating_sub(m.swap_free);
    let b = |v: u64| units::bytes(v as f64, style);

    let mut graphs = vec![Graph {
        id: "memory".into(),
        title: "Memory usage".into(),
        max_label: b(total),
        series: vec![Series {
            current: b(used),
            ..pct_series(h, "memory/used", "In use", Tint::Memory, false)
        }],
        size: "hero",
    }];
    if s.memory_swap_visible && m.swap_total > 0 {
        graphs.push(Graph {
            id: "swap".into(),
            title: "Swap".into(),
            max_label: b(m.swap_total),
            series: vec![Series {
                current: b(swap_used),
                ..pct_series(h, "memory/swap", "Swap", Tint::Swap, false)
            }],
            size: "half",
        });
    }

    let mut extras = vec![];
    if s.memory_composition_visible {
        let f = |v: u64| v as f32 / total as f32;
        extras.push(Extra::Composition {
            title: "Memory composition".into(),
            segments: vec![
                Segment {
                    label: "In use".into(),
                    value: b(used),
                    fraction: f(used),
                    tint: Tint::Memory,
                    tooltip: "Memory used by the operating system and running applications".into(),
                },
                Segment {
                    label: "Modified".into(),
                    value: b(modified),
                    fraction: f(modified),
                    tint: Tint::Swap,
                    tooltip: "Memory whose contents must be written to disk before it can be used by another process".into(),
                },
                Segment {
                    label: "Standby".into(),
                    value: b(standby),
                    fraction: f(standby),
                    tint: Tint::Kernel,
                    tooltip: "Memory that contains cached data and code that is not actively in use".into(),
                },
                Segment {
                    label: "Free".into(),
                    value: b(free),
                    fraction: f(free),
                    tint: Tint::Neutral,
                    tooltip: "Memory that is not currently in use, and that will be repurposed first when the operating system, drivers, or applications need more memory".into(),
                },
            ],
        });
    }

    if !r.memory_devices.is_empty() {
        extras.push(memory_slots(&r.memory_devices, m.max_devices));
    }

    let mut live = vec![
        primary("In use", b(used), Tint::Memory),
        primary("Available", b(total.saturating_sub(used)), Tint::Memory),
        stat("Committed", format!("{} / {}", b(m.committed), b(m.commit_limit))),
        stat("Cached", b(m.cached)),
    ];
    if m.swap_total > 0 {
        live.push(stat("Swap used", format!("{} / {}", b(swap_used), b(m.swap_total))));
    }
    if m.zram_orig_size > 0 {
        live.push(stat(
            "zram",
            format!("{} → {}", b(m.zram_orig_size), b(m.zram_compr_size)),
        ));
    }

    let mut hw = vec![];
    if let Some(d) = r.memory_devices.first() {
        if d.speed > 0 {
            hw.push(stat("Speed", format!("{} MT/s", d.speed)));
        }
        hw.push(stat("Type", d.ram_type.clone()));
        hw.push(stat("Form factor", d.form_factor.clone()));
        hw.push(stat(
            "Slots used",
            format!("{} of {}", r.memory_devices.len(), m.max_devices.max(r.memory_devices.len() as u64)),
        ));
    }

    let mut stats = vec![group(None, live)];
    if !hw.is_empty() {
        stats.push(group(Some("Hardware"), hw));
    }

    DeviceDetail {
        id: "memory".into(),
        kind: DeviceKind::Memory,
        title: "Memory".into(),
        model: format!("{} total", b(total)),
        graphs,
        grid: vec![],
        stats,
        extras,
    }
}

fn memory_slots(devices: &[MemoryDevice], max: u64) -> Extra {
    let groups = devices
        .iter()
        .map(|d| {
            group(
                Some(&d.locator),
                vec![
                    stat("Size", units::bytes(d.size as f64, ByteStyle::MEMORY)),
                    stat("Type", d.ram_type.clone()),
                    stat("Speed", format!("{} MT/s", d.speed)),
                    stat("Form factor", d.form_factor.clone()),
                    stat("Bank", d.bank_locator.clone()),
                    stat("Rank", d.rank.to_string()),
                ],
            )
        })
        .collect();
    Extra::Details { title: format!("Memory slots ({} of {})", devices.len(), max.max(devices.len() as u64)), groups }
}

fn disk_detail(index: usize, d: &Disk, h: &Histories, s: &Settings) -> DeviceDetail {
    let id = disk_id(d);
    let style = s.drive_style();
    let keys = [format!("{id}/read"), format!("{id}/write")];
    let observed = keys.iter().map(|k| h.max(k)).fold(0.0, f32::max) as f64;
    let max = nice_ceiling(observed.max(1024.0 * 1024.0)) as f32;

    let graphs = vec![
        Graph {
            id: "active".into(),
            title: "Active time".into(),
            max_label: "100%".into(),
            series: vec![pct_series(h, &format!("{id}/busy"), "Active time", Tint::Disk, false)],
            size: "hero",
        },
        Graph {
            id: "transfer".into(),
            title: "Transfer rate".into(),
            max_label: units::rate(max as f64, style),
            series: vec![
                rate_series(h, &keys[0], "Read", Tint::Disk, false, max, style),
                rate_series(h, &keys[1], "Write", Tint::DiskWrite, true, max, style),
            ],
            size: "half",
        },
    ];

    let b = |v: u64| units::bytes(v as f64, style);
    let live = vec![
        primary("Active time", units::percent_int(d.busy_percent), Tint::Disk),
        primary("Avg. response", format!("{:.2} ms", d.response_time_ms), Tint::Disk),
        primary("Read speed", units::rate(d.rx_speed_bytes_ps as f64, style), Tint::Disk),
        primary("Write speed", units::rate(d.tx_speed_bytes_ps as f64, style), Tint::DiskWrite),
        stat("Total read", b(d.rx_bytes_total)),
        stat("Total written", b(d.tx_bytes_total)),
    ];

    let mut info = vec![
        stat("Capacity", b(d.capacity_bytes)),
        stat("Type", disk_kind_label(d)),
        stat("System disk", if d.is_system { "Yes" } else { "No" }),
    ];
    if let Some(f) = d.formatted_bytes {
        info.insert(1, stat("Formatted", b(f)));
    }
    if let Some(t) = d.temperature_milli_k {
        info.push(stat("Temperature", units::milli_kelvin(t)));
    }
    if let Some(sn) = &d.serial_number {
        info.push(stat("Serial number", sn.clone()));
    }
    if let Some(w) = &d.world_wide_name {
        info.push(stat("WWN", w.clone()));
    }
    if let Some(rr) = d.rotation_rate.filter(|r| *r > 0) {
        info.push(stat("Rotation rate", format!("{rr} RPM")));
    }

    let mut parts: Vec<_> = d.partitions.values().collect();
    parts.sort_by(|a, b| a.devname.cmp(&b.devname));
    let items: Vec<UsageBar> = parts
        .into_iter()
        .map(|p| {
            let size = p.size.unwrap_or(0);
            let used = p.used.unwrap_or(0);
            let mount = p.mountpoints.first().cloned().unwrap_or_else(|| "Not mounted".into());
            UsageBar {
                title: format!("{} · {}", p.devname, mount),
                subtitle: match (p.used, p.size) {
                    (Some(_), Some(_)) => format!(
                        "{} of {} used{}",
                        b(used),
                        b(size),
                        p.filesystem.as_ref().map(|f| format!(" · {f}")).unwrap_or_default()
                    ),
                    _ => format!(
                        "{}{}",
                        b(size),
                        p.filesystem.as_ref().map(|f| format!(" · {f}")).unwrap_or_default()
                    ),
                },
                fraction: if size > 0 { used as f32 / size as f32 } else { 0.0 },
                tint: Tint::Disk,
            }
        })
        .collect();

    let mut extras = vec![];
    if !items.is_empty() {
        extras.push(Extra::Partitions { items });
    }
    extras.push(Extra::Actions { smart: d.smart_interface.is_some(), eject: d.ejectable });

    DeviceDetail {
        id,
        kind: DeviceKind::Disk,
        title: format!("Disk {index}"),
        model: d.model.clone().unwrap_or_else(|| format!("{} ({})", disk_kind_label(d), d.id)),
        graphs,
        grid: vec![],
        stats: vec![group(None, live), group(Some("Drive"), info)],
        extras,
    }
}

fn network_detail(c: &Connection, h: &Histories, s: &Settings) -> DeviceDetail {
    let id = net_id(c);
    let style = s.network_style();
    let keys = [format!("{id}/rx"), format!("{id}/tx")];
    let max = rate_max(h, &keys, s, c.max_speed_bytes_ps.map(|v| v as f64));

    let graphs = vec![Graph {
        id: "throughput".into(),
        title: "Throughput".into(),
        max_label: units::rate(max as f64, style),
        series: vec![
            rate_series(h, &keys[0], "Receive", Tint::Network, false, max, style),
            rate_series(h, &keys[1], "Send", Tint::NetworkSend, true, max, style),
        ],
        size: "hero",
    }];

    let total_style = ByteStyle { bytes: true, base2: style.base2 };
    let live = vec![
        primary("Receive", units::rate(c.rx_rate_bytes_ps as f64, style), Tint::Network),
        primary("Send", units::rate(c.tx_rate_bytes_ps as f64, style), Tint::NetworkSend),
        stat("Total received", units::bytes(c.rx_total_bytes as f64, total_style)),
        stat("Total sent", units::bytes(c.tx_total_bytes as f64, total_style)),
    ];

    let mut info = vec![
        stat("Interface", c.id.clone()),
        stat("Connection type", connection_label(c)),
        stat("State", c.state().as_str_name()),
    ];
    if let Some(m) = c.max_speed_bytes_ps.filter(|m| *m > 0) {
        info.push(stat("Link speed", units::rate(m as f64, ByteStyle { bytes: false, base2: false })));
    }
    if let Some(w) = &c.wireless_connection {
        if let Some(ssid) = &w.ssid {
            info.push(stat("SSID", ssid.clone()));
        }
        if let Some(sig) = w.signal_strength_percent {
            info.push(stat("Signal strength", format!("{sig}%")));
        }
        if let Some(br) = w.bitrate_kbps {
            info.push(stat("Bitrate", format!("{:.0} Mb/s", br as f64 / 1000.0)));
        }
        if let Some(f) = w.frequency_mhz {
            info.push(stat("Frequency", units::mhz(f as u64)));
        }
    }
    if !c.hw_address.is_empty() {
        info.push(stat("Hardware address", c.hw_address.clone()));
    }
    if let Some(a) = &c.ipv4_address {
        info.push(stat("IPv4 address", a.clone()));
    }
    if let Some(a) = &c.ipv6_address {
        info.push(stat("IPv6 address", a.clone()));
    }
    if let Some(t) = c.temperature_milli_k {
        info.push(stat("Temperature", units::milli_kelvin(t)));
    }

    DeviceDetail {
        id,
        kind: DeviceKind::Network,
        title: connection_label(c).into(),
        model: c.device_name.clone().unwrap_or_else(|| c.id.clone()),
        graphs,
        grid: vec![],
        stats: vec![group(None, live), group(Some("Adapter"), info)],
        extras: vec![],
    }
}

/// Graph choices for the GPU middle/bottom slots (upstream `performance-page-gpu-graph-*`).
fn gpu_graph(slot: u8, id: &str, g: &Gpu, h: &Histories, s: &Settings) -> Option<Graph> {
    let mem_style = s.memory_style();
    match slot {
        1 => Some(Graph {
            id: "memory".into(),
            title: "Memory usage".into(),
            max_label: g
                .total_memory
                .map(|t| units::bytes(t as f64, mem_style))
                .unwrap_or_else(|| "100%".into()),
            series: vec![Series {
                current: g
                    .used_memory
                    .map(|u| units::bytes(u as f64, mem_style))
                    .unwrap_or_else(|| "—".into()),
                ..pct_series(h, &format!("{id}/memory"), "Memory", Tint::GpuMemory, false)
            }],
            size: "half",
        }),
        2 => {
            let mut series =
                vec![pct_series(h, &format!("{id}/encode"), "Encode", Tint::Encoder, false)];
            if !g.encode_decode_shared {
                series.push(pct_series(h, &format!("{id}/decode"), "Decode", Tint::Decoder, true));
            }
            Some(Graph {
                id: "codec".into(),
                title: if g.encode_decode_shared { "Video encode/decode".into() } else { "Video engines".into() },
                max_label: "100%".into(),
                series,
                size: "half",
            })
        }
        3 => {
            let max = g.max_power_draw_watts.unwrap_or_else(|| {
                nice_ceiling(h.max(&format!("{id}/power")).max(10.0) as f64) as f32
            });
            Some(Graph {
                id: "power".into(),
                title: "Power draw".into(),
                max_label: units::watts(max),
                series: vec![Series {
                    label: "Power".into(),
                    tint: Tint::Gpu,
                    values: norm(h.values(&format!("{id}/power")), max),
                    dashed: false,
                    current: units::watts(g.power_draw_watts.unwrap_or(0.0)),
                }],
                size: "half",
            })
        }
        4 => Some(Graph {
            id: "temp".into(),
            title: "Temperature".into(),
            max_label: "100 °C".into(),
            series: vec![Series {
                label: "Temperature".into(),
                tint: Tint::Fan,
                values: norm(h.values(&format!("{id}/temp")), 100.0),
                dashed: false,
                current: units::celsius(g.temperature_c.unwrap_or(0.0)),
            }],
            size: "half",
        }),
        _ => None,
    }
}

fn gpu_detail(index: usize, g: &Gpu, h: &Histories, s: &Settings) -> DeviceDetail {
    let id = gpu_id(&g.id);
    let mut graphs = vec![Graph {
        id: "usage".into(),
        title: "Utilization".into(),
        max_label: "100%".into(),
        series: vec![pct_series(h, &format!("{id}/usage"), "Utilization", Tint::Gpu, false)],
        size: "hero",
    }];
    for slot in [s.gpu_graph_middle, s.gpu_graph_bottom] {
        if let Some(gr) = gpu_graph(slot, &id, g, h, s) {
            if !graphs.iter().any(|x| x.id == gr.id) {
                graphs.push(gr);
            }
        }
    }

    let mem_style = s.memory_style();
    let mut live = vec![primary(
        "Utilization",
        units::percent_int(g.utilization_percent.unwrap_or(0.0)),
        Tint::Gpu,
    )];
    if let (Some(u), Some(t)) = (g.used_memory, g.total_memory) {
        live.push(primary(
            "Memory",
            format!("{} / {}", units::bytes(u as f64, mem_style), units::bytes(t as f64, mem_style)),
            Tint::GpuMemory,
        ));
    }
    if let (Some(u), Some(t)) = (g.used_shared_memory, g.total_shared_memory) {
        live.push(stat(
            "Shared memory",
            format!("{} / {}", units::bytes(u as f64, mem_style), units::bytes(t as f64, mem_style)),
        ));
    }
    if let Some(e) = g.encoder_percent {
        live.push(stat(if g.encode_decode_shared { "Video codec" } else { "Encoder" }, units::percent_int(e)));
    }
    if !g.encode_decode_shared {
        if let Some(d) = g.decoder_percent {
            live.push(stat("Decoder", units::percent_int(d)));
        }
    }
    if let Some(c) = g.clock_speed_mhz {
        live.push(stat(
            "Clock speed",
            match g.max_clock_speed_mhz {
                Some(m) => format!("{} / {}", units::mhz(c as u64), units::mhz(m as u64)),
                None => units::mhz(c as u64),
            },
        ));
    }
    if let Some(c) = g.memory_speed_mhz {
        live.push(stat(
            "Memory speed",
            match g.max_memory_speed_mhz {
                Some(m) => format!("{} / {}", units::mhz(c as u64), units::mhz(m as u64)),
                None => units::mhz(c as u64),
            },
        ));
    }
    if let Some(p) = g.power_draw_watts {
        live.push(stat(
            "Power draw",
            match g.max_power_draw_watts {
                Some(m) => format!("{} / {}", units::watts(p), units::watts(m)),
                None => units::watts(p),
            },
        ));
    }
    if let Some(t) = g.temperature_c {
        live.push(stat("Temperature", units::celsius(t)));
    }
    if let Some(f) = g.fan_speed_percent {
        live.push(stat("Fan speed", units::percent_int(f)));
    }

    let mut info = vec![];
    let ver = |v: &crate::magpie::types::gpus::ApiVersion| match v.patch {
        Some(p) => format!("{}.{}.{}", v.major, v.minor, p),
        None => format!("{}.{}", v.major, v.minor),
    };
    if let Some(v) = &g.opengl_version {
        let es = g.opengl_variant.is_some()
            && g.opengl_variant() == crate::magpie::types::gpus::OpenGlVariant::OpenGles;
        info.push(stat(if es { "OpenGL ES" } else { "OpenGL" }, ver(v)));
    }
    if let Some(v) = &g.vulkan_version {
        info.push(stat("Vulkan", ver(v)));
    }
    if let Some(v) = &g.opencl_version {
        info.push(stat("OpenCL", ver(v)));
    }
    if let Some(v) = &g.cuda_version {
        info.push(stat("CUDA", ver(v)));
    }
    if let (Some(gen), Some(l)) = (g.pcie_gen, g.pcie_lanes) {
        let max = match (g.max_pcie_gen, g.max_pcie_lanes) {
            (Some(mg), Some(ml)) => format!(" (max Gen {mg} ×{ml})"),
            _ => String::new(),
        };
        info.push(stat("PCI Express", format!("Gen {gen} ×{l}{max}")));
    }
    info.push(stat("PCI ID", format!("{:04x}:{:04x}", g.vendor_id, g.device_id)));
    info.push(stat("Device", g.id.clone()));

    DeviceDetail {
        id,
        kind: DeviceKind::Gpu,
        title: format!("GPU {index}"),
        model: g.device_name.clone().unwrap_or_else(|| "Graphics".into()),
        graphs,
        grid: vec![],
        stats: vec![group(None, live), group(Some("Graphics card"), info)],
        extras: vec![],
    }
}

fn fan_detail(f: &Fan, h: &Histories) -> DeviceDetail {
    let id = fan_id(f);
    let max = f
        .max_rpm
        .map(|m| m as f32)
        .unwrap_or_else(|| nice_ceiling(h.max(&format!("{id}/rpm")).max(1000.0) as f64) as f32);
    let mut graphs = vec![Graph {
        id: "rpm".into(),
        title: "Fan speed".into(),
        max_label: format!("{} RPM", units::count(max as u64)),
        series: vec![Series {
            label: "Speed".into(),
            tint: Tint::Fan,
            values: norm(h.values(&format!("{id}/rpm")), max),
            dashed: false,
            current: format!("{} RPM", units::count(f.rpm as u64)),
        }],
        size: "hero",
    }];
    if let Some(t) = f.temp_amount {
        graphs.push(Graph {
            id: "temp".into(),
            title: "Temperature".into(),
            max_label: "100 °C".into(),
            series: vec![Series {
                label: "Temperature".into(),
                tint: Tint::Kernel,
                values: norm(h.values(&format!("{id}/temp")), 100.0),
                dashed: false,
                current: units::milli_kelvin(t),
            }],
            size: "half",
        });
    }
    let mut live = vec![primary("Speed", format!("{} RPM", units::count(f.rpm as u64)), Tint::Fan)];
    if let Some(p) = f.pwm_percent {
        live.push(primary("PWM", units::percent_int(p), Tint::Fan));
    }
    if let Some(t) = f.temp_amount {
        live.push(stat("Temperature", units::milli_kelvin(t)));
    }
    if let Some(n) = &f.temp_name {
        live.push(stat("Sensor", n.clone()));
    }
    if let Some(m) = f.max_rpm {
        live.push(stat("Max speed", format!("{} RPM", units::count(m as u64))));
    }
    DeviceDetail {
        id,
        kind: DeviceKind::Fan,
        title: f.fan_label.clone().unwrap_or_else(|| format!("Fan {}", f.fan_index)),
        model: format!("hwmon{} · fan{}", f.hwmon_index, f.fan_index),
        graphs,
        grid: vec![],
        stats: vec![group(None, live)],
        extras: vec![],
    }
}

fn battery_detail(b: &Battery, h: &Histories) -> DeviceDetail {
    let id = battery_id(b);
    let graphs = vec![Graph {
        id: "charge".into(),
        title: "Charge".into(),
        max_label: "100%".into(),
        series: vec![pct_series(h, &format!("{id}/charge"), "Charge", Tint::Battery, false)],
        size: "hero",
    }];
    let mut live = vec![
        primary("Charge", units::percent_int(b.percentage), Tint::Battery),
        primary("State", battery_state(b), Tint::Battery),
    ];
    if let Some(p) = b.power {
        live.push(stat("Power", units::watts(p.abs())));
    }
    if let Some(t) = b.time_to_empty.filter(|t| *t > 0) {
        live.push(stat("Time remaining", units::duration_words(t as u64)));
    }
    if let Some(t) = b.time_to_full.filter(|t| *t > 0) {
        live.push(stat("Time to full", units::duration_words(t as u64)));
    }
    if let Some(v) = b.voltage {
        live.push(stat("Voltage", format!("{v:.2} V")));
    }
    if let Some(t) = b.temp {
        live.push(stat("Temperature", units::celsius(t)));
    }

    let mut info = vec![];
    if let Some(v) = &b.vendor {
        info.push(stat("Vendor", v.clone()));
    }
    if let Some(m) = &b.model {
        info.push(stat("Model", m.clone()));
    }
    if let Some(sn) = &b.serial {
        info.push(stat("Serial", sn.clone()));
    }
    if b.technology.is_some() {
        info.push(stat("Technology", b.technology().as_str_name()));
    }
    if let Some(c) = b.capacity {
        info.push(stat("Health", units::percent_int(c)));
    }
    if let (Some(full), Some(design)) = (b.energy_full, b.energy_full_design) {
        info.push(stat(
            "Energy capacity",
            format!("{:.1} Wh / {:.1} Wh design", full as f32 / 1e3, design as f32 / 1e3),
        ));
    }
    if let Some(c) = b.charge_cycles {
        info.push(stat("Charge cycles", c.to_string()));
    }
    if b.charge_threshold_enabled {
        info.push(stat("Charge limit", "Enabled"));
    }

    DeviceDetail {
        id,
        kind: DeviceKind::Battery,
        title: "Battery".into(),
        model: b.model.clone().unwrap_or_else(|| b.name.clone()),
        graphs,
        grid: vec![],
        stats: vec![group(None, live), group(Some("Battery"), info)],
        extras: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_ceiling_steps() {
        assert_eq!(nice_ceiling(0.0), 1.0);
        assert_eq!(nice_ceiling(7.0), 10.0);
        assert_eq!(nice_ceiling(120.0), 200.0);
        assert_eq!(nice_ceiling(2100.0), 2500.0);
        assert_eq!(nice_ceiling(4900.0), 5000.0);
    }
}
