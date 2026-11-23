import { AnimatePresence, motion } from "motion/react";
import { Activity, AppWindow, Ruler, SlidersHorizontal } from "lucide-react";
import { useState, type ReactNode } from "react";
import { api, type Settings } from "../api";
import { Button, Segmented, Switch } from "../components/primitives";
import { spring } from "../motion";
import { useUi } from "../store";
import { Sheet } from "./Sheet";
import { SetupScriptPanel } from "./Sheets";

type Patch = Partial<Settings>;
const set = (p: Patch) => api.updateSettings(p);

function Row({ title, subtitle, children }: { title: string; subtitle?: string; children: ReactNode }) {
  return (
    <div className="pref-row">
      <div className="pref-text">
        <span className="pref-title">{title}</span>
        {subtitle && <span className="pref-sub">{subtitle}</span>}
      </div>
      <div className="pref-control">{children}</div>
    </div>
  );
}

function Toggle({ s, k, title, subtitle }: { s: Settings; k: keyof Settings; title: string; subtitle?: string }) {
  return (
    <Row title={title} subtitle={subtitle}>
      <Switch checked={Boolean(s[k])} onChange={(v) => set({ [k]: v } as Patch)} label={title} />
    </Row>
  );
}

function Group({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className="pref-group">
      {title && <h3 className="list-title">{title}</h3>}
      <div className="pref-card">{children}</div>
    </section>
  );
}

function Slider({ value, min, max, step, onCommit, label }: {
  value: number; min: number; max: number; step: number; onCommit: (v: number) => void; label: (v: number) => string;
}) {
  const [local, setLocal] = useState<number | null>(null);
  const v = local ?? value;
  return (
    <div className="slider">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={v}
        style={{ "--fill": `${((v - min) / (max - min)) * 100}%` } as React.CSSProperties}
        onChange={(e) => setLocal(Number(e.target.value))}
        onPointerUp={() => {
          if (local !== null) onCommit(local);
          setLocal(null);
        }}
        onKeyUp={() => {
          if (local !== null) onCommit(local);
          setLocal(null);
        }}
      />
      <span className="slider-value">{label(v)}</span>
    </div>
  );
}

const ACCENTS = ["blue", "purple", "pink", "orange", "green", "graphite"];

function General({ s }: { s: Settings }) {
  return (
    <>
      <Group title="Appearance">
        <Row title="Theme">
          <Segmented size="sm" value={s.appearance} onChange={(v) => set({ appearance: v })}
            options={[{ value: "system", label: "Auto" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]} />
        </Row>
        <Row title="Accent color">
          <div className="accents">
            {ACCENTS.map((a) => (
              <button key={a} className={`accent-dot acc-${a} ${s.accent === a ? "on" : ""}`} aria-label={a} onClick={() => set({ accent: a })}>
                {s.accent === a && <motion.span className="accent-ring" layoutId="accent-ring" transition={spring} />}
              </button>
            ))}
          </div>
        </Row>
        <Toggle s={s} k="reduce_transparency" title="Reduce transparency" subtitle="Use solid surfaces instead of glass." />
      </Group>
      <Group title="Updates">
        <Row title="Update interval" subtitle="How often readings refresh.">
          <Slider value={s.update_interval} min={10} max={200} step={2} onCommit={(v) => set({ update_interval: v })}
            label={(v) => `${(v * 0.05).toFixed(v < 40 ? 1 : 0)} s`} />
        </Row>
        <Row title="Graph history" subtitle="Number of samples shown in graphs.">
          <Slider value={s.data_points} min={10} max={600} step={10} onCommit={(v) => set({ data_points: v })} label={(v) => `${v}`} />
        </Row>
      </Group>
    </>
  );
}

function Performance({ s }: { s: Settings }) {
  return (
    <>
      <Group title="Graphs">
        <Toggle s={s} k="smooth_graphs" title="Smooth graphs" subtitle="Draw curves instead of straight segments." />
        <Toggle s={s} k="sliding_graphs" title="Sliding graphs" subtitle="Scroll continuously between samples." />
        <Toggle s={s} k="kernel_times" title="Show kernel times" />
        <Toggle s={s} k="memory_composition_visible" title="Memory composition" />
        <Toggle s={s} k="memory_swap_visible" title="Swap graph" />
        <Row title="GPU graphs">
          <div className="row-gap">
            {(["gpu_graph_middle", "gpu_graph_bottom"] as const).map((k) => (
              <select key={k} className="select" value={s[k]} onChange={(e) => set({ [k]: Number(e.target.value) } as Patch)}>
                <option value={0}>None</option><option value={1}>Memory</option><option value={2}>Video engines</option>
                <option value={3}>Power draw</option><option value={4}>Temperature</option>
              </select>
            ))}
          </div>
        </Row>
      </Group>
      <Group title="Sidebar devices">
        <Toggle s={s} k="show_cpu" title="CPU" />
        <Toggle s={s} k="show_memory" title="Memory" />
        <Toggle s={s} k="show_disks" title="Disks" />
        <Toggle s={s} k="show_network" title="Network" />
        <Toggle s={s} k="show_gpus" title="GPUs" />
        <Toggle s={s} k="show_fans" title="Fans" />
        <Toggle s={s} k="show_batteries" title="Batteries" />
      </Group>
      <AnimatePresence initial={false}>
        {s.show_network && (
          <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }} exit={{ opacity: 0, height: 0 }} transition={spring}>
            <Group title="Network connections">
              <Toggle s={s} k="show_network_wired" title="Wired" />
              <Toggle s={s} k="show_network_wireless" title="Wireless" />
              <Toggle s={s} k="show_network_vpn" title="VPN" />
              <Toggle s={s} k="show_network_virtual" title="Virtual & containers" />
              <Toggle s={s} k="show_network_other" title="Other" />
            </Group>
          </motion.div>
        )}
      </AnimatePresence>
      {s.sidebar_order.length > 0 && (
        <Group>
          <Row title="Sidebar order" subtitle="Drag devices in the sidebar to reorder them.">
            <Button size="sm" variant="plain" onClick={() => set({ sidebar_order: [] })}>Reset order</Button>
          </Row>
        </Group>
      )}
    </>
  );
}

function Units({ s }: { s: Settings }) {
  const pair = (bytesKey: keyof Settings, base2Key: keyof Settings, title: string) => (
    <>
      <Row title={title}>
        <Segmented size="sm" value={s[bytesKey] ? "bytes" : "bits"} onChange={(v) => set({ [bytesKey]: v === "bytes" } as Patch)}
          options={[{ value: "bytes", label: "Bytes" }, { value: "bits", label: "Bits" }]} />
      </Row>
      <Row title="" subtitle={s[base2Key] ? "Powers of 1024 (KiB, MiB…)" : "Powers of 1000 (kB, MB…)"}>
        <Segmented size="sm" value={s[base2Key] ? "2" : "10"} onChange={(v) => set({ [base2Key]: v === "2" } as Patch)}
          options={[{ value: "10", label: "Base 10" }, { value: "2", label: "Base 2" }]} />
      </Row>
    </>
  );
  return (
    <>
      <Group title="Memory">{pair("memory_use_bytes", "memory_use_base2", "Memory units")}</Group>
      <Group title="Drives">{pair("drive_use_bytes", "drive_use_base2", "Drive units")}</Group>
      <Group title="Network">
        {pair("network_use_bytes", "network_use_base2", "Network units")}
        <Toggle s={s} k="network_dynamic_scaling" title="Dynamic scaling" subtitle="Fit graphs to recent traffic instead of link speed." />
      </Group>
    </>
  );
}

function AppsServices({ s }: { s: Settings }) {
  return (
    <>
      <Group title="Tables">
        <Toggle s={s} k="tree_view" title="Show as tree" subtitle="Nest child processes under their parents." />
        <Toggle s={s} k="merged_process_stats" title="Include children in totals" subtitle="Parent rows show the sum of their subtree (tree view only)." />
        <Toggle s={s} k="core_count_affects_percentages" title="Per-core CPU percentages" subtitle="100% means one full core instead of the whole CPU." />
        <Toggle s={s} k="hide_is_zero" title="Hide zero values" />
      </Group>
      <Group title="Per-app network usage">
        <div className="pref-pad"><SetupScriptPanel /></div>
      </Group>
    </>
  );
}

const TABS = [
  { id: "general", label: "General", icon: SlidersHorizontal, view: General },
  { id: "performance", label: "Performance", icon: Activity, view: Performance },
  { id: "units", label: "Units", icon: Ruler, view: Units },
  { id: "apps", label: "Apps & Services", icon: AppWindow, view: AppsServices },
];

export function Preferences({ tab }: { tab?: string }) {
  const s = useUi((st) => st.snap?.settings);
  const [active, setActive] = useState(tab ?? "general");
  const [dir, setDir] = useState(1);
  if (!s) return null;
  const T = TABS.find((t) => t.id === active) ?? TABS[0];
  const View = T.view;
  return (
    <Sheet width={720} title="Preferences"
      footer={<><Button variant="plain" onClick={() => api.resetSettings()}>Restore defaults</Button><span className="grow" /></>}>
      <div className="prefs">
        <nav className="prefs-nav">
          {TABS.map((t, i) => (
            <button key={t.id} className={`prefs-tab ${active === t.id ? "active" : ""}`}
              onClick={() => {
                setDir(i > TABS.findIndex((x) => x.id === active) ? 1 : -1);
                setActive(t.id);
              }}>
              {active === t.id && <motion.span className="sb-pill" layoutId="prefs-pill" transition={spring} />}
              <t.icon size={15} />
              <span>{t.label}</span>
            </button>
          ))}
        </nav>
        <div className="prefs-content">
          <AnimatePresence mode="popLayout" initial={false} custom={dir}>
            <motion.div key={active} custom={dir}
              initial={{ opacity: 0, y: 14 * dir }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 * dir }}
              transition={spring}>
              <View s={s} />
            </motion.div>
          </AnimatePresence>
        </div>
      </div>
    </Sheet>
  );
}
