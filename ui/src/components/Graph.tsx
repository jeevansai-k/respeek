import { useEffect, useRef } from "react";
import type { Series } from "../api";
import { prefersReducedMotion } from "../motion";
import { useUi } from "../store";

interface Props {
  series: Series[];
  /** Slide continuously between samples (settings.sliding_graphs). */
  sliding?: boolean;
  /** Smooth curves instead of straight segments (settings.smooth_graphs). */
  smooth?: boolean;
  grid?: boolean;
  /** Thin sparkline mode: no grid, no glow, thinner stroke. */
  spark?: boolean;
  className?: string;
}

function hexA(color: string, a: number) {
  const c = color.trim();
  if (c.startsWith("#") && (c.length === 7 || c.length === 4)) {
    const full = c.length === 4 ? `#${c[1]}${c[1]}${c[2]}${c[2]}${c[3]}${c[3]}` : c;
    const n = parseInt(full.slice(1), 16);
    return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
  }
  return c;
}

/**
 * Canvas time-series renderer. Values arrive already normalized (0..1) from the core.
 *
 * Sliding is done on the compositor: each sample is drawn once onto a canvas one step wider
 * than the viewport (newest point just off the right edge), then the canvas is translated
 * left by one step over the sample interval with a GPU transform. When the next sample lands
 * the redraw matches the final frame exactly, so motion is seamless at near-zero CPU cost.
 */
export function Graph({ series, sliding = true, smooth = true, grid = true, spark = false, className }: Props) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const drawnOnce = useRef(false);
  const seriesRef = useRef(series);
  seriesRef.current = series;
  const interval = useUi((s) => s.snap?.interval_ms ?? 1000);
  const seq = useUi((s) => s.snap?.seq ?? 0);

  useEffect(() => {
    const wrap = wrapRef.current;
    const canvas = canvasRef.current;
    if (!wrap || !canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const cs = getComputedStyle(canvas);
    const slide = sliding && !spark && !prefersReducedMotion();
    let anim: Animation | null = null;

    const draw = (animate: boolean) => {
      const list = seriesRef.current;
      const n = list[0]?.values.length ?? 0;
      const W = wrap.clientWidth;
      const H = wrap.clientHeight;
      if (W === 0 || H === 0 || n < 3) return;
      const step = slide ? W / (n - 2) : W / (n - 1);
      const CW = slide ? W + step : W;
      const dpr = window.devicePixelRatio || 1;
      canvas.style.width = `${CW}px`;
      canvas.style.height = `${H}px`;
      if (canvas.width !== Math.round(CW * dpr) || canvas.height !== Math.round(H * dpr)) {
        canvas.width = Math.round(CW * dpr);
        canvas.height = Math.round(H * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, CW, H);

      const pad = spark ? 1.5 : 2;
      const h = H - pad * 2;
      // Sliding: newest point sits at CW (one step past the visible edge) before the slide.
      const xAt = (i: number) => (slide ? CW - (n - 1 - i) * step : i * step);
      const yAt = (v: number) => pad + h - Math.max(0, Math.min(1, v)) * h;

      if (grid && !spark) {
        ctx.strokeStyle = cs.getPropertyValue("--separator");
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (const f of [0.25, 0.5, 0.75]) {
          const y = Math.round(pad + h * f) + 0.5;
          ctx.moveTo(0, y);
          ctx.lineTo(CW, y);
        }
        // Vertical grid travels with the data (every 10 samples), like a strip chart.
        const offset = seq % 10;
        for (let i = n - 1 - offset; i >= 0; i -= 10) {
          const x = Math.round(xAt(i)) + 0.5;
          ctx.moveTo(x, pad);
          ctx.lineTo(x, H - pad);
        }
        ctx.stroke();
      }

      const trace = (values: number[]) => {
        ctx.beginPath();
        ctx.moveTo(xAt(0), yAt(values[0]));
        for (let i = 1; i < n; i++) {
          const x = xAt(i);
          const y = yAt(values[i]);
          if (smooth) {
            const px = xAt(i - 1);
            const py = yAt(values[i - 1]);
            const mx = (px + x) / 2;
            ctx.bezierCurveTo(mx, py, mx, y, x, y);
          } else {
            ctx.lineTo(x, y);
          }
        }
      };

      // Solid series on top of dashed comparisons.
      const ordered = [...list].sort((a, b) => Number(b.dashed) - Number(a.dashed));
      for (const s of ordered) {
        const color = cs.getPropertyValue(`--tint-${s.tint}`) || "#007aff";
        if (!s.dashed) {
          trace(s.values);
          ctx.lineTo(xAt(n - 1), H);
          ctx.lineTo(xAt(0), H);
          ctx.closePath();
          const g = ctx.createLinearGradient(0, 0, 0, H);
          g.addColorStop(0, hexA(color, spark ? 0.32 : 0.3));
          g.addColorStop(1, hexA(color, 0.02));
          ctx.fillStyle = g;
          ctx.fill();
        }
        trace(s.values);
        ctx.setLineDash(s.dashed ? [4, 3] : []);
        ctx.lineWidth = spark ? 1.25 : 1.75;
        ctx.lineJoin = "round";
        ctx.lineCap = "round";
        ctx.strokeStyle = color;
        ctx.stroke();
        ctx.setLineDash([]);

        if (!spark && !s.dashed) {
          // Leading-edge dot marks "now"; it rides the slide in from the right edge.
          const x = xAt(n - 1) - 3;
          const y = yAt(s.values[n - 1]);
          // Halo + dot. (shadowBlur would be a CPU Gaussian blur on WebKitGTK's 2D canvas.)
          ctx.beginPath();
          ctx.arc(x, y, 7, 0, Math.PI * 2);
          ctx.fillStyle = hexA(color, 0.18);
          ctx.fill();
          ctx.beginPath();
          ctx.arc(x, y, 3, 0, Math.PI * 2);
          ctx.fillStyle = color;
          ctx.fill();
        }
      }

      anim?.cancel();
      anim = null;
      if (slide && animate) {
        anim = canvas.animate([{ transform: "translateX(0)" }, { transform: `translateX(${-step}px)` }], {
          duration: interval,
          easing: "linear",
          fill: "forwards",
        });
      } else if (slide) {
        canvas.style.transform = `translateX(${-step}px)`;
      } else {
        canvas.style.transform = "";
      }
    };

    // Off-screen graphs (scrolled away, long sidebars) skip their redraw entirely.
    let visible = true;
    let stale = false;
    const io = new IntersectionObserver(([e]) => {
      visible = e.isIntersecting;
      if (visible && stale) {
        stale = false;
        draw(false);
      }
    });
    io.observe(wrap);
    const r = wrap.getBoundingClientRect();
    const onScreen = r.bottom > 0 && r.top < window.innerHeight && r.width > 0;
    // First draw of a freshly mounted graph waits two frames so a page's slide-in starts
    // painting immediately; later ticks draw synchronously.
    let firstRaf = 0;
    if (!onScreen) stale = true;
    else if (drawnOnce.current) draw(true);
    else {
      firstRaf = requestAnimationFrame(() => {
        firstRaf = requestAnimationFrame(() => {
          drawnOnce.current = true;
          draw(true);
        });
      });
    }

    let resizeRaf = 0;
    const ro = new ResizeObserver(() => {
      if (!visible) {
        stale = true;
        return;
      }
      cancelAnimationFrame(resizeRaf);
      resizeRaf = requestAnimationFrame(() => draw(false));
    });
    ro.observe(wrap);
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const redraw = () => requestAnimationFrame(() => draw(false));
    mq.addEventListener("change", redraw);
    return () => {
      anim?.cancel();
      cancelAnimationFrame(firstRaf);
      io.disconnect();
      cancelAnimationFrame(resizeRaf);
      ro.disconnect();
      mq.removeEventListener("change", redraw);
    };
  }, [series, seq, interval, sliding, smooth, grid, spark]);

  return (
    <div ref={wrapRef} className={className} style={{ width: "100%", height: "100%", overflow: "hidden", position: "relative" }}>
      <canvas ref={canvasRef} style={{ display: "block", position: "absolute", left: 0, top: 0, willChange: sliding && !spark ? "transform" : undefined }} />
    </div>
  );
}
