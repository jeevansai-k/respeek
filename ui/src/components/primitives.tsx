import { motion, type HTMLMotionProps } from "motion/react";
import {
  AppWindow, BatteryMedium, Box, Cpu, EthernetPort, Fan, Gpu, HardDrive, Layers, MemoryStick,
  Server, Terminal, Wifi, type LucideIcon,
} from "lucide-react";
import { forwardRef, useId, type ReactNode } from "react";
import { snappy, spring } from "../motion";
import { assetUrl, type DeviceKind, type Tint } from "../api";

// ----- Ticker: per-digit rolling numbers --------------------------------------------------------

function Digit({ d }: { d: number }) {
  return (
    <span className="tk-digit" aria-hidden>
      <span className="tk-strip" style={{ transform: `translateY(${-d * 10}%)` }}>
        {"0123456789".split("").map((c) => (
          <span key={c}>{c}</span>
        ))}
      </span>
    </span>
  );
}

/** Formatted value from the core with digits that roll on change (presentation only). */
export function Ticker({ value, className }: { value: string; className?: string }) {
  const chars = value.split("");
  return (
    <span className={`ticker ${className ?? ""}`} aria-label={value} role="text">
      {chars.map((c, i) => {
        const pos = chars.length - i;
        return /\d/.test(c) ? (
          <Digit key={`d${pos}`} d={Number(c)} />
        ) : (
          <span key={`s${pos}${c}`} className="tk-sym" aria-hidden>
            {c}
          </span>
        );
      })}
    </span>
  );
}

// ----- Device / row glyphs ------------------------------------------------------------------

export const deviceIcon: Record<DeviceKind, LucideIcon> = {
  cpu: Cpu,
  memory: MemoryStick,
  disk: HardDrive,
  network: Wifi,
  gpu: Gpu,
  fan: Fan,
  battery: BatteryMedium,
};

export function DeviceGlyph({ kind, title, tint, size = 15 }: { kind: DeviceKind; title?: string; tint: Tint; size?: number }) {
  const Icon = kind === "network" && title && title !== "Wi-Fi" ? EthernetPort : deviceIcon[kind];
  return (
    <span className="device-glyph" style={{ "--glyph": `var(--tint-${tint})` } as React.CSSProperties}>
      <Icon size={size} strokeWidth={1.9} />
    </span>
  );
}

const rowGlyphs: Record<string, LucideIcon> = { app: AppWindow, process: Terminal, service: Server, section: Layers };

export function RowIcon({ icon, glyph, size = 18 }: { icon: string | null; glyph: string; size?: number }) {
  if (icon) {
    return <img className="row-icon" src={assetUrl(icon) ?? undefined} width={size} height={size} alt="" draggable={false} />;
  }
  const G = rowGlyphs[glyph] ?? Box;
  return (
    <span className={`row-glyph g-${glyph}`} style={{ width: size, height: size }}>
      <G size={size * 0.68} strokeWidth={1.8} />
    </span>
  );
}

// ----- Buttons ---------------------------------------------------------------------------------

type BtnProps = HTMLMotionProps<"button"> & {
  variant?: "plain" | "tinted" | "filled" | "destructive" | "glass";
  size?: "sm" | "md" | "lg";
  icon?: ReactNode;
};

export const Button = forwardRef<HTMLButtonElement, BtnProps>(function Button(
  { variant = "plain", size = "md", icon, children, className, ...rest },
  ref,
) {
  return (
    <motion.button
      ref={ref}
      className={`btn btn-${variant} btn-${size} ${className ?? ""}`}
      whileTap={{ scale: 0.96 }}
      transition={snappy}
      {...rest}
    >
      {icon}
      {children as ReactNode}
    </motion.button>
  );
});

export function IconButton({ label, children, active, ...rest }: HTMLMotionProps<"button"> & { label: string; active?: boolean }) {
  return (
    <motion.button
      className={`icon-btn ${active ? "is-active" : ""}`}
      aria-label={label}
      title={label}
      whileTap={{ scale: 0.9 }}
      transition={snappy}
      {...rest}
    >
      {children}
    </motion.button>
  );
}

// ----- Switch ----------------------------------------------------------------------------------

export function Switch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`switch ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <motion.span className="switch-thumb" layout transition={snappy} whileTap={{ width: 26 }} />
    </button>
  );
}

// ----- Segmented control -------------------------------------------------------------------

export function Segmented<T extends string | number>({
  value, options, onChange, size = "md",
}: { value: T; options: { value: T; label: ReactNode; title?: string }[]; onChange: (v: T) => void; size?: "sm" | "md" }) {
  const id = useId();
  return (
    <div className={`segmented seg-${size}`} role="tablist">
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={String(o.value)}
            role="tab"
            aria-selected={active}
            title={o.title}
            className={`seg-item ${active ? "active" : ""}`}
            onClick={() => onChange(o.value)}
          >
            {active && <motion.span className="seg-thumb" layoutId={`seg-${id}`} transition={spring} />}
            <span className="seg-label">{o.label}</span>
          </button>
        );
      })}
    </div>
  );
}

// ----- Stat ------------------------------------------------------------------------------------

export function StatView({ label, value, primary, tint }: { label: string; value: string; primary: boolean; tint: Tint | null }) {
  return (
    <div className={`stat ${primary ? "stat-primary" : ""}`}>
      <span className="stat-label">
        {tint && <i className="stat-dot" style={{ background: `var(--tint-${tint})` }} />}
        {label}
      </span>
      {primary ? <Ticker className="stat-value" value={value} /> : <span className="stat-value" title={value}>{value}</span>}
    </div>
  );
}
