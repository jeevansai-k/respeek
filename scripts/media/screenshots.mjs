// Capture marketing screenshots of the real UI (fixture backend) at 2x, transparent corners.
import { openTab, sleep } from "./cdp.mjs";
const out = process.argv[2] ?? "site/assets/shots";
const W = 1440, H = 900;
const tab = await openTab();
await tab.send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });

const click = (text, sel = ".sb-item") =>
  tab.eval(`(() => { const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find(e => e.textContent.trim().startsWith(${JSON.stringify(text)})); el?.click(); return !!el; })()`);
const settle = () => sleep(2600);

async function scene(name, dark, setup) {
  await tab.viewport(W, H, 2, dark);
  await tab.goto("http://localhost:1420/");
  await sleep(2500);
  await setup();
  await settle();
  await tab.shot(`${out}/${name}.png`);
  console.log("captured", name);
}

await scene("overview-dark", true, () => click("Overview"));
await scene("overview-light", false, () => click("Overview"));
await scene("cpu-dark", true, async () => {
  await click("CPU");
  await sleep(600);
  await tab.eval(`document.querySelector('.segmented [title="Per core"]')?.click()`);
});
await scene("memory-light", false, () => click("Memory"));
await scene("gpu-dark", true, () => click("GPU 0"));
await scene("network-light", false, () => click("Wi-Fi"));
await scene("apps-dark", true, async () => {
  await click("Apps");
  await sleep(900);
  await tab.eval(`document.querySelector('.t-row.kind-app .chev-btn')?.click()`);
  await sleep(500);
  await tab.eval(`(() => { const r = document.querySelector('.t-row.kind-app'); r?.dispatchEvent(new PointerEvent('pointerdown', {bubbles: true})); })()`);
});
await scene("services-light", false, async () => {
  await click("Services");
  await sleep(900);
  await tab.eval(`(() => { const r = document.querySelectorAll('.t-row')[3]; r?.dispatchEvent(new PointerEvent('pointerdown', {bubbles: true})); })()`);
});
await scene("preferences-dark", true, async () => {
  await click("Memory");
  await sleep(500);
  await tab.eval(`document.querySelector('[aria-label="More"]')?.click()`);
  await sleep(400);
  await click("Preferences", ".menu-item");
});
await tab.close();
