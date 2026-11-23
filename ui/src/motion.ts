// Spring presets (apple-design: damping 1.0 by default, a little bounce only after momentum).
import type { Transition } from "motion/react";

/** Default UI spring — critically damped, no overshoot. */
export const spring: Transition = { type: "spring", bounce: 0, duration: 0.38 };
/** Snappy controls (pills, thumbs, chevrons). */
export const snappy: Transition = { type: "spring", bounce: 0, duration: 0.26 };
/** Sheets and drawers. */
export const sheetSpring: Transition = { type: "spring", bounce: 0.08, duration: 0.42 };
/** After a drag/flick release — momentum earns a touch of bounce. */
export const momentum: Transition = { type: "spring", bounce: 0.2, duration: 0.4 };
/** Reduced-motion cross-fade. */
export const fade: Transition = { duration: 0.18, ease: "easeOut" };

export const prefersReducedMotion = () =>
  typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
