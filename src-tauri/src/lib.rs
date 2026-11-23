// Respeek — Tauri shell. Thin bridge between the web UI and `respeek-core`.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use respeek_core::details::EjectFailure;
use respeek_core::model::{AboutSystem, ProcessDetails, ServiceDetails, SmartView};
use respeek_core::{Engine, Page, ServiceAction, SetupScriptState, Settings, Signal, Snapshot, TableId};
use tauri::ipc::Channel;
use tauri::{Emitter, Manager, State, WindowEvent};

type Subscriber = Arc<Mutex<Option<Channel<Snapshot>>>>;

struct AppState {
    engine: Engine,
    subscriber: Subscriber,
}

static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
async fn subscribe(state: State<'_, AppState>, on_snapshot: Channel<Snapshot>) -> Result<Snapshot, String> {
    *state.subscriber.lock().unwrap_or_else(|e| e.into_inner()) = Some(on_snapshot);
    Ok(state.engine.snapshot())
}

#[tauri::command]
async fn set_page(state: State<'_, AppState>, page: Page) -> Result<(), String> {
    state.engine.set_page(page);
    Ok(())
}

#[tauri::command]
async fn select_device(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.engine.select_device(&id);
    Ok(())
}

#[tauri::command]
async fn set_overview(state: State<'_, AppState>, on: bool) -> Result<(), String> {
    state.engine.set_overview(on);
    Ok(())
}

#[tauri::command]
async fn set_paused(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    state.engine.set_paused(paused);
    Ok(())
}

#[tauri::command]
async fn refresh_now(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.refresh_now();
    Ok(())
}

#[tauri::command]
async fn set_search(state: State<'_, AppState>, table: TableId, query: String) -> Result<(), String> {
    state.engine.set_search(table, &query);
    Ok(())
}

#[tauri::command]
async fn toggle_row(state: State<'_, AppState>, table: TableId, key: String) -> Result<(), String> {
    state.engine.toggle_row(table, &key);
    Ok(())
}

#[tauri::command]
async fn select_row(state: State<'_, AppState>, table: TableId, key: Option<String>) -> Result<(), String> {
    state.engine.select_row(table, key);
    Ok(())
}

#[tauri::command]
async fn sort_by(state: State<'_, AppState>, table: TableId, column: String) -> Result<(), String> {
    state.engine.sort_by(table, &column);
    Ok(())
}

#[tauri::command]
async fn collapse_all(state: State<'_, AppState>, table: TableId) -> Result<(), String> {
    state.engine.collapse_all(table);
    Ok(())
}

#[tauri::command]
async fn device_info_text(state: State<'_, AppState>, id: String) -> Result<Option<String>, String> {
    Ok(state.engine.device_info_text(&id))
}

#[tauri::command]
async fn set_services_user(state: State<'_, AppState>, user: bool) -> Result<(), String> {
    state.engine.set_services_user(user);
    Ok(())
}

#[tauri::command]
async fn update_settings(state: State<'_, AppState>, patch: serde_json::Value) -> Result<Settings, String> {
    state.engine.update_settings(patch)}

#[tauri::command]
async fn reset_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.engine.reset_settings())
}

#[tauri::command]
async fn signal_processes(state: State<'_, AppState>, pids: Vec<u32>, signal: Signal) -> Result<(), String> {
    state.engine.signal(pids, signal);
    Ok(())
}

#[tauri::command]
async fn service_action(state: State<'_, AppState>, id: u64, action: ServiceAction) -> Result<(), String> {
    state.engine.service_action(id, action);
    Ok(())
}

#[tauri::command]
async fn process_details(state: State<'_, AppState>, pid: u32) -> Result<Option<ProcessDetails>, ()> {
    Ok(state.engine.process_details(pid))
}

#[tauri::command]
async fn service_details(state: State<'_, AppState>, id: u64) -> Result<Option<ServiceDetails>, ()> {
    Ok(state.engine.service_details(id))
}

#[tauri::command]
async fn smart_data(state: State<'_, AppState>, device: String) -> Result<SmartView, ()> {
    Ok(state.engine.smart(&device))
}

#[tauri::command]
async fn eject_disk(state: State<'_, AppState>, device: String) -> Result<(), EjectFailure> {
    state.engine.eject(&device)
}

#[tauri::command]
async fn about_system(state: State<'_, AppState>) -> Result<Option<AboutSystem>, ()> {
    Ok(state.engine.about_system())
}

#[tauri::command]
async fn setup_script_state(state: State<'_, AppState>) -> Result<SetupScriptState, ()> {
    Ok(state.engine.setup_script_state())
}

#[tauri::command]
async fn setup_script_run(state: State<'_, AppState>, revert: bool) -> Result<(), String> {
    state.engine.setup_script_run(revert)
}

#[tauri::command]
async fn setup_script_open(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.setup_script_open()
}

#[tauri::command]
async fn finish_first_run(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.finish_first_run();
    Ok(())
}

/// Diagnostics from the UI (frame-timing tour) land in Respeek's log on stderr.
#[tauri::command]
async fn ui_log(line: String) -> Result<(), String> {
    eprintln!("{line}");
    Ok(())
}

#[tauri::command]
fn quit(app: tauri::AppHandle, state: State<'_, AppState>) {
    QUIT_REQUESTED.store(true, Ordering::Relaxed);
    state.engine.shutdown();
    app.exit(0);
}

/// Point Magpie at the bundled hardware-name database (PCI/USB/Wi-Fi vendor + model names).
fn locate_hw_db() {
    if std::env::var_os("MC_MAGPIE_HW_DB").is_some() {
        return;
    }
    let mut candidates = vec![];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("../lib/Respeek/resources/hw.db"));
            candidates.push(dir.join("../lib/respeek/resources/hw.db"));
            candidates.push(dir.join("resources/hw.db"));
        }
    }
    if cfg!(debug_assertions) {
        candidates.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/hw.db"));
    }
    if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
        std::env::set_var("MC_MAGPIE_HW_DB", found);
    }
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    // Note: if the window renders blank on an NVIDIA-only setup, launch with
    // WEBKIT_DISABLE_DMABUF_RENDERER=1. It is not forced here because that path copies every
    // frame through the CPU and makes glass/animation noticeably more expensive.

    locate_hw_db();

    let subscriber: Subscriber = Arc::new(Mutex::new(None));
    let sink_sub = subscriber.clone();
    let engine = Engine::start(move |snap: &Snapshot| {
        if let Some(ch) = sink_sub.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = ch.send(snap.clone());
        }
    });

    let initial = engine.settings();

    tauri::Builder::default()
        .manage(AppState { engine: engine.clone(), subscriber })
        .setup(move |app| {
            let handle = app.handle().clone();
            respeek_core::magpie::set_fatal_handler(move |msg| {
                let _ = handle.emit("fatal", msg.to_owned());
                // Give the UI a chance to show the error; it calls `quit` when dismissed.
                for _ in 0..600 {
                    if QUIT_REQUESTED.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            });

            if let Some(win) = app.get_webview_window("main") {
                // Window icon for X11/XWayland task bars (Wayland docks use the .desktop entry
                // matched by the GTK app id `io.respeek.Respeek`).
                if let Ok(icon) = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png")) {
                    let _ = win.set_icon(icon);
                }
                let _ = win.set_size(tauri::LogicalSize::new(
                    initial.window_width as f64,
                    initial.window_height as f64,
                ));
                if initial.is_maximized {
                    let _ = win.maximize();
                }
                let _ = win.center();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let state = window.state::<AppState>();
            match event {
                WindowEvent::Resized(size) => {
                    if let Ok(scale) = window.scale_factor() {
                        let logical = size.to_logical::<f64>(scale);
                        let maximized = window.is_maximized().unwrap_or(false);
                        if !maximized {
                            state.engine.save_window(
                                logical.width as i32,
                                logical.height as i32,
                                false,
                            );
                        } else {
                            let s = state.engine.settings();
                            state.engine.save_window(s.window_width, s.window_height, true);
                        }
                    }
                }
                WindowEvent::CloseRequested { .. } => {
                    state.engine.shutdown();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            subscribe,
            set_page,
            select_device,
            set_overview,
            set_paused,
            refresh_now,
            set_search,
            toggle_row,
            select_row,
            sort_by,
            collapse_all,
            device_info_text,
            set_services_user,
            update_settings,
            reset_settings,
            signal_processes,
            service_action,
            process_details,
            service_details,
            smart_data,
            eject_disk,
            about_system,
            setup_script_state,
            setup_script_run,
            setup_script_open,
            finish_first_run,
            quit,
            ui_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Respeek");
}
