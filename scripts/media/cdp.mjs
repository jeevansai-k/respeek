// Minimal Chrome DevTools Protocol client (Node 22 built-in WebSocket, no dependencies).
export async function openTab(port = 9333) {
  const res = await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" });
  const t = await res.json();
  const ws = new WebSocket(t.webSocketDebuggerUrl);
  await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
  let id = 0;
  const pending = new Map();
  const listeners = new Map();
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
    } else if (msg.method && listeners.has(msg.method)) {
      for (const cb of listeners.get(msg.method)) cb(msg.params);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const i = ++id;
      pending.set(i, { resolve, reject });
      ws.send(JSON.stringify({ id: i, method, params }));
    });
  const once = (method) => new Promise((r) => {
    const cb = (p) => { listeners.get(method).delete(cb); r(p); };
    if (!listeners.has(method)) listeners.set(method, new Set());
    listeners.get(method).add(cb);
  });
  await send("Page.enable");
  await send("Runtime.enable");
  const api = {
    send,
    async viewport(width, height, dpr = 2, dark = false) {
      await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: dpr, mobile: false });
      await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: dark ? "dark" : "light" }] });
    },
    async goto(url) {
      const loaded = once("Page.loadEventFired");
      await send("Page.navigate", { url });
      await loaded;
    },
    async eval(expr) {
      const r = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? "eval failed");
      return r.result.value;
    },
    async shot(path, opts = {}) {
      const r = await send("Page.captureScreenshot", { format: opts.format ?? "png", quality: opts.quality, captureBeyondViewport: false });
      const fs = await import("node:fs");
      fs.writeFileSync(path, Buffer.from(r.data, "base64"));
    },
    async close() {
      ws.close();
      await fetch(`http://127.0.0.1:${port}/json/close/${t.id}`).catch(() => {});
    },
  };
  return api;
}
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
