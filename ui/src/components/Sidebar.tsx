import { motion, Reorder, useDragControls } from "motion/react";
import { AppWindow, GripVertical, LayoutGrid, Server } from "lucide-react";
import { memo, useEffect, useRef, useState } from "react";
import { api, windowControl } from "../api";
import { momentum, spring } from "../motion";
import { useUi } from "../store";
import { Graph } from "./Graph";
import { DeviceGlyph } from "./primitives";

/** Window controls in the platform's right-hand order: minimize, maximize, close. */
export function WindowControls() {
  return (
    <div className="traffic-lights" onPointerDown={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
      <button className="tl tl-min" aria-label="Minimize" title="Minimize" onClick={() => windowControl("minimize")}>
        <svg viewBox="0 0 12 12"><path d="M3 6h6" /></svg>
      </button>
      <button className="tl tl-max" aria-label="Maximize" title="Maximize" onClick={() => windowControl("maximize")}>
        <svg viewBox="0 0 12 12"><path d="M4 8V4.6L7.4 8zM8 4v3.4L4.6 4z" className="fill" /></svg>
      </button>
      <button className="tl tl-close" aria-label="Close" title="Close" onClick={() => windowControl("close")}>
        <svg viewBox="0 0 12 12"><path d="M3.5 3.5l5 5M8.5 3.5l-5 5" /></svg>
      </button>
    </div>
  );
}

/** Live contents of a device row. Subscribes to its own device so ticks don't re-render (and
 * re-measure) the Reorder wrappers. */
function DeviceRowContent({ id }: { id: string }) {
  const d = useUi((s) => s.snap?.sidebar.find((x) => x.id === id));
  if (!d) return null;
  const sparks = [{ label: d.title, tint: d.tint, values: d.spark, dashed: false, current: "" }];
  if (d.spark2) sparks.push({ label: "", tint: d.tint, values: d.spark2, dashed: true, current: "" });
  return (
    <>
      <DeviceGlyph kind={d.kind} title={d.title} tint={d.tint} />
      <span className="sb-text">
        <span className="sb-title">{d.title}</span>
        <span className="sb-sub">{d.subtitle}</span>
      </span>
      <span className="sb-spark">
        <Graph series={sparks} spark grid={false} sliding={false} />
      </span>
    </>
  );
}

const DeviceItem = memo(function DeviceItem({ id, active }: { id: string; active: boolean }) {
  const controls = useDragControls();
  return (
    <Reorder.Item
      value={id}
      dragListener={false}
      dragControls={controls}
      transition={momentum}
      className="sb-item-wrap"
      whileDrag={{ scale: 1.03, zIndex: 5, boxShadow: "var(--shadow-float)" }}
    >
      <button className={`sb-item sb-device ${active ? "active" : ""}`} onClick={() => api.selectDevice(id)} aria-current={active}>
        {active && <motion.span className="sb-pill" layoutId="sb-pill" transition={spring} />}
        <DeviceRowContent id={id} />
        <span
          className="sb-grip"
          onPointerDown={(e) => {
            e.stopPropagation();
            controls.start(e);
          }}
          title="Drag to reorder"
        >
          <GripVertical size={12} />
        </span>
      </button>
    </Reorder.Item>
  );
});

function NavItem({ active, label, glyph, icon, onClick, className }: {
  active: boolean; label: string; glyph: string; icon: React.ReactNode; onClick: () => void; className?: string;
}) {
  return (
    <button className={`sb-item ${active ? "active" : ""} ${className ?? ""}`} onClick={onClick} aria-current={active}>
      {active && <motion.span className="sb-pill" layoutId="sb-pill" transition={spring} />}
      <span className="device-glyph" style={{ "--glyph": glyph } as React.CSSProperties}>{icon}</span>
      <span className="sb-text"><span className="sb-title">{label}</span></span>
    </button>
  );
}

export function Sidebar() {
  const collapsed = useUi((s) => s.sidebarCollapsed);
  // Only re-render when the device list or the selection changes — not on every tick.
  const ids = useUi((s) => (s.snap?.sidebar ?? []).map((d) => d.id).join("|"));
  const nav = useUi((s) => {
    const p = s.snap?.page;
    if (!p) return "";
    if (p.page === "performance") return p.overview || !p.detail ? "overview" : `dev:${p.detail.id}`;
    return p.page;
  });
  const [order, setOrder] = useState<string[]>([]);
  const dragging = useRef(false);

  // Follow the core's order except while the user is dragging.
  useEffect(() => {
    if (!dragging.current) setOrder(ids ? ids.split("|") : []);
  }, [ids]);

  const activeDevice = nav.startsWith("dev:") ? nav.slice(4) : null;
  const overviewActive = nav === "overview";

  return (
    <motion.aside
      className="sidebar"
      initial={false}
      animate={{ width: collapsed ? 0 : 262, opacity: collapsed ? 0 : 1 }}
      transition={spring}
    >
      <div className="sidebar-inner">
        <div className="sidebar-top" data-tauri-drag-region />
        <nav className="sb-scroll">
          <NavItem
            active={!!overviewActive}
            label="Overview"
            glyph="var(--accent)"
            icon={<LayoutGrid size={15} strokeWidth={1.9} />}
            onClick={() => api.setOverview(true)}
            className="sb-overview"
          />

          <div className="sb-heading">Activity</div>
          <NavItem active={nav === "apps"} label="Apps" glyph="var(--tint-cpu)" icon={<AppWindow size={15} strokeWidth={1.9} />} onClick={() => api.setPage("apps")} />
          <NavItem active={nav === "services"} label="Services" glyph="var(--tint-gpu_memory)" icon={<Server size={15} strokeWidth={1.9} />} onClick={() => api.setPage("services")} />

          <div className="sb-heading">Monitor</div>
          <Reorder.Group
            axis="y"
            values={order}
            onReorder={(o) => {
              dragging.current = true;
              setOrder(o);
            }}
            onPointerUp={() => {
              if (dragging.current) {
                dragging.current = false;
                api.updateSettings({ sidebar_order: order });
              }
            }}
            className="sb-list"
          >
            {order.map((id) => (
              <DeviceItem key={id} id={id} active={activeDevice === id} />
            ))}
          </Reorder.Group>
        </nav>
      </div>
    </motion.aside>
  );
}
