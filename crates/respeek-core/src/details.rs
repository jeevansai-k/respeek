/* details.rs — view-models for sheets: process, service, SMART, eject failure, about system.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use base64::Engine as _;
use serde::Serialize;

use crate::magpie::types::about::About;
use crate::magpie::types::disks::smart_data::{ata::PrettyUnit, Data};
use crate::magpie::{ErrorEjectFailed, Service, SmartData};
use crate::model::*;
use crate::readings::Readings;
use crate::settings::Settings;
use crate::tables::pretty_process_name;
use crate::units::{self, ByteStyle};

pub fn process_details(
    pid: u32,
    r: &Readings,
    s: &Settings,
    icon: Option<String>,
) -> Option<ProcessDetails> {
    let p = r.processes.get(&pid)?;
    let st = &p.usage_stats;
    let merged = p.merged_usage_stats(&r.processes);
    let mem = s.memory_style();
    let parent = r
        .processes
        .get(&p.parent)
        .map(|pp| format!("{} ({})", pretty_process_name(pp), pp.pid))
        .unwrap_or_else(|| p.parent.to_string());

    Some(ProcessDetails {
        pid,
        name: pretty_process_name(p),
        icon,
        state: format!("{:?}", p.state()),
        command_line: p.cmd.join(" "),
        exe: p.exe.clone(),
        groups: vec![
            group(
                None,
                vec![
                    primary("CPU", units::percent(st.cpu_usage), Tint::Cpu),
                    primary("Memory", units::bytes(st.memory_usage as f64, mem), Tint::Memory),
                    primary(
                        "Drive",
                        units::rate(st.disk_usage as f64, s.drive_style()),
                        Tint::Disk,
                    ),
                    primary(
                        "Network",
                        units::rate(st.network_usage as f64, s.network_style()),
                        Tint::Network,
                    ),
                ],
            ),
            group(
                Some("Process"),
                vec![
                    stat("PID", pid.to_string()),
                    stat("Parent", parent),
                    stat("State", format!("{:?}", p.state())),
                    stat("Threads", units::count(p.task_count)),
                    stat("Child processes", p.children.len().to_string()),
                    stat("Swap", units::bytes(st.swap_usage as f64, mem)),
                    stat("GPU", units::percent(st.gpu_usage)),
                    stat("GPU memory", units::bytes(st.gpu_memory_usage as f64, ByteStyle::MEMORY)),
                ],
            ),
            group(
                Some("Including children"),
                vec![
                    stat("CPU", units::percent(merged.cpu_usage)),
                    stat("Memory", units::bytes(merged.memory_usage as f64, mem)),
                    stat("Drive", units::rate(merged.disk_usage as f64, s.drive_style())),
                    stat("Network", units::rate(merged.network_usage as f64, s.network_style())),
                ],
            ),
        ],
    })
}

pub fn service_details(svc: &Service, logs: String) -> ServiceDetails {
    let status = if svc.running {
        "Running"
    } else if svc.failed {
        "Failed"
    } else if svc.enabled {
        "Stopped"
    } else {
        "Inactive"
    };
    let mut stats = vec![
        stat("Status", status),
        stat("Startup", if svc.enabled { "Enabled" } else { "Disabled" }),
    ];
    if let Some(pid) = svc.pid {
        stats.push(stat("PID", pid.to_string()));
    }
    if let Some(u) = &svc.user {
        stats.push(stat("User", u.clone()));
    }
    if let Some(g) = &svc.group {
        stats.push(stat("Group", g.clone()));
    }
    if let Some(f) = &svc.file_path {
        stats.push(stat("Unit file", f.clone()));
    }
    ServiceDetails {
        id: svc.id,
        name: svc.name.clone(),
        description: svc.description.clone().unwrap_or_default(),
        groups: vec![group(None, stats)],
        logs,
    }
}

fn pretty_ata(value: i64, unit: PrettyUnit) -> String {
    match unit {
        PrettyUnit::Milliseconds => units::long_duration((value / 1000).max(0) as u64),
        PrettyUnit::Sectors => format!("{} sectors", units::count(value.max(0) as u64)),
        PrettyUnit::Millikelvin => units::milli_kelvin(value.max(0) as u32),
        _ => units::count(value.max(0) as u64),
    }
}

pub fn smart_view(title: &str, data: Option<SmartData>) -> SmartView {
    let Some(d) = data else {
        return SmartView {
            title: title.into(),
            available: false,
            groups: vec![],
            attributes_header: vec![],
            attributes: vec![],
        };
    };
    let result = if d.test_result.is_some() { format!("{:?}", d.test_result()) } else { "Unknown".into() };
    let mut groups = vec![group(
        None,
        vec![
            primary("Last self-test", result, Tint::Disk),
            stat("Powered on", units::long_duration(d.powered_on_seconds)),
        ],
    )];
    let mut header = vec![];
    let mut rows = vec![];
    match d.data {
        Some(Data::Ata(ata)) => {
            header = ["ID", "Attribute", "Value", "Normalized", "Worst", "Threshold"]
                .map(String::from)
                .to_vec();
            for a in ata.attributes {
                let unit = PrettyUnit::try_from(a.pretty_unit).unwrap_or(PrettyUnit::Unknown);
                rows.push(vec![
                    a.id.to_string(),
                    a.name,
                    pretty_ata(a.pretty, unit),
                    a.value.to_string(),
                    a.worst.to_string(),
                    a.threshold.to_string(),
                ]);
            }
        }
        Some(Data::Nvme(n)) => {
            let b = |v: u64| units::bytes(v as f64, ByteStyle::BYTES_SI);
            let mut st = vec![];
            let mut push = |label: &str, v: Option<String>| {
                if let Some(v) = v {
                    st.push(stat(label, v));
                }
            };
            push("Available spare", n.avail_spare.map(|v| format!("{v}%")));
            push("Spare threshold", n.spare_thresh.map(|v| format!("{v}%")));
            push("Percentage used", n.percent_used.map(|v| format!("{v}%")));
            push("Data read", n.total_data_read.map(b));
            push("Data written", n.total_data_written.map(b));
            push("Controller busy", n.ctrl_busy_minutes.map(|m| units::long_duration(m * 60)));
            push("Power cycles", n.power_cycles.map(units::count));
            push("Unsafe shutdowns", n.unsafe_shutdowns.map(units::count));
            push("Media errors", n.media_errors.map(units::count));
            push("Error log entries", n.num_err_log_entries.map(units::count));
            for (i, t) in n.temp_sensors.iter().enumerate() {
                push(&format!("Temperature sensor {i}"), Some(units::milli_kelvin(*t)));
            }
            groups.push(group(Some("NVMe health"), st));
        }
        None => {}
    }
    SmartView { title: title.into(), available: true, groups, attributes_header: header, attributes: rows }
}

#[derive(Debug, Clone, Serialize)]
pub struct EjectFailure {
    pub blockers: Vec<EjectBlocker>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EjectBlocker {
    pub pid: u32,
    pub name: String,
    pub icon: Option<String>,
    pub files: Vec<String>,
}

pub fn eject_failure(e: ErrorEjectFailed, r: &Readings) -> EjectFailure {
    EjectFailure {
        blockers: e
            .blockers
            .into_iter()
            .map(|b| EjectBlocker {
                name: r
                    .processes
                    .get(&b.pid)
                    .map(pretty_process_name)
                    .unwrap_or_else(|| format!("Process {}", b.pid)),
                pid: b.pid,
                icon: None,
                files: b.files.into_iter().chain(b.dirs).collect(),
            })
            .collect(),
    }
}

pub fn about_system(a: About) -> AboutSystem {
    let os = a.os_info;
    let de = a.de_info;
    let dev = a.device_info;
    let logo = os.logo.as_ref().filter(|l| !l.is_empty()).map(|bytes| {
        let mime = if bytes.starts_with(b"<") { "image/svg+xml" } else { "image/png" };
        format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
    });
    let opt = |label: &str, v: Option<String>| v.filter(|v| !v.is_empty()).map(|v| stat(label, v));
    let device: Vec<Stat> = [
        opt("Hostname", dev.hostname),
        opt("Vendor", dev.vendor),
        opt("Model", dev.model),
    ]
    .into_iter()
    .flatten()
    .collect();
    let system: Vec<Stat> = [
        opt("OS", os.pretty_name.clone().or(os.name.clone())),
        opt("Version", os.version),
        opt("Architecture", os.os_architecture),
        opt("Kernel", os.kernel_release),
        opt("Package manager", os.package_manager.map(|p| match os.package_manager_version {
            Some(v) => format!("{p} {v}"),
            None => p,
        })),
    ]
    .into_iter()
    .flatten()
    .collect();
    let desktop: Vec<Stat> = [
        opt("Desktop", de.desktop_environment.map(|d| match de.version.clone() {
            Some(v) if !v.is_empty() => format!("{d} {v}"),
            _ => d,
        })),
        opt("Windowing system", de.windowing_system),
        opt("Session type", de.session_type),
    ]
    .into_iter()
    .flatten()
    .collect();
    AboutSystem {
        logo,
        title: os.pretty_name.or(os.name).unwrap_or_else(|| "Linux".into()),
        groups: vec![
            group(Some("Device"), device),
            group(Some("System"), system),
            group(Some("Desktop"), desktop),
        ]
        .into_iter()
        .filter(|g| !g.stats.is_empty())
        .collect(),
    }
}
