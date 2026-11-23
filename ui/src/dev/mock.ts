// DEV-ONLY fixture backend used when the UI runs in a plain browser (no Tauri).
// It exists purely to preview layouts and motion; the real app gets all data from respeek-core.
import type { DeviceDetail, DeviceSummary, Graph, Row, Settings, Snapshot, TableView, Tint } from "../api";

const N = 60;
let seq = 0;
let t = 0;
const hist: Record<string, number[]> = {};
const push = (k: string, v: number) => {
  const a = (hist[k] ??= Array(N).fill(0));
  a.push(v);
  if (a.length > N) a.shift();
  return a;
};
const wave = (base: number, amp: number, f: number, k = 0) =>
  Math.max(0, Math.min(1, base + amp * Math.sin(t * f + k) + (Math.random() - 0.5) * amp * 0.6));

let page: "performance" | "apps" | "services" = "performance";
let device = "cpu";
let overview = false;
let search = "";
let selected: string | null = null;
const expanded = new Set<string>();
let settings: Settings = {
  window_width: 1180, window_height: 760, is_maximized: false, selected_page: "performance",
  appearance: "system", accent: "blue", reduce_transparency: false, update_interval: 20, data_points: 60,
  memory_use_bytes: true, memory_use_base2: true, drive_use_bytes: true, drive_use_base2: true,
  network_use_bytes: false, network_use_base2: false, network_dynamic_scaling: true,
  merged_process_stats: false, remember_sorting: false, remember_column_order: false,
  core_count_affects_percentages: false, show_column_separators: false, hide_is_zero: false, tree_view: true,
  apps_sort_column: "cpu", apps_sort_direction: "descending", apps_column_order: [],
  apps_hidden_columns: ["swap", "gpu_memory"], services_sort_column: "name", services_sort_direction: "ascending",
  services_column_order: [], services_hidden_columns: [], cpu_graph: 1, kernel_times: false,
  selected_device: "cpu", smooth_graphs: true, sliding_graphs: true, show_cpu: true, show_memory: true,
  show_disks: true, show_network: true, show_network_wired: true, show_network_wireless: true,
  show_network_vpn: true, show_network_virtual: true, show_network_other: true, show_gpus: true,
  show_fans: true, show_batteries: true, memory_composition_visible: true, memory_swap_visible: true,
  gpu_graph_middle: 1, gpu_graph_bottom: 2, sidebar_order: [], sidebar_hidden_graphs: [], first_time_running: false,
};
let cb: ((s: Snapshot) => void) | null = null;

const pct = (v: number) => `${Math.round(v * 100)}%`;
const series = (label: string, tint: Tint, values: number[], dashed = false) => ({
  label, tint, values: [...values], dashed, current: pct(values[values.length - 1]),
});
const graph = (id: string, title: string, s: ReturnType<typeof series>[], size: Graph["size"] = "hero"): Graph => ({
  id, title, max_label: "100%", series: s, size,
});

function tick() {
  t += 0.35;
  push("cpu", wave(0.32, 0.18, 0.6));
  for (let i = 0; i < 8; i++) push(`core${i}`, wave(0.3, 0.25, 0.5 + i * 0.13, i));
  push("mem", wave(0.58, 0.02, 0.2));
  push("disk", wave(0.08, 0.08, 1.1));
  push("rx", wave(0.25, 0.22, 0.9));
  push("tx", wave(0.1, 0.08, 1.3, 2));
  push("gpu", wave(0.18, 0.15, 0.7, 1));
  push("gpumem", wave(0.4, 0.02, 0.3));
  push("fan", wave(0.45, 0.06, 0.4));
  push("bat", 0.82);
}

function sidebar(): DeviceSummary[] {
  const last = (k: string) => hist[k][N - 1];
  return ([
    { id: "cpu", kind: "cpu", title: "CPU", subtitle: `${pct(last("cpu"))} · 3.92 GHz`, level: last("cpu"), tint: "cpu", spark: hist.cpu, spark2: null },
    { id: "memory", kind: "memory", title: "Memory", subtitle: "9.3 GiB / 15.5 GiB", level: last("mem"), tint: "memory", spark: hist.mem, spark2: null },
    { id: "disk:nvme0n1", kind: "disk", title: "Disk 0", subtitle: `NVMe SSD · ${pct(last("disk"))}`, level: last("disk"), tint: "disk", spark: hist.disk, spark2: null },
    { id: "net:wlp2s0", kind: "network", title: "Wi-Fi", subtitle: `↓ ${(last("rx") * 40).toFixed(1)} Mb/s  ↑ ${(last("tx") * 40).toFixed(1)} Mb/s`, level: last("rx"), tint: "network", spark: hist.rx, spark2: hist.tx },
    { id: "gpu:0", kind: "gpu", title: "GPU 0", subtitle: `${pct(last("gpu"))} · 48 °C`, level: last("gpu"), tint: "gpu", spark: hist.gpu, spark2: null },
    { id: "fan:1-1", kind: "fan", title: "CPU Fan", subtitle: `${Math.round(last("fan") * 5000).toLocaleString()} RPM`, level: last("fan"), tint: "fan", spark: hist.fan, spark2: null },
    { id: "battery:BAT0", kind: "battery", title: "Battery", subtitle: "82% · On battery", level: 0.82, tint: "battery", spark: hist.bat, spark2: null },
  ] as Omit<DeviceSummary, "level_label">[]).map((d) => ({ ...d, level_label: pct(d.level) }));
}

const stat = (label: string, value: string, primary = false, tint: Tint | null = null) => ({ label, value, primary, tint });

function detail(): DeviceDetail {
  const id = device;
  if (id === "memory")
    return {
      id, kind: "memory", title: "Memory", model: "15.5 GiB total",
      graphs: [graph("memory", "Memory usage", [series("In use", "memory", hist.mem)]),
        graph("swap", "Swap", [series("Swap", "swap", hist.mem.map((v) => v * 0.1))], "half")],
      grid: [],
      stats: [{ title: null, stats: [stat("In use", "9.3 GiB", true, "memory"), stat("Available", "6.2 GiB", true, "memory"), stat("Committed", "14.1 / 23.2 GiB"), stat("Cached", "4.8 GiB"), stat("Swap used", "312 MiB / 8 GiB")] },
        { title: "Hardware", stats: [stat("Speed", "5600 MT/s"), stat("Type", "DDR5"), stat("Form factor", "SODIMM"), stat("Slots used", "2 of 2")] }],
      extras: [{ type: "composition", title: "Memory composition", segments: [
        { label: "In use", value: "9.3 GiB", fraction: 0.6, tint: "memory", tooltip: "Memory used by the operating system and running applications" },
        { label: "Modified", value: "212 MiB", fraction: 0.013, tint: "swap", tooltip: "Memory that must be written to disk" },
        { label: "Standby", value: "4.8 GiB", fraction: 0.31, tint: "kernel", tooltip: "Cached data" },
        { label: "Free", value: "1.2 GiB", fraction: 0.077, tint: "neutral", tooltip: "Unused" }] }],
    };
  if (id.startsWith("disk"))
    return {
      id, kind: "disk", title: "Disk 0", model: "Samsung SSD 990 PRO 1TB",
      graphs: [graph("active", "Active time", [series("Active time", "disk", hist.disk)]),
        { id: "transfer", title: "Transfer rate", max_label: "50 MB/s", size: "half", series: [
          { ...series("Read", "disk", hist.disk), current: "12.4 MB/s" }, { ...series("Write", "disk_write", hist.tx, true), current: "3.1 MB/s" }] }],
      grid: [],
      stats: [{ title: null, stats: [stat("Active time", pct(hist.disk[N - 1]), true, "disk"), stat("Avg. response", "0.21 ms", true, "disk"), stat("Read speed", "12.4 MB/s", true, "disk"), stat("Write speed", "3.1 MB/s", true, "disk_write")] },
        { title: "Drive", stats: [stat("Capacity", "931.5 GiB"), stat("Type", "NVMe SSD"), stat("System disk", "Yes"), stat("Temperature", "41 °C")] }],
      extras: [{ type: "partitions", items: [
        { title: "nvme0n1p1 · /boot/efi", subtitle: "64 MiB of 1 GiB used · vfat", fraction: 0.06, tint: "disk" },
        { title: "nvme0n1p2 · /", subtitle: "412 GiB of 930 GiB used · ext4", fraction: 0.44, tint: "disk" }] },
        { type: "actions", smart: true, eject: false }],
    };
  if (id.startsWith("net"))
    return {
      id, kind: "network", title: "Wi-Fi", model: "Intel Wi-Fi 6E AX211",
      graphs: [{ id: "throughput", title: "Throughput", max_label: "50 Mb/s", size: "hero", series: [
        { ...series("Receive", "network", hist.rx), current: "12.1 Mb/s" }, { ...series("Send", "network_send", hist.tx, true), current: "3.4 Mb/s" }] }],
      grid: [],
      stats: [{ title: null, stats: [stat("Receive", "12.1 Mb/s", true, "network"), stat("Send", "3.4 Mb/s", true, "network_send"), stat("Total received", "4.2 GB"), stat("Total sent", "612 MB")] },
        { title: "Adapter", stats: [stat("Interface", "wlp2s0"), stat("SSID", "Studio"), stat("Signal strength", "78%"), stat("Frequency", "5.18 GHz"), stat("IPv4 address", "192.168.1.24")] }],
      extras: [],
    };
  if (id.startsWith("gpu"))
    return {
      id, kind: "gpu", title: "GPU 0", model: "AMD Radeon 780M",
      graphs: [graph("usage", "Utilization", [series("Utilization", "gpu", hist.gpu)]),
        graph("memory", "Memory usage", [series("Memory", "gpu_memory", hist.gpumem)], "half"),
        graph("codec", "Video engines", [series("Encode", "encoder", hist.tx), series("Decode", "decoder", hist.disk, true)], "half")],
      grid: [],
      stats: [{ title: null, stats: [stat("Utilization", pct(hist.gpu[N - 1]), true, "gpu"), stat("Memory", "1.6 / 4 GiB", true, "gpu_memory"), stat("Clock speed", "1.20 / 2.70 GHz"), stat("Temperature", "48 °C")] },
        { title: "Graphics card", stats: [stat("Vulkan", "1.3.296"), stat("OpenGL", "4.6"), stat("PCI ID", "1002:15bf")] }],
      extras: [],
    };
  if (id.startsWith("fan"))
    return { id, kind: "fan", title: "CPU Fan", model: "hwmon3 · fan1", graphs: [graph("rpm", "Fan speed", [series("Speed", "fan", hist.fan)])], grid: [],
      stats: [{ title: null, stats: [stat("Speed", "2,310 RPM", true, "fan"), stat("PWM", "46%", true, "fan")] }], extras: [] };
  if (id.startsWith("battery"))
    return { id, kind: "battery", title: "Battery", model: "5B11M90052", graphs: [graph("charge", "Charge", [series("Charge", "battery", hist.bat)])], grid: [],
      stats: [{ title: null, stats: [stat("Charge", "82%", true, "battery"), stat("State", "On battery", true, "battery"), stat("Time remaining", "4 h 12 min")] }], extras: [] };
  return {
    id: "cpu", kind: "cpu", title: "CPU", model: "AMD Ryzen 7 7840U w/ Radeon 780M Graphics",
    graphs: [graph("cpu", "Utilization", [series("Utilization", "cpu", hist.cpu)])],
    grid: settings.cpu_graph === 2 ? Array.from({ length: 8 }, (_, i) => graph(`core${i}`, `CPU ${i}`, [series(`CPU ${i}`, "cpu", hist[`core${i}`])], "mini")) : [],
    stats: [{ title: null, stats: [stat("Utilization", pct(hist.cpu[N - 1]), true, "cpu"), stat("Speed", "3.92 GHz", true, "cpu"), stat("Processes", "412"), stat("Threads", "1,904"), stat("Handles", "12,380"), stat("Up time", "2:14:08")] },
      { title: "Processor", stats: [stat("Base speed", "3.30 GHz"), stat("Sockets", "1"), stat("Logical processors", "16"), stat("Virtualization", "AMD-V"), stat("L1 cache", "512 KiB"), stat("L2 cache", "8 MiB"), stat("L3 cache", "16 MiB")] }],
    extras: [],
  };
}

const APPS = [
  ["firefox", "Firefox", 0.18, "1.4 GiB"], ["code", "Visual Studio Code", 0.09, "980 MiB"],
  ["org.gnome.Nautilus", "Files", 0.01, "120 MiB"], ["spotify", "Spotify", 0.03, "410 MiB"],
  ["slack", "Slack", 0.02, "620 MiB"], ["terminal", "Terminal", 0.004, "64 MiB"],
] as const;

function appsTable(): TableView {
  const rows: Row[] = [];
  const cells = (cpu: number, mem: string) => [
    { text: "", heat: null }, { text: pct(cpu), heat: cpu }, { text: mem, heat: 0.2 },
    { text: cpu > 0.05 ? "1.2 MB/s" : "", heat: 0.05 }, { text: cpu > 0.1 ? "320 kb/s" : "", heat: 0.03 }, { text: cpu > 0.08 ? "4%" : "", heat: 0.04 },
  ];
  rows.push({ key: "apps", kind: "section", depth: 0, name: "Apps", subtitle: null, icon: null, glyph: "section", pid: null, expandable: true, expanded: true, status: null, cells: cells(0, "").map(() => ({ text: "", heat: null })) });
  for (const [id, name, cpu, mem] of APPS) {
    if (search && !name.toLowerCase().includes(search.toLowerCase())) continue;
    const key = `app:${id}`;
    const c = Math.max(0, cpu + (Math.random() - 0.5) * cpu);
    rows.push({ key, kind: "app", depth: 1, name, subtitle: null, icon: null, glyph: "app", pid: null, expandable: true, expanded: expanded.has(key), status: null, cells: cells(c, mem) });
    if (expanded.has(key))
      for (let i = 0; i < 3; i++) {
        const pid = 2000 + i + name.length * 10;
        rows.push({ key: `${key}/${pid}`, kind: "process", depth: 2, name: `${id}${i ? "-helper" : ""}`, subtitle: `/usr/bin/${id} --type=${i}`, icon: null, glyph: "process", pid, expandable: false, expanded: false, status: null, cells: [{ text: String(pid), heat: null }, ...cells(c / 3, "120 MiB").slice(1)] });
      }
  }
  rows.push({ key: "procs", kind: "section", depth: 0, name: "Processes", subtitle: null, icon: null, glyph: "section", pid: null, expandable: true, expanded: true, status: null, cells: cells(0, "").map(() => ({ text: "", heat: null })) });
  for (let i = 0; i < 120; i++) {
    const name = ["systemd", "pipewire", "gnome-shell", "Xwayland", "dbus-daemon", "NetworkManager", "kworker", "bash"][i % 8];
    if (search && !name.includes(search)) continue;
    const c = Math.random() * 0.03;
    rows.push({ key: `procs/${100 + i}`, kind: "process", depth: 1, name, subtitle: `/usr/bin/${name}`, icon: null, glyph: "process", pid: 100 + i, expandable: i % 5 === 0, expanded: false, status: i === 7 ? "suspended" : null, cells: [{ text: String(100 + i), heat: null }, ...cells(c, `${(Math.random() * 90 + 4).toFixed(1)} MiB`).slice(1)] });
  }
  const sel = rows.find((r) => r.key === selected);
  return {
    headline: `${APPS.length} Running Apps`, subheadline: "412 Processes",
    columns: [
      { id: "name", title: "Name", total: null, numeric: false, visible: true },
      { id: "pid", title: "PID", total: null, numeric: true, visible: true },
      { id: "cpu", title: "CPU", total: pct(hist.cpu[N - 1]), numeric: true, visible: true },
      { id: "memory", title: "Memory", total: "60%", numeric: true, visible: true },
      { id: "swap", title: "Swap", total: null, numeric: true, visible: false },
      { id: "drive", title: "Drive", total: "4%", numeric: true, visible: true },
      { id: "network", title: "Network", total: null, numeric: true, visible: true },
      { id: "gpu", title: "GPU", total: "18%", numeric: true, visible: true },
      { id: "gpu_memory", title: "GPU Mem", total: null, numeric: true, visible: false },
    ],
    rows, sort_column: settings.apps_sort_column, sort_ascending: settings.apps_sort_direction === "ascending",
    selection: sel ? { key: sel.key, title: sel.name, pids: sel.pid ? [sel.pid] : [1234], service_id: null, is_app: sel.kind === "app", service_running: false, service_enabled: false, suspended: false } : null,
    notice: null,
  };
}

function servicesTable(): TableView {
  const names = ["bluetooth", "cron", "cups", "dbus", "gdm", "NetworkManager", "power-profiles-daemon", "snapd", "ssh", "systemd-journald", "systemd-resolved", "udisks2", "upower", "wpa_supplicant"];
  const rows: Row[] = names
    .filter((n) => !search || n.includes(search))
    .map((n, i) => {
      const status = i === 2 ? "failed" : i % 4 === 3 ? "stopped" : "running";
      return {
        key: `svc:${i}`, kind: "service", depth: 0, name: n, subtitle: `${n} daemon`, icon: null, glyph: "service",
        pid: status === "running" ? 400 + i : null, expandable: false, expanded: false, status,
        cells: [{ text: status[0].toUpperCase() + status.slice(1), heat: null }, { text: i % 5 ? "Enabled" : "Disabled", heat: null },
          { text: status === "running" ? String(400 + i) : "", heat: null }, { text: "root", heat: null }, { text: "root", heat: null }, { text: `${n} daemon`, heat: null }],
      } satisfies Row;
    });
  const sel = rows.find((r) => r.key === selected);
  return {
    headline: `${names.length} System Services`, subheadline: "11 running · 1 failed",
    columns: ["Name", "Status", "Startup", "PID", "User", "Group", "Description"].map((title) => ({
      id: title.toLowerCase(), title, total: null, numeric: title === "PID", visible: true })),
    rows, sort_column: "name", sort_ascending: true,
    selection: sel ? { key: sel.key, title: sel.name, pids: sel.pid ? [sel.pid] : [], service_id: 1, is_app: false, service_running: sel.status === "running", service_enabled: true, suspended: false } : null,
    notice: null,
  };
}

function snapshot(): Snapshot {
  seq++;
  return {
    seq, interval_ms: Math.round(settings.update_interval * 50), sidebar: sidebar(), settings, paused: false,
    page: page === "apps" ? { page: "apps", table: appsTable() }
      : page === "services" ? { page: "services", table: servicesTable(), user: false }
      : { page: "performance", detail: overview ? null : detail(), overview },
  };
}

const emit = () => cb?.(snapshot());

export async function subscribe(fn: (s: Snapshot) => void) {
  cb = fn;
  for (let i = 0; i < N; i++) tick();
  const loop = () => {
    tick();
    emit();
    setTimeout(loop, settings.update_interval * 50);
  };
  setTimeout(loop, settings.update_interval * 50);
  return snapshot();
}

export async function invoke(cmd: string, args: Record<string, any> = {}) {
  switch (cmd) {
    case "set_page": page = args.page; selected = null; search = ""; break;
    case "select_device": page = "performance"; overview = false; device = args.id; break;
    case "set_overview": page = "performance"; overview = args.on; break;
    case "set_search": search = args.query; break;
    case "toggle_row": expanded.has(args.key) ? expanded.delete(args.key) : expanded.add(args.key); break;
    case "select_row": selected = args.key; break;
    case "collapse_all": expanded.clear(); break;
    case "sort_by": settings = { ...settings, apps_sort_column: args.column }; break;
    case "update_settings": settings = { ...settings, ...args.patch }; emit(); return settings;
    case "process_details":
      return { pid: args.pid, name: "firefox", icon: null, state: "Sleeping", command_line: "/usr/lib/firefox/firefox -contentproc", exe: "/usr/lib/firefox/firefox",
        groups: [{ title: null, stats: [stat("CPU", "4.2%", true, "cpu"), stat("Memory", "412 MiB", true, "memory"), stat("Drive", "0 B/s", true, "disk"), stat("Network", "12 kb/s", true, "network")] },
          { title: "Process", stats: [stat("PID", String(args.pid)), stat("Parent", "systemd (1)"), stat("Threads", "84")] }] };
    case "service_details":
      return { id: args.id, name: "bluetooth.service", description: "Bluetooth service", groups: [{ title: null, stats: [stat("Status", "Running"), stat("PID", "812")] }],
        logs: "Nov 23 10:02:11 host bluetoothd[812]: Bluetooth daemon 5.72\nNov 23 10:02:11 host bluetoothd[812]: Starting SDP server\n" };
    case "smart_data":
      return { title: "Samsung SSD 990 PRO", available: true, groups: [{ title: null, stats: [stat("Last self-test", "Success", true, "disk"), stat("Powered on", "1 year, 2 months")] },
        { title: "NVMe health", stats: [stat("Available spare", "100%"), stat("Percentage used", "2%"), stat("Data read", "21.4 TB"), stat("Data written", "18.9 TB")] }], attributes_header: [], attributes: [] };
    case "about_system":
      return { logo: null, title: "Pop!_OS 24.04 LTS", groups: [{ title: "Device", stats: [stat("Hostname", "studio"), stat("Model", "ThinkPad T14s")] }, { title: "System", stats: [stat("Kernel", "6.18")] }] };
    case "device_info_text": return "CPU\nAMD Ryzen 7\n";
    case "setup_script_state": return { state: "idle", file: "/usr/share/respeek/setup.sh", command: "pkexec setup.sh", revert_command: "pkexec setup.sh --revert" };
  }
  setTimeout(emit, 0);
  return undefined;
}
