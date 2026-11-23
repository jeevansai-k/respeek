// Respeek website interactions. No dependencies.
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

/* ---------- Nav: glass appears only once content scrolls underneath ---------- */
const nav = document.getElementById("nav");
const onScroll = () => nav.classList.toggle("scrolled", scrollY > 8);
addEventListener("scroll", onScroll, { passive: true });
onScroll();

/* ---------- Reveal on scroll ---------- */
const io = new IntersectionObserver(
  (entries) => {
    for (const e of entries) {
      if (e.isIntersecting) {
        e.target.classList.add("in");
        io.unobserve(e.target);
      }
    }
  },
  { rootMargin: "0px 0px -8% 0px", threshold: 0.12 },
);
document.querySelectorAll(".reveal").forEach((el) => io.observe(el));

/* ---------- Hero window: tilt flattens as it scrolls into view (transform only) ---------- */
const win = document.getElementById("hero-window");
if (win && !reduced) {
  let ticking = false;
  const update = () => {
    ticking = false;
    const r = win.getBoundingClientRect();
    const p = Math.min(1, Math.max(0, 1 - (r.top - innerHeight * 0.25) / (innerHeight * 0.75)));
    const e = 1 - Math.pow(1 - p, 3);
    win.style.setProperty("--tilt", `${(1 - e) * 18}deg`);
    win.style.setProperty("--scale", String(0.92 + e * 0.08));
  };
  addEventListener("scroll", () => {
    if (!ticking) {
      ticking = true;
      requestAnimationFrame(update);
    }
  }, { passive: true });
  update();
}

/* ---------- Spring (critically damped by default), velocity-aware & interruptible ---------- */
function springTo(get, set, target, { velocity = 0, response = 0.4, damping = 1 } = {}) {
  // Apple's parameters → physics: ω = 2π / response, c = 2ζω (unit mass).
  const w = (2 * Math.PI) / response;
  let x = get();
  let v = velocity;
  let last = performance.now();
  let raf = 0;
  const step = (now) => {
    const dt = Math.min(0.032, (now - last) / 1000);
    last = now;
    const a = -w * w * (x - target) - 2 * damping * w * v;
    v += a * dt;
    x += v * dt;
    set(x);
    if (Math.abs(v) > 0.01 || Math.abs(x - target) > 0.01) raf = requestAnimationFrame(step);
    else set(target);
  };
  raf = requestAnimationFrame(step);
  return () => cancelAnimationFrame(raf);
}

const rubberband = (overshoot, dim, c = 0.55) => (overshoot * dim * c) / (dim + c * Math.abs(overshoot));
const project = (v, d = 0.998) => ((v / 1000) * d) / (1 - d);

/* ---------- Light/dark compare: 1:1 drag, momentum projection, rubber-band edges ---------- */
const cmp = document.getElementById("compare");
if (cmp) {
  const handle = cmp.querySelector(".compare-handle");
  let pos = 50; // percent
  let stop = null;
  const render = (p) => {
    pos = p;
    cmp.style.setProperty("--pos", `${p}%`);
    handle.setAttribute("aria-valuenow", String(Math.round(Math.min(100, Math.max(0, p)))));
  };
  const toPct = (clientX) => {
    const r = cmp.getBoundingClientRect();
    return ((clientX - r.left) / r.width) * 100;
  };

  let dragging = false;
  let grabOffset = 0;
  let history = [];
  cmp.addEventListener("pointerdown", (e) => {
    stop?.(); // interrupt any settling spring; continue from the on-screen value
    dragging = true;
    cmp.setPointerCapture(e.pointerId);
    cmp.classList.add("dragging");
    const p = toPct(e.clientX);
    // Grabbing the handle keeps its offset; tapping elsewhere jumps there (instant response).
    grabOffset = Math.abs(p - pos) < 4 ? pos - p : 0;
    if (!grabOffset) render(p);
    history = [{ t: performance.now(), p: pos }];
  });
  cmp.addEventListener("pointermove", (e) => {
    if (!dragging) return;
    let p = toPct(e.clientX) + grabOffset;
    if (p < 0) p = -rubberband(-p, 100);
    if (p > 100) p = 100 + rubberband(p - 100, 100);
    render(p);
    history.push({ t: performance.now(), p });
    if (history.length > 6) history.shift();
  });
  const release = () => {
    if (!dragging) return;
    dragging = false;
    cmp.classList.remove("dragging");
    const a = history[0];
    const b = history[history.length - 1];
    const dt = Math.max(1, b.t - a.t);
    const vPctPerSec = ((b.p - a.p) / dt) * 1000;
    const target = Math.min(100, Math.max(0, pos + project(vPctPerSec, 0.99)));
    stop = springTo(() => pos, render, target, { velocity: vPctPerSec, response: 0.45, damping: Math.abs(vPctPerSec) > 40 ? 0.85 : 1 });
  };
  cmp.addEventListener("pointerup", release);
  cmp.addEventListener("pointercancel", release);

  handle.addEventListener("keydown", (e) => {
    const step = e.shiftKey ? 10 : 2;
    let t = null;
    if (e.key === "ArrowLeft") t = pos - step;
    if (e.key === "ArrowRight") t = pos + step;
    if (e.key === "Home") t = 0;
    if (e.key === "End") t = 100;
    if (t !== null) {
      e.preventDefault();
      stop?.();
      t = Math.min(100, Math.max(0, t));
      stop = reduced ? (render(t), null) : springTo(() => pos, render, t, { response: 0.3 });
    }
  });
}

/* ---------- Copy buttons ---------- */
document.querySelectorAll(".copy").forEach((btn) => {
  btn.addEventListener("click", async () => {
    const text = document.querySelector(btn.dataset.copy)?.innerText ?? "";
    try {
      await navigator.clipboard.writeText(text);
      btn.textContent = "Copied";
    } catch {
      btn.textContent = "Select & copy";
    }
    setTimeout(() => (btn.textContent = "Copy"), 1600);
  });
});
