import { AnimatePresence, motion } from "motion/react";
import { CirclePause, CirclePlay, Ellipsis, Info, Keyboard, MonitorSmartphone, PanelLeft, Settings as Gear } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { api, isTauri, windowControl, type Snapshot, type Tint } from "./api";
import { IconButton } from "./components/primitives";
import { Sidebar, WindowControls } from "./components/Sidebar";
import { fade, prefersReducedMotion, spring } from "./motion";
import { DeviceView, OverviewView } from "./pages/Performance";
import { TablePage } from "./pages/TablePage";
import { SheetHost } from "./sheets/Sheets";
import { useUi } from "./store";

/** Order index of the current view, so page transitions move in the sidebar's direction. */
function viewKey(s: Snapshot): { key: string; index: number; tint: Tint } {
  const p = s.page;
  if (p.page === "performance") {
    if (p.overview || !p.detail) return { key: "overview", index: -1, tint: "cpu" };
    const i = s.sidebar.findIndex((d) => d.id === p.detail!.id);
    return { key: `dev:${p.detail.id}`, index: i, tint: s.sidebar[i]?.tint ?? "cpu" };
  }
  if (p.page === "apps") return { key: "apps", index: 100, tint: "cpu" };
  return { key: "services", index: 101, tint: "gpu_memory" };
}

function MoreMenu() {
  const [open, setOpen] = useState(false);
  const openSheet = useUi((s) => s.openSheet);
  const item = (label: string, icon: React.ReactNode, fn: () => void) => (
    <button className="menu-item" onClick={() => { setOpen(false); fn(); }}>{icon}<span>{label}</span></button>
  );
  return (
    <div className="popover-anchor">
      <IconButton label="More" active={open} onClick={() => setOpen((v) => !v)}><Ellipsis size={16} /></IconButton>
      <AnimatePresence>
        {open && (
          <>
            <div className="popover-catcher" onClick={() => setOpen(false)} />
            <motion.div className="popover menu" style={{ transformOrigin: "top right" }}
              initial={{ opacity: 0, scale: 0.9, filter: "blur(6px)" }} animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
              exit={{ opacity: 0, scale: 0.95 }} transition={spring}>
              {item("Preferences…", <Gear size={14} />, () => openSheet({ kind: "preferences" }))}
              {item("About This Computer", <MonitorSmartphone size={14} />, () => openSheet({ kind: "about-system" }))}
              {item("Keyboard Shortcuts", <Keyboard size={14} />, () => openSheet({ kind: "shortcuts" }))}
              <div className="menu-sep" />
              {item("About Respeek", <Info size={14} />, () => openSheet({ kind: "about" }))}
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
}

function Toolbar() {
  const snap = useUi((s) => s.snap);
  const collapsed = useUi((s) => s.sidebarCollapsed);
  const toggleSidebar = useUi((s) => s.toggleSidebar);
  const paused = snap?.paused ?? false;
  return (
    <div className="toolbar" data-tauri-drag-region onDoubleClick={(e) => {
      if ((e.target as HTMLElement).hasAttribute("data-tauri-drag-region")) windowControl("maximize");
    }}>
      <IconButton label="Toggle sidebar (Ctrl+T)" active={collapsed} onClick={toggleSidebar}><PanelLeft size={16} /></IconButton>
      <span className="grow" data-tauri-drag-region />
      <IconButton label={paused ? "Resume updates" : "Pause updates"} active={paused} onClick={() => api.setPaused(!paused)}>
        {paused ? <CirclePlay size={16} /> : <CirclePause size={16} />}
      </IconButton>
      <MoreMenu />
      <span className="tb-sep" />
      <WindowControls />
    </div>
  );
}

/** Page enter: critically damped, short travel. Exit: quick fade so two full pages never compete. */
const pageIn = { y: { type: "spring", bounce: 0, duration: 0.5 }, opacity: { duration: 0.22, ease: [0.2, 0, 0, 1] } } as const;
const pageOut = { opacity: { duration: 0.12, ease: "linear" } } as const;

/**
 * Mounts heavy page content one frame after the page container, so the first animation frame
 * is painted immediately instead of waiting for tables and canvases to build.
 */
function DeferredPage({ children }: { children: React.ReactNode }) {
  const [ready, setReady] = useState(false);
  useEffect(() => {
    const id = requestAnimationFrame(() => setReady(true));
    return () => cancelAnimationFrame(id);
  }, []);
  return ready ? <>{children}</> : <div className="page page-placeholder" />;
}

function Ambient({ tint }: { tint: Tint }) {
  // Soft colour field behind the glass, tinted by the current device. Each tint is its own
  // layer that is painted once and cross-faded by opacity (compositor-only) — animating the
  // gradient colour would re-rasterize a huge layer every frame.
  return (
    <div className="ambient" aria-hidden>
      <AnimatePresence initial={false}>
        <motion.div
          key={tint}
          className="amb amb-a"
          style={{ color: `var(--tint-${tint})` }}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.7, ease: [0.2, 0, 0, 1] }}
        />
      </AnimatePresence>
      <div className="amb amb-b" />
      <div className="amb amb-c" />
    </div>
  );
}

export default function App() {
  const snap = useUi((s) => s.snap);
  const setSnap = useUi((s) => s.setSnap);
  const openSheet = useUi((s) => s.openSheet);
  const sheet = useUi((s) => s.sheet);
  const toast = useUi((s) => s.toast);
  const firstRunShown = useRef(false);
  const contentRef = useRef<HTMLElement>(null);
  const lastKey = useRef("");

  useEffect(() => {
    api.subscribe(setSnap).then(setSnap);
    if (isTauri) {
      import("@tauri-apps/api/event").then(({ listen }) =>
        listen<string>("fatal", (e) => useUi.getState().openSheet({ kind: "fatal", message: e.payload })),
      );
    }
  }, [setSnap]);

  // Theme, accent and transparency preferences → <html> attributes.
  useEffect(() => {
    if (!snap) return;
    const root = document.documentElement;
    const s = snap.settings;
    if (s.appearance === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", s.appearance);
    root.setAttribute("data-accent", s.accent);
    root.setAttribute("data-solid", String(s.reduce_transparency));
    if (s.first_time_running && !firstRunShown.current) {
      firstRunShown.current = true;
      openSheet({ kind: "first-run" });
    }
  }, [snap?.settings, openSheet]); // eslint-disable-line react-hooks/exhaustive-deps

  // Global shortcuts.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      const k = e.key.toLowerCase();
      const ui = useUi.getState();
      if (e.key === "Escape" && ui.sheet && ui.sheet.kind !== "first-run" && ui.sheet.kind !== "fatal") {
        ui.closeSheet();
        return;
      }
      if (ctrl && (k === "q" || k === "w")) { e.preventDefault(); api.quit(); }
      else if (ctrl && k === ",") { e.preventDefault(); ui.openSheet({ kind: "preferences" }); }
      else if (ctrl && (k === "?" || (e.shiftKey && k === "/"))) { e.preventDefault(); ui.openSheet({ kind: "shortcuts" }); }
      else if ((ctrl || e.altKey) && k === "1") { e.preventDefault(); api.setPage("performance"); }
      else if ((ctrl || e.altKey) && k === "2") { e.preventDefault(); api.setPage("apps"); }
      else if ((ctrl || e.altKey) && k === "3") { e.preventDefault(); api.setPage("services"); }
      else if (ctrl && k === "a" && (e.target as HTMLElement).tagName !== "INPUT") { e.preventDefault(); ui.openSheet({ kind: "about-system" }); }
      else if ((ctrl && k === "t") || e.key === "F9") { e.preventDefault(); ui.toggleSidebar(); }
      else if (ctrl && k === "m") {
        e.preventDefault();
        const p = ui.snap?.page;
        api.setOverview(!(p?.page === "performance" && p.overview));
      } else if (/^F[1-8]$/.test(e.key)) {
        const d = ui.snap?.sidebar[Number(e.key.slice(1)) - 1];
        if (d) { e.preventDefault(); api.selectDevice(d.id); }
      } else if (ctrl && k === "c" && ui.snap?.page.page === "performance" && !window.getSelection()?.toString()) {
        const id = ui.snap.page.detail?.id;
        if (id)
          api.deviceInfoText(id).then(async (t) => {
            if (!t) return;
            await navigator.clipboard?.writeText(t).catch(() => undefined);
            ui.showToast("Device information copied");
          });
      } else if (e.key === "Control" && !e.repeat && !ui.sheet) {
        // Hold Ctrl to pause (upstream behaviour) — only after a short hold so shortcuts don't flicker it.
        holdTimer.current = window.setTimeout(() => api.setPaused(true), 600);
      }
    };
    const onUp = (e: KeyboardEvent) => {
      if (e.key === "Control" && holdTimer.current) {
        clearTimeout(holdTimer.current);
        holdTimer.current = 0;
        if (useUi.getState().snap?.paused) api.setPaused(false);
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onUp);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onUp);
    };
  }, []);
  const holdTimer = useRef(0);

  if (!snap) {
    return (
      <div className="window booting">
        <div className="boot">
          <img src="/respeek.svg" width={84} height={84} alt="" />
          <span className="spinner" />
        </div>
      </div>
    );
  }

  const v = viewKey(snap);
  if (lastKey.current !== v.key) {
    lastKey.current = v.key;
    contentRef.current?.classList.remove("scrolled");
  }
  const reduced = prefersReducedMotion();
  const p = snap.page;
  return (
    <div className={`window ${sheet ? "sheet-open" : ""}`}>
      <Ambient tint={v.tint} />
      <Sidebar />
      <main
        className="content"
        ref={contentRef}
        onScrollCapture={(e) => {
          const t = e.target as HTMLElement;
          if (!t.classList.contains("stage-page")) return;
          // Toggle a class directly: no React render on scroll.
          contentRef.current?.classList.toggle("scrolled", t.scrollTop > 4);
        }}
      >
        <Toolbar />
        <div className="content-stage">
          <AnimatePresence initial={false}>
            <motion.div
              key={v.key}
              className="stage-page"
              // New page rises from below on a critically damped spring; the old one simply fades
              // in place underneath, so only transform + opacity ever animate (GPU-only).
              initial={reduced ? { opacity: 0 } : { opacity: 0, y: 36 }}
              animate={{ opacity: 1, y: 0, transition: reduced ? fade : pageIn }}
              exit={{ opacity: 0, zIndex: 0, transition: pageOut }}
              style={{ zIndex: 1 }}
            >
              <DeferredPage>
                {p.page === "performance" ? (p.overview || !p.detail ? <OverviewView /> : <DeviceView detail={p.detail} />)
                  : p.page === "apps" ? <TablePage table="apps" view={p.table} />
                  : <TablePage table="services" view={p.table} userServices={p.user} />}
              </DeferredPage>
            </motion.div>
          </AnimatePresence>
        </div>
      </main>
      <SheetHost />
      <AnimatePresence>
        {toast && (
          <motion.div className="toast" initial={{ opacity: 0, y: 16, scale: 0.96 }} animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8 }} transition={spring}>{toast}</motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
