import { useVirtualizer } from "@tanstack/react-virtual";
import { AnimatePresence, motion } from "motion/react";
import {
  ChevronDown, ChevronRight, CirclePause, CirclePlay, Columns3, Info, ListCollapse, OctagonX, Play, Power,
  PowerOff, RotateCw, Search, Square, X,
} from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import { api, type Column, type Row, type TableId, type TableView } from "../api";
import { Button, IconButton, RowIcon, Segmented, Switch } from "../components/primitives";
import { prefersReducedMotion, sheetSpring, snappy, spring } from "../motion";
import { useUi } from "../store";

const HEAT_TINT: Record<string, string> = {
  cpu: "var(--tint-cpu)", memory: "var(--tint-memory)", drive: "var(--tint-disk)",
  network: "var(--tint-network)", gpu: "var(--tint-gpu)",
};

const WIDTH: Record<string, string> = {
  pid: "72px", cpu: "84px", memory: "96px", swap: "88px", drive: "96px", network: "100px", gpu: "72px",
  gpu_memory: "92px", status: "96px", startup: "88px", user: "96px", group: "96px", description: "minmax(160px, 1.3fr)",
};

export function SearchField({ table, autoFocusKey }: { table: TableId; autoFocusKey: number }) {
  const [q, setQ] = useState("");
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (autoFocusKey) ref.current?.focus();
  }, [autoFocusKey]);
  return (
    <label className="search">
      <Search size={14} className="search-icon" />
      <input
        ref={ref}
        value={q}
        placeholder="Search"
        spellCheck={false}
        onChange={(e) => {
          setQ(e.target.value);
          api.setSearch(table, e.target.value);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setQ("");
            api.setSearch(table, "");
            ref.current?.blur();
          }
        }}
      />
      <AnimatePresence>
        {q && (
          <motion.button
            className="search-clear"
            aria-label="Clear search"
            initial={{ opacity: 0, scale: 0.6 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.6 }}
            transition={snappy}
            onClick={() => {
              setQ("");
              api.setSearch(table, "");
            }}
          >
            <X size={10} strokeWidth={3} />
          </motion.button>
        )}
      </AnimatePresence>
    </label>
  );
}

function ColumnsMenu({ table, columns }: { table: TableId; columns: Column[] }) {
  const [open, setOpen] = useState(false);
  const toggle = (id: string, visible: boolean) => {
    const hidden = columns.filter((c) => (c.id === id ? !visible : !c.visible)).map((c) => c.id);
    api.updateSettings(table === "apps" ? { apps_hidden_columns: hidden } : { services_hidden_columns: hidden });
  };
  return (
    <div className="popover-anchor">
      <IconButton label="Columns" onClick={() => setOpen((v) => !v)} active={open}><Columns3 size={15} /></IconButton>
      <AnimatePresence>
        {open && (
          <>
            <div className="popover-catcher" onClick={() => setOpen(false)} />
            <motion.div className="popover menu" style={{ transformOrigin: "top right" }}
              initial={{ opacity: 0, scale: 0.92, filter: "blur(6px)" }} animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
              exit={{ opacity: 0, scale: 0.95 }} transition={spring}>
              <h4>Columns</h4>
              {columns.filter((c) => c.id !== "name").map((c) => (
                <div key={c.id} className="menu-row">
                  <span>{c.title}</span>
                  <Switch checked={c.visible} onChange={(v) => toggle(c.id, v)} label={c.title} />
                </div>
              ))}
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
}

function sameRow(a: Row, b: Row) {
  if (a === b) return true;
  if (a.key !== b.key || a.name !== b.name || a.expanded !== b.expanded || a.expandable !== b.expandable ||
      a.status !== b.status || a.icon !== b.icon || a.cells.length !== b.cells.length) return false;
  for (let i = 0; i < a.cells.length; i++) {
    if (a.cells[i].text !== b.cells[i].text || a.cells[i].heat !== b.cells[i].heat) return false;
  }
  return true;
}

/** Rows are plain elements (no Motion node per row) and skip re-rendering when nothing changed. */
const RowView = memo(function RowView({ row, cols, template, selected, table, animateIn }: {
  row: Row; cols: Column[]; template: string; selected: boolean; table: TableId; animateIn: boolean;
}) {
  const openSheet = useUi((s) => s.openSheet);
  const markExpanded = useUi((s) => s.markExpanded);
  const toggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!row.expanded) markExpanded(row.key);
    api.toggleRow(table, row.key);
  };

  if (row.kind === "section") {
    return (
      <div className="t-section" onClick={toggle}>
        <motion.span className="chev" animate={{ rotate: row.expanded ? 90 : 0 }} transition={snappy}>
          <ChevronRight size={12} strokeWidth={2.5} />
        </motion.span>
        {row.name}
      </div>
    );
  }

  const openDetails = (el: HTMLElement) => {
    const origin = el.getBoundingClientRect();
    if (row.kind === "service") {
      const id = Number(row.key.slice(4));
      if (!Number.isNaN(id)) openSheet({ kind: "service", id, origin });
    } else if (row.pid) {
      openSheet({ kind: "process", pid: row.pid, origin });
    }
  };

  return (
    <div
      className={`t-row ${selected ? "selected" : ""} kind-${row.kind} ${animateIn ? "row-enter" : ""}`}
      style={{ gridTemplateColumns: template }}
      onPointerDown={() => api.selectRow(table, row.key)}
      onDoubleClick={(e) => openDetails(e.currentTarget)}
      role="row"
      aria-selected={selected}
    >
      <div className="t-name" style={{ paddingLeft: 10 + Math.max(0, row.depth - (table === "apps" ? 1 : 0)) * 18 }}>
        {row.expandable ? (
          <button className="chev-btn" onPointerDown={(e) => e.stopPropagation()} onClick={toggle} aria-label={row.expanded ? "Collapse" : "Expand"}>
            <motion.span className="chev" animate={{ rotate: row.expanded ? 90 : 0 }} transition={snappy}>
              <ChevronRight size={12} strokeWidth={2.5} />
            </motion.span>
          </button>
        ) : (
          <span className="chev-space" />
        )}
        {row.kind === "service" ? (
          <span className={`svc-dot st-${row.status}`} />
        ) : (
          <RowIcon icon={row.icon} glyph={row.glyph} />
        )}
        <span className="t-label" title={row.subtitle ?? row.name}>{row.name}</span>
        {row.status === "suspended" && <span className="badge">Suspended</span>}
        {row.status === "failed" && <span className="badge badge-red">Failed</span>}
      </div>
      {cols.map((c, i) => {
        const cell = row.cells[i];
        const heat = cell?.heat ?? 0;
        const tint = HEAT_TINT[c.id];
        return (
          <div
            key={c.id}
            className={`t-cell ${c.numeric ? "num" : ""} col-${c.id}`}
            style={tint && heat > 0.01 ? ({ "--heat": Math.min(heat, 1), "--heat-tint": tint } as React.CSSProperties) : undefined}
          >
            {cell?.text}
          </div>
        );
      })}
    </div>
  );
}, (a, b) =>
  a.selected === b.selected && a.template === b.template && a.table === b.table &&
  a.animateIn === b.animateIn && a.cols === b.cols && sameRow(a.row, b.row));

function ActionBar({ view, table }: { view: TableView; table: TableId }) {
  const sel = view.selection;
  const openSheet = useUi((s) => s.openSheet);
  if (!sel) return null;
  const isService = sel.service_id !== null;
  return (
    <motion.div
      className="action-bar"
      initial={{ y: 70, opacity: 0, scale: 0.96, filter: "blur(8px)" }}
      animate={{ y: 0, opacity: 1, scale: 1, filter: "blur(0px)" }}
      exit={{ y: 70, opacity: 0, scale: 0.96, filter: "blur(8px)" }}
      transition={sheetSpring}
    >
      <span className="ab-title">{sel.title}</span>
      <span className="ab-sep" />
      {isService ? (
        <>
          {sel.service_running ? (
            <Button size="sm" variant="plain" icon={<Square size={13} />} onClick={() => api.serviceAction(sel.service_id!, "stop")}>Stop</Button>
          ) : (
            <Button size="sm" variant="plain" icon={<Play size={13} />} onClick={() => api.serviceAction(sel.service_id!, "start")}>Start</Button>
          )}
          <Button size="sm" variant="plain" icon={<RotateCw size={13} />} onClick={() => api.serviceAction(sel.service_id!, "restart")}>Restart</Button>
          {sel.service_enabled ? (
            <Button size="sm" variant="plain" icon={<PowerOff size={13} />} onClick={() => api.serviceAction(sel.service_id!, "disable")}>Disable</Button>
          ) : (
            <Button size="sm" variant="plain" icon={<Power size={13} />} onClick={() => api.serviceAction(sel.service_id!, "enable")}>Enable</Button>
          )}
          <Button size="sm" variant="plain" icon={<Info size={13} />} onClick={() => openSheet({ kind: "service", id: sel.service_id! })}>Details</Button>
        </>
      ) : (
        <>
          {!sel.is_app && sel.pids[0] !== undefined && (
            <Button size="sm" variant="plain" icon={<Info size={13} />} onClick={() => openSheet({ kind: "process", pid: sel.pids[0] })}>Details</Button>
          )}
          {sel.suspended ? (
            <Button size="sm" variant="plain" icon={<CirclePlay size={13} />} onClick={() => api.signal(sel.pids, "continue")}>Resume</Button>
          ) : (
            <Button size="sm" variant="plain" icon={<CirclePause size={13} />} onClick={() => api.signal(sel.pids, "suspend")}>Suspend</Button>
          )}
          <Button
            size="sm"
            variant="destructive"
            icon={<OctagonX size={13} />}
            onClick={() => openSheet({ kind: "confirm-quit", title: sel.title, pids: sel.pids, isApp: sel.is_app })}
          >
            {sel.is_app ? "Quit" : "End"}
          </Button>
        </>
      )}
      <IconButton label="Deselect" onClick={() => api.selectRow(table, null)}><X size={14} /></IconButton>
    </motion.div>
  );
}

export function TablePage({ table, view, userServices }: { table: TableId; view: TableView; userServices?: boolean }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [focusKey, setFocusKey] = useState(0);
  const justExpanded = useUi((s) => s.justExpanded);
  const openSheet = useUi((s) => s.openSheet);
  // Stable across ticks (columns arrive as new objects every snapshot) so memoized rows can skip.
  const colSig = view.columns.map((c) => `${c.id}:${c.visible ? 1 : 0}`).join(",");
  const cols = useMemo(() => view.columns.filter((c) => c.visible && c.id !== "name"), [colSig]); // eslint-disable-line react-hooks/exhaustive-deps
  const template = useMemo(() => ["minmax(200px, 2fr)", ...cols.map((c) => WIDTH[c.id] ?? "96px")].join(" "), [cols]);
  // Minimum table width: name column + fixed columns (+ row padding); narrower windows scroll sideways.
  const minWidth = useMemo(
    () => 200 + 16 + cols.reduce((n, c) => n + (parseInt(WIDTH[c.id] ?? "96", 10) || 160), 0),
    [cols],
  );
  const rows = view.rows;
  const reduced = prefersReducedMotion();

  const virt = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (rows[i]?.kind === "section" ? 34 : 32),
    getItemKey: (i) => rows[i]?.key ?? i,
    overscan: 14,
  });

  // Keyboard: arrows move/expand, Enter opens details, Delete ends, Ctrl+F/L handled here too.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (useUi.getState().sheet) return;
      const target = e.target as HTMLElement;
      const typing = target.tagName === "INPUT" || target.tagName === "TEXTAREA";
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFocusKey((k) => k + 1);
        return;
      }
      if (ctrl && e.key.toLowerCase() === "l") {
        e.preventDefault();
        api.collapseAll(table);
        return;
      }
      const sel = view.selection;
      if (ctrl && e.key.toLowerCase() === "i" && sel) {
        e.preventDefault();
        if (sel.service_id !== null) openSheet({ kind: "service", id: sel.service_id });
        else if (sel.pids[0] !== undefined && !sel.is_app) openSheet({ kind: "process", pid: sel.pids[0] });
        return;
      }
      if (table === "apps" && sel && ctrl && (e.key.toLowerCase() === "e" || e.key.toLowerCase() === "x")) {
        e.preventDefault();
        if (e.key.toLowerCase() === "x") api.signal(sel.pids, "kill");
        else openSheet({ kind: "confirm-quit", title: sel.title, pids: sel.pids, isApp: sel.is_app });
        return;
      }
      if (table === "services" && sel?.service_id != null && ctrl && ["s", "e", "r"].includes(e.key.toLowerCase())) {
        e.preventDefault();
        api.serviceAction(sel.service_id, ({ s: "start", e: "stop", r: "restart" } as const)[e.key.toLowerCase() as "s" | "e" | "r"]);
        return;
      }
      if (typing) return;
      const idx = rows.findIndex((r) => r.key === sel?.key);
      const move = (d: number) => {
        let i = idx;
        do i += d;
        while (rows[i] && rows[i].kind === "section");
        if (rows[i]) {
          api.selectRow(table, rows[i].key);
          virt.scrollToIndex(i, { align: "auto" });
        }
      };
      if (e.key === "ArrowDown") { e.preventDefault(); move(1); }
      else if (e.key === "ArrowUp") { e.preventDefault(); move(-1); }
      else if ((e.key === "ArrowRight" || e.key === "ArrowLeft") && idx >= 0) {
        const r = rows[idx];
        if (r.expandable && r.expanded === (e.key === "ArrowLeft")) {
          if (!r.expanded) useUi.getState().markExpanded(r.key);
          api.toggleRow(table, r.key);
        }
      } else if (e.key === "Delete" && sel && table === "apps") {
        openSheet({ kind: "confirm-quit", title: sel.title, pids: sel.pids, isApp: sel.is_app });
      } else if (e.key === "Escape" && sel) {
        api.selectRow(table, null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rows, view.selection, table, virt, openSheet]);

  const animatingParent =
    !reduced && justExpanded && performance.now() - justExpanded.at < 450 ? justExpanded.key + "/" : null;
  const sortIcon = (c: Column) =>
    view.sort_column === c.id ? (
      <motion.span className="sort-ind" animate={{ rotate: view.sort_ascending ? 180 : 0 }} transition={snappy}>
        <ChevronDown size={11} strokeWidth={2.6} />
      </motion.span>
    ) : null;

  return (
    <div className="page table-page">
      <header className="page-header" data-tauri-drag-region>
        <div className="page-titles" data-tauri-drag-region>
          <h1>{view.headline}</h1>
          <p className="page-sub">{view.subheadline}</p>
        </div>
        <div className="page-actions">
          {table === "services" && (
            <Segmented
              size="sm"
              value={userServices ? "user" : "system"}
              onChange={(v) => api.setServicesUser(v === "user")}
              options={[{ value: "system", label: "System" }, { value: "user", label: "User" }]}
            />
          )}
          <IconButton label="Collapse all (Ctrl+L)" onClick={() => api.collapseAll(table)}><ListCollapse size={15} /></IconButton>
          <ColumnsMenu table={table} columns={view.columns} />
          <SearchField table={table} autoFocusKey={focusKey} />
        </div>
      </header>

      {view.notice && <div className="notice">{view.notice}</div>}

      <div className="table card">
        <div className="t-inner" style={{ minWidth }}>
        <div className="t-head" style={{ gridTemplateColumns: template }} role="row">
          {view.columns.filter((c) => c.visible).map((c) => (
            <button key={c.id} className={`t-hcell ${c.numeric ? "num" : ""} ${view.sort_column === c.id ? "sorted" : ""}`} onClick={() => api.sortBy(table, c.id)}>
              <span className="th-title">{c.title}{sortIcon(c)}</span>
              {c.total && <span className="th-total">{c.total}</span>}
            </button>
          ))}
        </div>
        <div className="t-scroll" ref={scrollRef} role="grid">
          <div style={{ height: virt.getTotalSize(), position: "relative" }}>
            {virt.getVirtualItems().map((vi) => {
              const row = rows[vi.index];
              if (!row) return null;
              return (
                <div key={vi.key} className="t-vrow" style={{ transform: `translateY(${vi.start}px)`, height: vi.size }}>
                  <RowView
                    row={row}
                    cols={cols}
                    template={template}
                    table={table}
                    selected={view.selection?.key === row.key}
                    animateIn={!!animatingParent && row.key.startsWith(animatingParent)}
                  />
                </div>
              );
            })}
          </div>
          {rows.length === 0 && <div className="empty">No results</div>}
        </div>
        </div>
      </div>

      <AnimatePresence>{view.selection && <ActionBar key="ab" view={view} table={table} />}</AnimatePresence>
    </div>
  );
}
