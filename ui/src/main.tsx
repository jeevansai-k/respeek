import { Profiler, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MotionConfig } from "motion/react";
import "@fontsource-variable/inter";
import "./styles/tokens.css";
import "./styles/app.css";
import App from "./App";

const tour = import.meta.env.VITE_PERF_TOUR === "1";

async function boot() {
  const app = (
    /* reducedMotion="user": springs become instant/cross-fades when the OS asks for less motion. */
    <MotionConfig reducedMotion="user">
      <App />
    </MotionConfig>
  );
  let tree = app;
  if (tour) {
    const { onRender } = await import("./dev/perfTour");
    tree = <Profiler id="app" onRender={onRender}>{app}</Profiler>;
  }
  createRoot(document.getElementById("root")!).render(tour ? tree : <StrictMode>{tree}</StrictMode>);
}
boot();

if (tour) {
  import("./dev/perfTour").then((m) => m.runPerfTour());
}
