// DEV-ONLY: drives page switches and reports frame pacing (enabled with VITE_PERF_TOUR=1).
import { api, isTauri } from "../api";

const report = async (line: string) => {
  console.warn("[perf]", line);
  if (isTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("ui_log", { line: `[perf] ${line}` }).catch(() => undefined);
  }
};

export const commits: { at: number; ms: number; phase: string }[] = [];
export const onRender = (_id: string, phase: string, actual: number) => {
  if (actual > 4) commits.push({ at: performance.now(), ms: actual, phase });
};

let sampleStart = 0;
const slowFrames: number[] = [];
function sample(ms: number): Promise<number[]> {
  return new Promise((resolve) => {
    const gaps: number[] = [];
    let last = performance.now();
    sampleStart = last;
    slowFrames.length = 0;
    commits.length = 0;
    const end = last + ms;
    const f = (t: number) => {
      const g = t - last;
      gaps.push(g);
      if (g > 20) slowFrames.push(Math.round(t - sampleStart));
      last = t;
      if (t < end) requestAnimationFrame(f);
      else resolve(gaps.slice(1));
    };
    requestAnimationFrame(f);
  });
}

const detail = () =>
  ` slowAt=[${slowFrames.join(",")}] commits=[${commits
    .map((c) => `${Math.round(c.at - sampleStart)}:${c.phase[0]}${c.ms.toFixed(0)}`)
    .join(" ")}]`;

const stats = (label: string, g: number[]) => {
  const sorted = [...g].sort((a, b) => a - b);
  const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? 0;
  return `${label}: frames=${g.length} avg=${(g.reduce((a, b) => a + b, 0) / g.length).toFixed(1)}ms p95=${p95.toFixed(1)}ms max=${Math.max(...g).toFixed(1)}ms >20ms=${g.filter((x) => x > 20).length}`;
};

export async function runPerfTour() {
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  await wait(4000);
  const steps: [string, () => Promise<unknown>][] = [
    ["idle-cpu", async () => api.selectDevice("cpu")],
    ["->apps", async () => api.setPage("apps")],
    ["->services", async () => api.setPage("services")],
    ["->overview", async () => api.setOverview(true)],
    ["->cpu", async () => api.selectDevice("cpu")],
    ["->apps(2)", async () => api.setPage("apps")],
    ["->memory", async () => api.selectDevice("memory")],
    ["->cpu(b)", async () => api.selectDevice("cpu")],
    ["->memory(2)", async () => api.selectDevice("memory")],
    ["->cpu(c)", async () => api.updateSettings({ memory_composition_visible: false }).then(() => api.selectDevice("cpu"))],
    ["->memory(noComp)", async () => api.selectDevice("memory")],
    ["->cpu(d)", async () => api.updateSettings({ memory_composition_visible: true, memory_swap_visible: false }).then(() => api.selectDevice("cpu"))],
    ["->memory(noSwap)", async () => api.selectDevice("memory")],
    ["->cpu(e)", async () => api.updateSettings({ memory_swap_visible: true }).then(() => api.selectDevice("cpu"))],
  ];
  for (const [label, go] of steps) {
    const p = sample(700);
    await go();
    await report(stats(label, await p) + detail());
    await wait(1500);
  }
  await report(stats("steady-memory-3s", await sample(3000)));
}
