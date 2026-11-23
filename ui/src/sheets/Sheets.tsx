import { AnimatePresence, motion } from "motion/react";
import { AlertTriangle, Check, Cpu, HardDrive, Keyboard, MonitorSmartphone, OctagonX, Server, Sparkles } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import {
  api, type AboutSystem, type EjectFailure, type ProcessDetails, type ServiceDetails, type SetupScriptState,
  type SmartView, type StatGroup,
} from "../api";
import { Button, RowIcon, StatView } from "../components/primitives";
import { useUi, type SheetState } from "../store";
import { Loading, Sheet } from "./Sheet";
import { Preferences } from "./Preferences";
import logo from "../assets/respeek.svg";

function useAsync<T>(fn: () => Promise<T>, deps: unknown[]) {
  const [v, setV] = useState<T | undefined>(undefined);
  useEffect(() => {
    let alive = true;
    fn().then((r) => alive && setV(r)).catch(() => alive && setV(undefined));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return v;
}

function Groups({ groups }: { groups: StatGroup[] }) {
  return (
    <>
      {groups.map((g, i) =>
        g.stats.some((s) => s.primary) ? (
          <div key={i} className="stats-row sheet-stats">
            {g.stats.map((s) => <StatView key={s.label} {...s} />)}
          </div>
        ) : (
          <section key={i} className="inset-list">
            {g.title && <h3 className="list-title">{g.title}</h3>}
            <div className="kv-grid">
              {g.stats.map((s) => (
                <div key={s.label} className="kv-row"><span>{s.label}</span><span className="selectable">{s.value}</span></div>
              ))}
            </div>
          </section>
        ),
      )}
    </>
  );
}

function ProcessSheet({ pid, origin }: { pid: number; origin?: DOMRect }) {
  const seq = useUi((s) => s.snap?.seq);
  const [d, setD] = useState<ProcessDetails | null | undefined>(undefined);
  useEffect(() => {
    api.processDetails(pid).then(setD);
  }, [pid, seq]);
  const open = useUi((s) => s.openSheet);
  return (
    <Sheet
      origin={origin}
      icon={<RowIcon icon={d?.icon ?? null} glyph="process" size={34} />}
      title={d?.name ?? "Process"}
      subtitle={d ? `PID ${d.pid} · ${d.state}` : undefined}
      footer={d && (
        <>
          <Button variant="plain" onClick={() => api.signal([pid], "suspend")}>Suspend</Button>
          <Button variant="plain" onClick={() => api.signal([pid], "continue")}>Resume</Button>
          <span className="grow" />
          <Button variant="destructive" icon={<OctagonX size={14} />} onClick={() => open({ kind: "confirm-quit", title: d.name, pids: [pid], isApp: false })}>
            End process…
          </Button>
        </>
      )}
    >
      {d === undefined ? <Loading /> : d === null ? <p className="muted">This process has exited.</p> : (
        <>
          <Groups groups={d.groups} />
          <section className="inset-list">
            <h3 className="list-title">Command line</h3>
            <pre className="code selectable">{d.command_line || d.exe || "—"}</pre>
          </section>
        </>
      )}
    </Sheet>
  );
}

function ServiceSheet({ id, origin }: { id: number; origin?: DOMRect }) {
  const d = useAsync<ServiceDetails | null>(() => api.serviceDetails(id), [id]);
  return (
    <Sheet origin={origin} width={680} icon={<span className="sheet-glyph"><Server size={20} /></span>} title={d?.name ?? "Service"} subtitle={d?.description}>
      {d === undefined ? <Loading /> : d === null ? <p className="muted">Service not found.</p> : (
        <>
          <Groups groups={d.groups} />
          <section className="inset-list">
            <h3 className="list-title">Logs</h3>
            <pre className="code logs selectable">{d.logs || "No log entries."}</pre>
          </section>
        </>
      )}
    </Sheet>
  );
}

function SmartSheet({ device }: { device: string }) {
  const d = useAsync<SmartView>(() => api.smartData(device), [device]);
  return (
    <Sheet width={700} icon={<span className="sheet-glyph tint-disk"><HardDrive size={20} /></span>} title="Drive health" subtitle={d?.title}>
      {d === undefined ? <Loading /> : !d.available ? (
        <p className="muted">S.M.A.R.T. data is not available for this drive.</p>
      ) : (
        <>
          <Groups groups={d.groups} />
          {d.attributes.length > 0 && (
            <section className="inset-list">
              <h3 className="list-title">Attributes</h3>
              <div className="smart-table">
                <div className="smart-row head">{d.attributes_header.map((h) => <span key={h}>{h}</span>)}</div>
                {d.attributes.map((r, i) => (
                  <div key={i} className="smart-row">{r.map((c, j) => <span key={j}>{c}</span>)}</div>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </Sheet>
  );
}

function EjectSheet({ device }: { device: string }) {
  const close = useUi((s) => s.closeSheet);
  const [state, setState] = useState<"working" | "ok" | EjectFailure>("working");
  useEffect(() => {
    api.ejectDisk(device).then(() => setState("ok")).catch((e: EjectFailure) => setState(e));
  }, [device]);
  useEffect(() => {
    if (state === "ok") {
      const t = setTimeout(close, 900);
      return () => clearTimeout(t);
    }
  }, [state, close]);
  return (
    <Sheet width={480} title={state === "ok" ? "Ejected" : typeof state === "object" ? "Couldn’t eject the drive" : "Ejecting…"}
      icon={<span className={`sheet-glyph ${typeof state === "object" ? "tint-red" : "tint-disk"}`}>{state === "ok" ? <Check size={20} /> : typeof state === "object" ? <AlertTriangle size={20} /> : <HardDrive size={20} />}</span>}
      subtitle={typeof state === "object" ? "These apps are still using files on the drive." : undefined}
      footer={typeof state === "object" && (
        <>
          <Button variant="plain" onClick={close}>Cancel</Button>
          <span className="grow" />
          <Button variant="destructive" onClick={() => api.signal(state.blockers.map((b) => b.pid), "kill").then(() => setState("working")).then(() => api.ejectDisk(device)).then(() => setState("ok")).catch((e) => setState(e))}>
            Force quit all &amp; eject
          </Button>
        </>
      )}
    >
      {state === "working" ? <Loading /> : state === "ok" ? <p className="muted">It’s safe to remove the drive.</p> : (
        <ul className="blockers">
          {state.blockers.map((b) => (
            <li key={b.pid}>
              <RowIcon icon={b.icon} glyph="process" size={22} />
              <div>
                <strong>{b.name}</strong> <span className="muted">PID {b.pid}</span>
                {b.files.slice(0, 3).map((f) => <div key={f} className="code-inline">{f}</div>)}
              </div>
              <Button size="sm" variant="plain" onClick={() => api.signal([b.pid], "kill")}>Force quit</Button>
            </li>
          ))}
        </ul>
      )}
    </Sheet>
  );
}

/** macOS-style quit confirmation: Cancel · Force Quit · Quit. */
function ConfirmQuit({ title, pids, isApp }: { title: string; pids: number[]; isApp: boolean }) {
  const close = useUi((s) => s.closeSheet);
  const act = (signal: "terminate" | "kill") => {
    api.signal(pids, signal);
    close();
  };
  return (
    <Sheet
      width={420}
      icon={<span className="sheet-glyph tint-red"><OctagonX size={20} /></span>}
      title={`${isApp ? "Quit" : "End"} “${title}”?`}
      subtitle={isApp ? "Unsaved changes in this app may be lost." : "Ending a system process can make your computer unstable."}
      footer={
        <>
          <Button variant="plain" onClick={close}>Cancel</Button>
          <span className="grow" />
          <Button variant="plain" className="danger-text" onClick={() => act("kill")}>Force Quit</Button>
          <Button variant="filled" autoFocus onClick={() => act("terminate")}>{isApp ? "Quit" : "End"}</Button>
        </>
      }
    >
      {null}
    </Sheet>
  );
}

function AboutSystemSheet() {
  const d = useAsync<AboutSystem | null>(() => api.aboutSystem(), []);
  return (
    <Sheet width={520} icon={d?.logo ? <img src={d.logo} width={40} height={40} alt="" /> : <span className="sheet-glyph"><MonitorSmartphone size={20} /></span>} title={d?.title ?? "About this computer"}>
      {d === undefined ? <Loading /> : d === null ? <p className="muted">Unavailable.</p> : <Groups groups={d.groups} />}
    </Sheet>
  );
}

function AboutSheet() {
  return (
    <Sheet width={420} title="Respeek" subtitle="Version 0.1.0" icon={<img src={logo} width={44} height={44} alt="" />}>
      <p className="about-text">A fluid, glass-clear system monitor for Linux.</p>
      <p className="about-text muted">
        Respeek is built on the Rust core of <strong>Mission Center</strong> (Magpie gatherer and client) by the
        Mission Center developers, with a new interface. Licensed under the GNU GPL v3 or later.
      </p>
    </Sheet>
  );
}

const SHORTCUTS: [string, [string, string][]][] = [
  ["General", [["Preferences", "Ctrl ,"], ["Keyboard shortcuts", "Ctrl ?"], ["Quit", "Ctrl Q"], ["Toggle sidebar", "Ctrl T"], ["About this computer", "Ctrl A"], ["Pause updates", "Space (hold Ctrl)"]]],
  ["Navigation", [["Performance", "Ctrl 1"], ["Apps", "Ctrl 2"], ["Services", "Ctrl 3"], ["Select device", "F1 … F8"], ["Overview", "Ctrl M"], ["Copy device information", "Ctrl C"]]],
  ["Apps & Services", [["Search", "Ctrl F"], ["Collapse all", "Ctrl L"], ["Details", "Ctrl I"], ["Quit app / end process", "Ctrl E"], ["Force quit", "Ctrl X"], ["Start / stop / restart service", "Ctrl S / E / R"], ["Move selection", "↑ ↓"], ["Expand / collapse", "→ ←"]]],
];

function ShortcutsSheet() {
  return (
    <Sheet width={560} title="Keyboard Shortcuts" icon={<span className="sheet-glyph"><Keyboard size={20} /></span>}>
      {SHORTCUTS.map(([group, items]) => (
        <section key={group} className="inset-list">
          <h3 className="list-title">{group}</h3>
          <div className="kv-grid">
            {items.map(([label, keys]) => (
              <div key={label} className="kv-row"><span>{label}</span><span className="keys">{keys.split(" ").map((k, i) => <kbd key={i}>{k}</kbd>)}</span></div>
            ))}
          </div>
        </section>
      ))}
    </Sheet>
  );
}

export function SetupScriptPanel() {
  const [st, setSt] = useState<SetupScriptState | undefined>(undefined);
  const refresh = () => api.setupScriptState().then(setSt);
  useEffect(() => {
    refresh();
  }, []);
  useEffect(() => {
    if (st?.state !== "running") return;
    const t = setInterval(refresh, 700);
    return () => clearInterval(t);
  }, [st?.state]);
  if (!st) return <Loading />;
  return (
    <div className="setup-panel">
      {st.state === "unavailable" && <p className="muted">{st.reason}</p>}
      {st.state === "idle" && (
        <>
          <p className="muted">Respeek can track per-app network usage after a one-time setup that grants the required capability. You’ll be asked for your password.</p>
          <pre className="code selectable">{st.command}</pre>
          <div className="row-gap">
            <Button variant="filled" onClick={() => api.setupScriptRun(false).then(refresh)}>Run setup</Button>
            <Button variant="plain" onClick={() => api.setupScriptOpen()}>View script</Button>
            <Button variant="plain" onClick={() => api.setupScriptRun(true).then(refresh)}>Revert</Button>
          </div>
        </>
      )}
      {st.state === "running" && <Loading />}
      {st.state === "finished" && (
        <p className={st.ok ? "ok-text" : "danger-text"}>{st.ok ? "Setup complete." : "Setup failed."} {st.message}</p>
      )}
    </div>
  );
}

function FirstRun() {
  const close = useUi((s) => s.closeSheet);
  const done = () => {
    api.finishFirstRun();
    close();
  };
  const features: [ReactNode, string, string][] = [
    [<Cpu size={18} />, "Live performance", "CPU, memory, disks, network, GPUs, fans and batteries — updated smoothly."],
    [<Sparkles size={18} />, "Apps & processes", "See what’s using your computer and quit anything misbehaving."],
    [<Server size={18} />, "Services", "Start, stop and inspect systemd services and their logs."],
  ];
  return (
    <Sheet width={520} dismissable={false} title="Welcome to Respeek" subtitle="A clear view of everything your computer is doing."
      icon={<img src={logo} width={52} height={52} alt="" />}
      footer={<><span className="grow" /><Button variant="filled" onClick={done}>Continue</Button></>}>
      <ul className="features">
        {features.map(([icon, t, d], i) => (
          <motion.li key={t} initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ type: "spring", bounce: 0, duration: 0.45, delay: 0.15 + i * 0.07 }}>
            <span className="feat-icon">{icon}</span>
            <div><strong>{t}</strong><p className="muted">{d}</p></div>
          </motion.li>
        ))}
      </ul>
      <section className="inset-list">
        <h3 className="list-title">Per-app network usage (optional)</h3>
        <SetupScriptPanel />
      </section>
    </Sheet>
  );
}

function FatalSheet({ message }: { message: string }) {
  return (
    <Sheet width={440} dismissable={false} title="A fatal error has occurred" subtitle={message}
      icon={<span className="sheet-glyph tint-red"><AlertTriangle size={20} /></span>}
      footer={<><span className="grow" /><Button variant="destructive" onClick={() => api.quit()}>Quit</Button></>}>
      {null}
    </Sheet>
  );
}

function render(s: SheetState) {
  switch (s.kind) {
    case "process": return <ProcessSheet pid={s.pid} origin={s.origin} />;
    case "service": return <ServiceSheet id={s.id} origin={s.origin} />;
    case "smart": return <SmartSheet device={s.device} />;
    case "eject": return <EjectSheet device={s.device} />;
    case "confirm-quit": return <ConfirmQuit title={s.title} pids={s.pids} isApp={s.isApp} />;
    case "preferences": return <Preferences tab={s.tab} />;
    case "about-system": return <AboutSystemSheet />;
    case "about": return <AboutSheet />;
    case "shortcuts": return <ShortcutsSheet />;
    case "first-run": return <FirstRun />;
    case "fatal": return <FatalSheet message={s.message} />;
  }
}

export function SheetHost() {
  const sheet = useUi((s) => s.sheet);
  return (
    <AnimatePresence>
      {sheet && (
        <div className="sheet-layer" key={sheet.kind + ("pid" in sheet ? sheet.pid : "") + ("id" in sheet ? sheet.id : "")}>
          {render(sheet)}
        </div>
      )}
    </AnimatePresence>
  );
}
