/* engine.rs — owns the Magpie client, the sampling thread, UI state and every user intent.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::details::{self, EjectFailure};
use crate::devices;
use crate::history::Histories;
use crate::icons::IconCache;
use crate::magpie::{Client, ScriptRunResult, ScriptStatus};
use crate::model::*;
use crate::readings::{Needs, Readings};
use crate::settings::{Settings, SortDirection};
use crate::tables::{self, TableState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Page {
    Performance,
    Apps,
    Services,
}

impl Page {
    fn parse(s: &str) -> Self {
        match s {
            "apps" => Page::Apps,
            "services" => Page::Services,
            _ => Page::Performance,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Page::Performance => "performance",
            Page::Apps => "apps",
            Page::Services => "services",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Signal {
    Terminate,
    Kill,
    Interrupt,
    Hangup,
    Suspend,
    Continue,
    User1,
    User2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableId {
    Apps,
    Services,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SetupScriptState {
    Unavailable { reason: String },
    Idle { file: String, command: String, revert_command: String },
    Running,
    Finished { ok: bool, message: Option<String> },
}

struct UiState {
    page: Page,
    device: String,
    overview: bool,
    apps: TableState,
    services: TableState,
    services_user: bool,
}

struct Data {
    readings: Readings,
    histories: Histories,
    settings: Settings,
    ui: UiState,
    icons: IconCache,
    seq: u64,
    have_processes: bool,
    have_services: bool,
    have_memory_devices: bool,
}

type Sink = Box<dyn Fn(&Snapshot) + Send + Sync>;

struct Inner {
    client: OnceLock<Client>,
    data: Mutex<Data>,
    sink: Sink,
    wake: (Mutex<bool>, Condvar),
    running: AtomicBool,
    paused: AtomicBool,
    last_settings_save: AtomicU64,
}

#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

impl Engine {
    /// Create the engine and start sampling on a background thread.
    /// `sink` receives every snapshot (sampler ticks and intent-driven rebuilds).
    pub fn start(sink: impl Fn(&Snapshot) + Send + Sync + 'static) -> Self {
        let settings = Settings::load();
        let ui = UiState {
            page: Page::parse(&settings.selected_page),
            device: settings.selected_device.clone(),
            overview: false,
            apps: TableState::default(),
            services: TableState::default(),
            services_user: false,
        };
        let mut apps = TableState::default();
        apps.selected = None;
        let inner = Arc::new(Inner {
            client: OnceLock::new(),
            data: Mutex::new(Data {
                readings: Readings::default(),
                histories: Histories::new(settings.data_points as usize),
                settings,
                ui,
                icons: IconCache::new(),
                seq: 0,
                have_processes: false,
                have_services: false,
                have_memory_devices: false,
            }),
            sink: Box::new(sink),
            wake: (Mutex::new(false), Condvar::new()),
            running: AtomicBool::new(true),
            paused: AtomicBool::new(false),
            last_settings_save: AtomicU64::new(0),
        });

        let thread_inner = inner.clone();
        std::thread::Builder::new()
            .name("respeek-sampler".into())
            .spawn(move || sampler(thread_inner))
            .expect("failed to spawn sampler thread");

        Self { inner }
    }

    fn data(&self) -> std::sync::MutexGuard<'_, Data> {
        self.inner.data.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn client(&self) -> Option<&Client> {
        self.inner.client.get()
    }

    fn wake(&self) {
        let (lock, cv) = &self.inner.wake;
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = true;
        cv.notify_all();
    }

    /// Rebuild from cached readings and push to the sink immediately (no Magpie round-trip).
    fn emit(&self) {
        let snap = {
            let mut d = self.data();
            d.seq += 1;
            build_snapshot(&d, self.inner.paused.load(Ordering::Relaxed))
        };
        (self.inner.sink)(&snap);
    }

    pub fn snapshot(&self) -> Snapshot {
        build_snapshot(&self.data(), self.inner.paused.load(Ordering::Relaxed))
    }

    pub fn shutdown(&self) {
        self.inner.running.store(false, Ordering::Relaxed);
        self.wake();
        let _ = self.data().settings.save();
        if let Some(c) = self.client() {
            c.stop();
        }
    }

    // ----- navigation ------------------------------------------------------------------------

    pub fn set_page(&self, page: Page) {
        let needs_fetch = {
            let mut d = self.data();
            d.ui.page = page;
            d.settings.selected_page = page.as_str().into();
            match page {
                Page::Apps => !d.have_processes,
                Page::Services => !d.have_services,
                Page::Performance => false,
            }
        };
        self.emit();
        if needs_fetch {
            self.wake();
        }
    }

    pub fn select_device(&self, id: &str) {
        let needs_fetch = {
            let mut d = self.data();
            d.ui.page = Page::Performance;
            d.ui.overview = false;
            d.ui.device = id.to_owned();
            d.settings.selected_device = id.to_owned();
            d.settings.selected_page = "performance".into();
            false
        };
        self.emit();
        if needs_fetch {
            self.wake();
        }
    }

    pub fn set_overview(&self, on: bool) {
        {
            let mut d = self.data();
            d.ui.page = Page::Performance;
            d.ui.overview = on;
        }
        self.emit();
    }

    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::Relaxed);
        self.emit();
        if !paused {
            self.wake();
        }
    }

    pub fn refresh_now(&self) {
        self.wake();
    }

    // ----- tables ----------------------------------------------------------------------------

    fn with_table<R>(&self, t: TableId, f: impl FnOnce(&mut TableState, &mut Settings) -> R) -> R {
        let r = {
            let mut d = self.data();
            let Data { ui, settings, .. } = &mut *d;
            match t {
                TableId::Apps => f(&mut ui.apps, settings),
                TableId::Services => f(&mut ui.services, settings),
            }
        };
        self.emit();
        r
    }

    pub fn set_search(&self, t: TableId, q: &str) {
        self.with_table(t, |s, _| s.search = q.to_owned());
    }

    pub fn toggle_row(&self, t: TableId, key: &str) {
        let is_section = matches!(key, "apps" | "procs");
        self.with_table(t, |s, _| s.toggle(key, is_section));
    }

    pub fn select_row(&self, t: TableId, key: Option<String>) {
        self.with_table(t, |s, _| s.selected = key);
    }

    pub fn sort_by(&self, t: TableId, column: &str) {
        self.with_table(t, |_, settings| {
            let (col, dir) = match t {
                TableId::Apps => (&mut settings.apps_sort_column, &mut settings.apps_sort_direction),
                TableId::Services => {
                    (&mut settings.services_sort_column, &mut settings.services_sort_direction)
                }
            };
            if col == column {
                *dir = match *dir {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
            } else {
                *col = column.to_owned();
                *dir = if column == "name" || column == "pid" {
                    SortDirection::Ascending
                } else {
                    SortDirection::Descending
                };
            }
        });
    }

    pub fn collapse_all(&self, t: TableId) {
        self.with_table(t, |s, _| {
            s.expanded.clear();
            s.collapsed.clear();
        });
    }

    /// Plain-text dump of a device page (upstream "Copy Device Information").
    pub fn device_info_text(&self, id: &str) -> Option<String> {
        let d = self.data();
        let detail = devices::detail(id, &d.readings, &d.histories, &d.settings)?;
        let mut out = format!("{}\n{}\n", detail.title, detail.model);
        for g in &detail.stats {
            out.push('\n');
            if let Some(t) = &g.title {
                out.push_str(&format!("{t}\n"));
            }
            for st in &g.stats {
                out.push_str(&format!("{}: {}\n", st.label, st.value));
            }
        }
        Some(out)
    }

    pub fn set_services_user(&self, user: bool) {
        {
            let mut d = self.data();
            d.ui.services_user = user;
            d.ui.services.selected = None;
        }
        self.emit();
    }

    // ----- settings --------------------------------------------------------------------------

    pub fn settings(&self) -> Settings {
        self.data().settings.clone()
    }

    pub fn update_settings(&self, patch: serde_json::Value) -> Result<Settings, String> {
        let (s, scale_changed) = {
            let mut d = self.data();
            let before = d.settings.core_count_affects_percentages;
            d.settings.apply_patch(patch)?;
            d.histories.cap = d.settings.data_points as usize;
            (d.settings.clone(), before != d.settings.core_count_affects_percentages)
        };
        if scale_changed {
            if let Some(c) = self.client() {
                c.set_scale_cpu_usage_to_core_count(s.core_count_affects_percentages);
            }
        }
        let _ = s.save();
        self.emit();
        self.wake();
        Ok(s)
    }

    pub fn reset_settings(&self) -> Settings {
        let s = {
            let mut d = self.data();
            let keep_first_run = d.settings.first_time_running;
            d.settings = Settings { first_time_running: keep_first_run, ..Settings::default() };
            d.histories.cap = d.settings.data_points as usize;
            d.settings.clone()
        };
        let _ = s.save();
        self.emit();
        s
    }

    /// Persist window geometry; throttled so resize storms don't hammer the disk.
    pub fn save_window(&self, width: i32, height: i32, maximized: bool) {
        let mut d = self.data();
        d.settings.window_width = width;
        d.settings.window_height = height;
        d.settings.is_maximized = maximized;
        d.settings.sanitize();
        let now = Instant::now().elapsed().as_secs();
        self.inner.last_settings_save.store(now, Ordering::Relaxed);
        let _ = d.settings.save();
    }

    // ----- actions ---------------------------------------------------------------------------

    pub fn signal(&self, pids: Vec<u32>, signal: Signal) {
        let Some(c) = self.client() else { return };
        match signal {
            Signal::Terminate => c.terminate_processes(pids),
            Signal::Kill => c.kill_processes(pids),
            Signal::Interrupt => c.interrupt_processes(pids),
            Signal::Hangup => c.hangup_processes(pids),
            Signal::Suspend => c.suspend_processes(pids),
            Signal::Continue => c.continue_processes(pids),
            Signal::User1 => c.signal_user_one_processes(pids),
            Signal::User2 => c.signal_user_two_processes(pids),
        }
        self.wake();
    }

    pub fn service_action(&self, id: u64, action: ServiceAction) {
        let Some(c) = self.client() else { return };
        match action {
            ServiceAction::Start => c.start_service(id),
            ServiceAction::Stop => c.stop_service(id),
            ServiceAction::Restart => c.restart_service(id),
            ServiceAction::Enable => c.enable_service(id),
            ServiceAction::Disable => c.disable_service(id),
        }
        self.wake();
    }

    pub fn process_details(&self, pid: u32) -> Option<ProcessDetails> {
        let d = self.data();
        let icon = d
            .readings
            .apps
            .values()
            .find(|a| {
                let mut set = std::collections::HashSet::new();
                for p in &a.pids {
                    tables::collect_process_subtree(&d.readings.processes, *p, &mut set);
                }
                set.contains(&pid)
            })
            .and_then(|a| d.icons.get(&a.id));
        details::process_details(pid, &d.readings, &d.settings, icon)
    }

    pub fn service_details(&self, id: u64) -> Option<ServiceDetails> {
        let svc = {
            let d = self.data();
            d.readings
                .user_services
                .get(&id)
                .or_else(|| d.readings.system_services.get(&id))
                .cloned()?
        };
        let logs = self
            .client()
            .map(|c| c.service_logs(id, svc.pid.and_then(NonZeroU32::new)))
            .unwrap_or_default();
        Some(details::service_details(&svc, logs))
    }

    pub fn smart(&self, device_id: &str) -> SmartView {
        let disk_id = device_id.trim_start_matches("disk:").to_owned();
        let title = {
            let d = self.data();
            d.readings
                .disks
                .iter()
                .find(|x| x.id == disk_id)
                .and_then(|x| x.model.clone())
                .unwrap_or_else(|| disk_id.clone())
        };
        let data = self.client().and_then(|c| c.smart_data(disk_id));
        details::smart_view(&title, data)
    }

    pub fn eject(&self, device_id: &str) -> Result<(), EjectFailure> {
        let disk_id = device_id.trim_start_matches("disk:").to_owned();
        let Some(c) = self.client() else { return Ok(()) };
        let res = c.eject_disk(disk_id);
        self.wake();
        res.map_err(|e| details::eject_failure(e, &self.data().readings))
    }

    pub fn about_system(&self) -> Option<AboutSystem> {
        self.client().map(|c| details::about_system(c.about()))
    }

    pub fn setup_script_state(&self) -> SetupScriptState {
        let Some(c) = self.client() else {
            return SetupScriptState::Unavailable { reason: "Still connecting to Magpie".into() };
        };
        match c.setup_script_status() {
            Some(ScriptStatus::Running) => SetupScriptState::Running,
            Some(ScriptStatus::Finished(res)) => match res {
                ScriptRunResult::Ok => SetupScriptState::Finished { ok: true, message: None },
                ScriptRunResult::WithWarnings(w) => {
                    SetupScriptState::Finished { ok: true, message: Some(w) }
                }
                ScriptRunResult::Err(e) => SetupScriptState::Finished { ok: false, message: Some(e) },
            },
            _ => match c.setup_script_info() {
                Ok(n) => SetupScriptState::Idle {
                    file: n.file,
                    command: n.command,
                    revert_command: n.revert_command,
                },
                Err(e) => SetupScriptState::Unavailable { reason: e },
            },
        }
    }

    pub fn setup_script_run(&self, revert: bool) -> Result<(), String> {
        self.client().ok_or("Magpie not connected")?.setup_script_start(revert)
    }

    pub fn setup_script_open(&self) -> Result<(), String> {
        self.client().ok_or("Magpie not connected")?.setup_script_open()
    }

    pub fn finish_first_run(&self) {
        let _ = self.update_settings(serde_json::json!({ "first_time_running": false }));
    }
}

fn build_snapshot(d: &Data, paused: bool) -> Snapshot {
    let sidebar = devices::summaries(&d.readings, &d.histories, &d.settings);
    let page = match d.ui.page {
        Page::Performance => {
            let id = if sidebar.iter().any(|s| s.id == d.ui.device) {
                d.ui.device.clone()
            } else {
                sidebar.first().map(|s| s.id.clone()).unwrap_or_else(|| "cpu".into())
            };
            PageData::Performance {
                detail: (!d.ui.overview)
                    .then(|| devices::detail(&id, &d.readings, &d.histories, &d.settings))
                    .flatten(),
                overview: d.ui.overview,
            }
        }
        Page::Apps => PageData::Apps {
            table: tables::apps_table(&d.readings, &d.settings, &d.ui.apps, &d.icons),
        },
        Page::Services => PageData::Services {
            table: tables::services_table(
                &d.readings,
                &d.settings,
                &d.ui.services,
                d.ui.services_user,
                &d.icons,
            ),
            user: d.ui.services_user,
        },
    };
    Snapshot {
        seq: d.seq,
        interval_ms: (d.settings.interval_secs() * 1000.0) as u64,
        sidebar,
        page,
        settings: d.settings.clone(),
        paused,
    }
}

fn sampler(inner: Arc<Inner>) {
    let client = Client::new();
    client.start();
    {
        let d = inner.data.lock().unwrap_or_else(|e| e.into_inner());
        client.set_scale_cpu_usage_to_core_count(d.settings.core_count_affects_percentages);
    }
    // Ignore if already set (never happens: only this thread sets it).
    let _ = inner.client.set(client);
    let client = inner.client.get().expect("client just set");

    let mut tick: u64 = 0;
    while inner.running.load(Ordering::Relaxed) {
        let started = Instant::now();
        let (interval, page, needs_devices, device_page, overview) = {
            let d = inner.data.lock().unwrap_or_else(|e| e.into_inner());
            (
                Duration::from_secs_f64(d.settings.interval_secs()),
                d.ui.page,
                // Memory modules (SMBIOS) never change: fetch once up front so the Memory page
                // never lays out twice mid-transition on its first visit.
                !d.have_memory_devices,
                (d.ui.page == Page::Performance && !d.ui.overview).then(|| d.ui.device.clone()),
                d.ui.page == Page::Performance && d.ui.overview,
            )
        };

        if !inner.paused.load(Ordering::Relaxed) {
            // Services change rarely; refresh them every 4th tick unless on the Services page.
            tick = tick.wrapping_add(1);
            let needs = Needs {
                processes: page == Page::Apps,
                services: page == Page::Services || tick % 4 == 0 && page == Page::Apps,
                memory_devices: needs_devices,
                ..Needs::for_tick(tick, device_page.as_deref(), overview)
            };

            let mut next = {
                let mut d = inner.data.lock().unwrap_or_else(|e| e.into_inner());
                std::mem::take(&mut d.readings)
            };
            next.refresh(client, needs);

            // Resolve icons for newly-seen apps outside the lock.
            let missing = {
                let d = inner.data.lock().unwrap_or_else(|e| e.into_inner());
                d.icons.missing(next.apps.keys())
            };
            let fetched = (!missing.is_empty()).then(|| client.app_icons(missing.clone()));

            let snap = {
                let mut d = inner.data.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(icons) = fetched {
                    d.icons.insert(icons, &missing);
                }
                d.have_processes |= needs.processes;
                d.have_services |= needs.services;
                d.have_memory_devices |= needs.memory_devices;
                devices::record(&mut d.histories, &next);
                d.readings = next;
                d.seq += 1;
                build_snapshot(&d, false)
            };
            (inner.sink)(&snap);
        }

        // Sleep until the next tick or an explicit wake-up.
        let (lock, cv) = &inner.wake;
        let mut woken = lock.lock().unwrap_or_else(|e| e.into_inner());
        let deadline = started + interval;
        while !*woken && inner.running.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let (g, _) = cv.wait_timeout(woken, deadline - now).unwrap_or_else(|e| e.into_inner());
            woken = g;
        }
        *woken = false;
    }
}
