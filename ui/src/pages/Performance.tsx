import { AnimatePresence, motion } from "motion/react";
import { Copy, Disc, HeartPulse, Info, Layers2, Sigma } from "lucide-react";
import { useState } from "react";
import { api, type DeviceDetail, type Extra, type Graph as GraphT, type Settings } from "../api";
import { Graph } from "../components/Graph";
import { Button, DeviceGlyph, IconButton, Segmented, StatView, Ticker } from "../components/primitives";
import { snappy, spring } from "../motion";
import { useUi } from "../store";
import logo from "../assets/respeek.svg";

function GraphCard({ g, settings, className }: { g: GraphT; settings: Settings; className?: string }) {
  return (
    <section className={`card graph-card size-${g.size} ${className ?? ""}`}>
      <header className="graph-head">
        <h3>{g.title}</h3>
        <span className="graph-max">{g.max_label}</span>
      </header>
      <div className="graph-body">
        <Graph series={g.series} sliding={settings.sliding_graphs} smooth={settings.smooth_graphs} grid={g.size !== "mini"} />
      </div>
      {g.size !== "mini" && (
        <footer className="graph-legend">
          {g.series.map((s) => (
            <span key={s.label} className="legend-item">
              <i className={`legend-swatch ${s.dashed ? "dashed" : ""}`} style={{ "--sw": `var(--tint-${s.tint})` } as React.CSSProperties} />
              <span className="legend-label">{s.label}</span>
              <Ticker className="legend-value" value={s.current} />
            </span>
          ))}
        </footer>
      )}
      {g.size === "mini" && <span className="mini-value">{g.series[0]?.current}</span>}
    </section>
  );
}

function Composition({ extra }: { extra: Extract<Extra, { type: "composition" }> }) {
  const [hover, setHover] = useState<number | null>(null);
  return (
    <section className="card">
      <header className="graph-head"><h3>{extra.title}</h3></header>
      <div className="composition" onPointerLeave={() => setHover(null)}>
        {extra.segments.map((s, i) => (
          <div
            key={s.label}
            className={`comp-seg ${hover === i ? "hover" : ""}`}
            style={{ "--sw": `var(--tint-${s.tint})`, flexGrow: Math.max(s.fraction, 0.004) } as React.CSSProperties}
            onPointerEnter={() => setHover(i)}
          />
        ))}
      </div>
      <div className="comp-legend">
        {extra.segments.map((s, i) => (
          <div key={s.label} className={`comp-item ${hover === i ? "hover" : ""}`} onPointerEnter={() => setHover(i)} onPointerLeave={() => setHover(null)}>
            <i className="legend-swatch" style={{ "--sw": `var(--tint-${s.tint})` } as React.CSSProperties} />
            <span className="comp-label">{s.label}</span>
            <span className="comp-value">{s.value}</span>
          </div>
        ))}
      </div>
      <AnimatePresence>
        {hover !== null && (
          <motion.p className="comp-tip" initial={{ opacity: 0, y: -4 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={snappy}>
            {extra.segments[hover].tooltip}
          </motion.p>
        )}
      </AnimatePresence>
    </section>
  );
}

function Partitions({ extra }: { extra: Extract<Extra, { type: "partitions" }> }) {
  return (
    <section className="card">
      <header className="graph-head"><h3>Volumes</h3></header>
      <ul className="usage-list">
        {extra.items.map((it) => (
          <li key={it.title}>
            <div className="usage-text">
              <span className="usage-title">{it.title}</span>
              <span className="usage-sub">{it.subtitle}</span>
            </div>
            <div className="usage-track">
              <div className="usage-fill" style={{ "--sw": `var(--tint-${it.tint})`, width: `${Math.max(it.fraction * 100, 0.5)}%` } as React.CSSProperties} />
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}

function DetailsPopover({ extra }: { extra: Extract<Extra, { type: "details" }> }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="popover-anchor">
      <Button variant="glass" size="sm" icon={<Info size={14} />} onClick={() => setOpen((v) => !v)} aria-expanded={open}>
        {extra.title}
      </Button>
      <AnimatePresence>
        {open && (
          <>
            <div className="popover-catcher" onClick={() => setOpen(false)} />
            <motion.div
              className="popover"
              style={{ transformOrigin: "top right" }}
              initial={{ opacity: 0, scale: 0.92, filter: "blur(6px)" }}
              animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
              exit={{ opacity: 0, scale: 0.95, filter: "blur(4px)" }}
              transition={spring}
            >
              {extra.groups.map((g, i) => (
                <div key={i} className="kv-group">
                  {g.title && <h4>{g.title}</h4>}
                  {g.stats.map((s) => (
                    <div key={s.label} className="kv-row"><span>{s.label}</span><span>{s.value}</span></div>
                  ))}
                </div>
              ))}
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
}

export function DeviceView({ detail }: { detail: DeviceDetail }) {
  const settings = useUi((s) => s.snap!.settings);
  const openSheet = useUi((s) => s.openSheet);
  const showToast = useUi((s) => s.showToast);
  const hero = detail.graphs.filter((g) => g.size === "hero");
  const halves = detail.graphs.filter((g) => g.size === "half");
  const [live, ...info] = detail.stats;
  const actions = detail.extras.find((e) => e.type === "actions") as Extract<Extra, { type: "actions" }> | undefined;
  const popovers = detail.extras.filter((e) => e.type === "details") as Extract<Extra, { type: "details" }>[];

  const copy = async () => {
    const text = await api.deviceInfoText(detail.id);
    if (text) {
      await navigator.clipboard?.writeText(text).catch(() => undefined);
      showToast("Device information copied");
    }
  };

  return (
    <div className="page perf-page">
      <header className="page-header" data-tauri-drag-region>
        <DeviceGlyph kind={detail.kind} title={detail.title} tint={(hero[0]?.series[0]?.tint ?? "cpu")} size={20} />
        <div className="page-titles" data-tauri-drag-region>
          <h1>{detail.title}</h1>
          <p className="page-sub">{detail.model}</p>
        </div>
        <div className="page-actions">
          {detail.kind === "cpu" && (
            <>
              <Segmented
                size="sm"
                value={settings.cpu_graph}
                onChange={(v) => api.updateSettings({ cpu_graph: v })}
                options={[{ value: 1, label: <Sigma size={13} />, title: "Overall" }, { value: 2, label: <Layers2 size={13} />, title: "Per core" }]}
              />
              <Button variant={settings.kernel_times ? "tinted" : "glass"} size="sm" onClick={() => api.updateSettings({ kernel_times: !settings.kernel_times })}>
                Kernel times
              </Button>
            </>
          )}
          {popovers.map((p) => <DetailsPopover key={p.title} extra={p} />)}
          {actions?.smart && (
            <Button variant="glass" size="sm" icon={<HeartPulse size={14} />} onClick={() => openSheet({ kind: "smart", device: detail.id })}>
              Drive health
            </Button>
          )}
          {actions?.eject && (
            <Button variant="glass" size="sm" icon={<Disc size={14} />} onClick={() => openSheet({ kind: "eject", device: detail.id })}>
              Eject
            </Button>
          )}
          <IconButton label="Copy device information (Ctrl+C)" onClick={copy}><Copy size={15} /></IconButton>
        </div>
      </header>

      <div className="perf-grid">
        {hero.map((g) => <GraphCard key={g.id} g={g} settings={settings} className="span-2" />)}

        {live && (
          <section className="card stats-card span-2">
            <div className="stats-row">
              {live.stats.map((s) => <StatView key={s.label} {...s} />)}
            </div>
          </section>
        )}

        <AnimatePresence initial={false}>
          {detail.grid.length > 0 && (
            <motion.div
              key="grid"
              className="core-grid span-2"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 8 }}
              transition={spring}
            >
              {detail.grid.map((g) => <GraphCard key={g.id} g={g} settings={settings} />)}
            </motion.div>
          )}
        </AnimatePresence>

        {halves.map((g, i) => (
          <GraphCard key={g.id} g={g} settings={settings} className={halves.length % 2 === 1 && i === halves.length - 1 ? "span-2" : undefined} />
        ))}

        {detail.extras.map((e, i) =>
          e.type === "composition" ? <div key={i} className="span-2"><Composition extra={e} /></div>
          : e.type === "partitions" ? <div key={i} className="span-2"><Partitions extra={e} /></div>
          : null,
        )}

        {info.map((g, i) => (
          <section key={i} className="card inset-list span-2">
            {g.title && <h3 className="list-title">{g.title}</h3>}
            <div className="kv-grid">
              {g.stats.map((s) => (
                <div key={s.label} className="kv-row"><span>{s.label}</span><span className="selectable">{s.value}</span></div>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}

export function OverviewView() {
  const snap = useUi((s) => s.snap!);
  return (
    <div className="page perf-page">
      <header className="page-header brand-header" data-tauri-drag-region>
        <img className="brand-mark" src={logo} alt="" draggable={false} />
        <div className="page-titles" data-tauri-drag-region>
          <h1 className="brand-title">Respeek</h1>
          <p className="page-sub">Overview · {snap.sidebar.length} devices</p>
        </div>
      </header>
      <div className="overview-grid">
        {snap.sidebar.map((d, i) => (
          <button
            key={d.id}
            className="card overview-card"
            style={{ animationDelay: `${Math.min(i * 28, 220)}ms` }}
            onClick={() => api.selectDevice(d.id)}
          >
            <div className="ov-head">
              <DeviceGlyph kind={d.kind} title={d.title} tint={d.tint} />
              <span className="ov-title">{d.title}</span>
              <span className="ov-level" style={{ color: `var(--tint-${d.tint})` }}>
                <Ticker value={d.level_label} />
              </span>
            </div>
            <p className="ov-sub">{d.subtitle}</p>
            <div className="ov-graph">
              <Graph
                series={[
                  { label: d.title, tint: d.tint, values: d.spark, dashed: false, current: "" },
                  ...(d.spark2 ? [{ label: "", tint: d.tint, values: d.spark2, dashed: true, current: "" }] : []),
                ]}
                sliding={snap.settings.sliding_graphs}
                smooth={snap.settings.smooth_graphs}
                grid={false}
              />
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
