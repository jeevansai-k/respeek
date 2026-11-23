/* readings.rs — one sample of everything Magpie reports.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::collections::HashMap;

use crate::magpie::types::processes::processes_response::process_map::NetworkStatsError;
use crate::magpie::{
    App, Battery, Client, Connection, Cpu, Disk, Fan, Gpu, Memory, MemoryDevice, Process, Service,
};

#[derive(Debug, Default)]
pub struct Readings {
    pub cpu: Cpu,
    pub memory: Memory,
    pub memory_devices: Vec<MemoryDevice>,
    pub disks: Vec<Disk>,
    pub connections: HashMap<String, Connection>,
    pub gpus: HashMap<String, Gpu>,
    pub fans: Vec<Fan>,
    pub batteries: Vec<Battery>,

    pub apps: HashMap<String, App>,
    pub processes: HashMap<u32, Process>,
    pub network_stats_error: Option<NetworkStatsError>,

    pub user_services: HashMap<u64, Service>,
    pub system_services: HashMap<u64, Service>,
}

/// What the current page needs beyond the always-on device data.
#[derive(Debug, Clone, Copy, Default)]
pub struct Needs {
    pub processes: bool,
    pub services: bool,
    pub memory_devices: bool,
    /// Expensive collectors (GPUs wake the dGPU via NVML; fans hit slow SMM/hwmon chips;
    /// batteries go through sysfs + UPower). The sampler polls them at a lower cadence
    /// unless their page is open. Each Magpie request triggers a re-collection, so skipping
    /// a request skips the work.
    pub gpus: bool,
    pub fans: bool,
    pub batteries: bool,
}

impl Needs {
    /// Cadence policy for a given tick and visible device.
    pub fn for_tick(tick: u64, device_page: Option<&str>, overview: bool) -> Self {
        let showing = |prefix: &str| overview || device_page.is_some_and(|d| d.starts_with(prefix));
        Self {
            gpus: showing("gpu:") || tick % 2 == 0,
            fans: showing("fan:") || tick % 3 == 0,
            batteries: showing("battery:") || tick % 5 == 0,
            ..Default::default()
        }
    }
}

impl Readings {
    /// Refresh in place. Process/service data is kept from the previous sample when not needed,
    /// so switching pages shows the last known rows instantly.
    pub fn refresh(&mut self, client: &Client, needs: Needs) {
        // `RUST_LOG=respeek::timing=debug` prints how long Magpie takes for each request.
        macro_rules! timed {
            ($name:literal, $e:expr) => {{
                let t = std::time::Instant::now();
                let v = $e;
                log::debug!(target: "respeek::timing", "{:<16} {:>7.2} ms", $name, t.elapsed().as_secs_f64() * 1e3);
                v
            }};
        }
        self.cpu = timed!("cpu", client.cpu());
        self.memory = timed!("memory", client.memory());
        if needs.memory_devices {
            self.memory_devices = timed!("memory_devices", client.memory_devices());
        }
        self.disks = timed!("disks", client.disks_info());
        self.connections = timed!("network", client.network_connections());
        if needs.gpus || self.gpus.is_empty() {
            self.gpus = timed!("gpus", client.gpus());
        }
        if needs.fans || self.fans.is_empty() {
            self.fans = timed!("fans", client.fans_info());
        }
        if needs.batteries || self.batteries.is_empty() {
            self.batteries = timed!("batteries", client.batteries_info());
        }

        if needs.processes || needs.services {
            let (processes, err) = timed!("processes", client.processes());
            self.processes = processes;
            self.network_stats_error = err;
        }
        if needs.processes {
            self.apps = timed!("apps", client.apps());
        }
        if needs.services {
            self.user_services = timed!("user_services", client.user_services());
            self.system_services = timed!("system_services", client.system_services());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Needs;

    #[test]
    fn cadence_backs_off_unless_visible() {
        let polled = |dev: Option<&str>, f: fn(&Needs) -> bool| (0..30).filter(|t| f(&Needs::for_tick(*t, dev, false))).count();
        assert_eq!(polled(Some("cpu"), |n| n.gpus), 15);
        assert_eq!(polled(Some("cpu"), |n| n.fans), 10);
        assert_eq!(polled(Some("cpu"), |n| n.batteries), 6);
        assert_eq!(polled(Some("gpu:0000:01:00.0"), |n| n.gpus), 30);
        assert_eq!(polled(Some("fan:4-1"), |n| n.fans), 30);
        assert!(Needs::for_tick(1, None, true).batteries, "overview shows everything live");
    }
}
