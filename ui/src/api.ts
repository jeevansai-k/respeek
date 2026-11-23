// Typed bridge to the Rust core. Types mirror crates/respeek-core/src/model.rs.
// The UI never computes data: it renders these view-models and sends intents.

export type Tint =
  | "cpu" | "memory" | "swap" | "disk" | "disk_write" | "network" | "network_send" | "gpu"
  | "gpu_memory" | "encoder" | "decoder" | "fan" | "battery" | "kernel" | "neutral";
export type DeviceKind = "cpu" | "memory" | "disk" | "network" | "gpu" | "fan" | "battery";

export interface Series { label: string; tint: Tint; values: number[]; dashed: boolean; current: string }
export interface Graph { id: string; title: string; max_label: string; series: Series[]; size: "hero" | "half" | "mini" }
export interface Stat { label: string; value: string; primary: boolean; tint: Tint | null }
export interface StatGroup { title: string | null; stats: Stat[] }
export interface Segment { label: string; value: string; fraction: number; tint: Tint; tooltip: string }
export interface UsageBar { title: string; subtitle: string; fraction: number; tint: Tint }
export type Extra =
  | { type: "composition"; title: string; segments: Segment[] }
  | { type: "partitions"; items: UsageBar[] }
  | { type: "details"; title: string; groups: StatGroup[] }
  | { type: "actions"; smart: boolean; eject: boolean };

export interface DeviceSummary {
  id: string; kind: DeviceKind; title: string; subtitle: string; level: number; level_label: string; tint: Tint;
  spark: number[]; spark2: number[] | null;
}
export interface DeviceDetail {
  id: string; kind: DeviceKind; title: string; model: string; graphs: Graph[]; grid: Graph[];
  stats: StatGroup[]; extras: Extra[];
}

export type RowKind = "section" | "app" | "process" | "service";
export interface Cell { text: string; heat: number | null }
export interface Row {
  key: string; kind: RowKind; depth: number; name: string; subtitle: string | null; icon: string | null;
  glyph: string; pid: number | null; expandable: boolean; expanded: boolean; status: string | null; cells: Cell[];
}
export interface Column { id: string; title: string; total: string | null; numeric: boolean; visible: boolean }
export interface Selection {
  key: string; title: string; pids: number[]; service_id: number | null; is_app: boolean;
  service_running: boolean; service_enabled: boolean; suspended: boolean;
}
export interface TableView {
  headline: string; subheadline: string; columns: Column[]; rows: Row[]; sort_column: string;
  sort_ascending: boolean; selection: Selection | null; notice: string | null;
}

export type PageData =
  | { page: "performance"; detail: DeviceDetail | null; overview: boolean }
  | { page: "apps"; table: TableView }
  | { page: "services"; table: TableView; user: boolean };

export interface Settings {
  window_width: number; window_height: number; is_maximized: boolean; selected_page: string;
  appearance: "system" | "light" | "dark"; accent: string; reduce_transparency: boolean;
  update_interval: number; data_points: number;
  memory_use_bytes: boolean; memory_use_base2: boolean; drive_use_bytes: boolean; drive_use_base2: boolean;
  network_use_bytes: boolean; network_use_base2: boolean; network_dynamic_scaling: boolean;
  merged_process_stats: boolean; remember_sorting: boolean; remember_column_order: boolean;
  core_count_affects_percentages: boolean; show_column_separators: boolean; hide_is_zero: boolean;
  tree_view: boolean; apps_sort_column: string; apps_sort_direction: "ascending" | "descending";
  apps_column_order: string[]; apps_hidden_columns: string[]; services_sort_column: string;
  services_sort_direction: "ascending" | "descending"; services_column_order: string[];
  services_hidden_columns: string[];
  cpu_graph: number; kernel_times: boolean; selected_device: string; smooth_graphs: boolean;
  sliding_graphs: boolean; show_cpu: boolean; show_memory: boolean; show_disks: boolean;
  show_network: boolean; show_network_wired: boolean; show_network_wireless: boolean;
  show_network_vpn: boolean; show_network_virtual: boolean; show_network_other: boolean;
  show_gpus: boolean; show_fans: boolean; show_batteries: boolean;
  memory_composition_visible: boolean; memory_swap_visible: boolean; gpu_graph_middle: number;
  gpu_graph_bottom: number; sidebar_order: string[]; sidebar_hidden_graphs: string[];
  first_time_running: boolean;
}

export interface Snapshot {
  seq: number; interval_ms: number; sidebar: DeviceSummary[]; page: PageData; settings: Settings; paused: boolean;
}

export interface ProcessDetails {
  pid: number; name: string; icon: string | null; state: string; groups: StatGroup[]; command_line: string; exe: string;
}
export interface ServiceDetails { id: number; name: string; description: string; groups: StatGroup[]; logs: string }
export interface SmartView {
  title: string; available: boolean; groups: StatGroup[]; attributes_header: string[]; attributes: string[][];
}
export interface EjectFailure { blockers: { pid: number; name: string; icon: string | null; files: string[] }[] }
export interface AboutSystem { logo: string | null; title: string; groups: StatGroup[] }
export type SetupScriptState =
  | { state: "unavailable"; reason: string }
  | { state: "idle"; file: string; command: string; revert_command: string }
  | { state: "running" }
  | { state: "finished"; ok: boolean; message: string | null };

export type Page = "performance" | "apps" | "services";
export type TableId = "apps" | "services";
export type Signal = "terminate" | "kill" | "interrupt" | "hangup" | "suspend" | "continue" | "user1" | "user2";
export type ServiceAction = "start" | "stop" | "restart" | "enable" | "disable";

// ---------------------------------------------------------------------------------------------

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
let invokeImpl: Invoke;
let subscribeImpl: (cb: (s: Snapshot) => void) => Promise<Snapshot>;
let assetUrlImpl: (path: string) => string = (p) => p;

async function init() {
  if (isTauri) {
    const core = await import("@tauri-apps/api/core");
    invokeImpl = core.invoke as Invoke;
    assetUrlImpl = (p) => core.convertFileSrc(p);
    subscribeImpl = async (cb) => {
      const ch = new core.Channel<Snapshot>();
      ch.onmessage = cb;
      return core.invoke<Snapshot>("subscribe", { onSnapshot: ch });
    };
  } else {
    // Browser preview only (vite dev without Tauri): a fixture backend for visual work.
    const mock = await import("./dev/mock");
    invokeImpl = mock.invoke as Invoke;
    subscribeImpl = mock.subscribe;
  }
}
const ready = init();

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await ready;
  return invokeImpl<T>(cmd, args);
}

export const assetUrl = (path: string | null) => (path ? assetUrlImpl(path) : null);

export const api = {
  subscribe: async (cb: (s: Snapshot) => void) => {
    await ready;
    return subscribeImpl(cb);
  },
  setPage: (page: Page) => call<void>("set_page", { page }),
  selectDevice: (id: string) => call<void>("select_device", { id }),
  setOverview: (on: boolean) => call<void>("set_overview", { on }),
  setPaused: (paused: boolean) => call<void>("set_paused", { paused }),
  refreshNow: () => call<void>("refresh_now"),
  setSearch: (table: TableId, query: string) => call<void>("set_search", { table, query }),
  toggleRow: (table: TableId, key: string) => call<void>("toggle_row", { table, key }),
  selectRow: (table: TableId, key: string | null) => call<void>("select_row", { table, key }),
  sortBy: (table: TableId, column: string) => call<void>("sort_by", { table, column }),
  collapseAll: (table: TableId) => call<void>("collapse_all", { table }),
  setServicesUser: (user: boolean) => call<void>("set_services_user", { user }),
  updateSettings: (patch: Partial<Settings>) => call<Settings>("update_settings", { patch }),
  resetSettings: () => call<Settings>("reset_settings"),
  signal: (pids: number[], signal: Signal) => call<void>("signal_processes", { pids, signal }),
  serviceAction: (id: number, action: ServiceAction) => call<void>("service_action", { id, action }),
  processDetails: (pid: number) => call<ProcessDetails | null>("process_details", { pid }),
  serviceDetails: (id: number) => call<ServiceDetails | null>("service_details", { id }),
  smartData: (device: string) => call<SmartView>("smart_data", { device }),
  ejectDisk: (device: string) => call<void>("eject_disk", { device }),
  aboutSystem: () => call<AboutSystem | null>("about_system"),
  deviceInfoText: (id: string) => call<string | null>("device_info_text", { id }),
  setupScriptState: () => call<SetupScriptState>("setup_script_state"),
  setupScriptRun: (revert: boolean) => call<void>("setup_script_run", { revert }),
  setupScriptOpen: () => call<void>("setup_script_open"),
  finishFirstRun: () => call<void>("finish_first_run"),
  quit: () => call<void>("quit"),
};

export async function windowControl(action: "close" | "minimize" | "maximize" | "drag") {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (action === "close") return api.quit();
  if (action === "minimize") return w.minimize();
  if (action === "maximize") return w.toggleMaximize();
  if (action === "drag") return w.startDragging();
}
