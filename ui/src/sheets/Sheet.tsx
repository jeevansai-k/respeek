import { motion } from "motion/react";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { fade, prefersReducedMotion, sheetSpring } from "../motion";
import { useUi } from "../store";

/**
 * Modal sheet: dims the window (scrim) and materializes — blur, scale and opacity animate
 * together. When opened from a row, it grows out of that row (transform-origin = row).
 */
export function Sheet({
  title, subtitle, icon, children, width = 560, origin, footer, onClose, dismissable = true,
}: {
  title: ReactNode; subtitle?: ReactNode; icon?: ReactNode; children: ReactNode; width?: number;
  origin?: DOMRect; footer?: ReactNode; onClose?: () => void; dismissable?: boolean;
}) {
  const close = useUi((s) => s.closeSheet);
  const doClose = () => (onClose ? onClose() : close());
  const reduced = prefersReducedMotion();

  const vw = typeof window !== "undefined" ? window.innerWidth : 1200;
  const vh = typeof window !== "undefined" ? window.innerHeight : 800;
  const originStyle = origin
    ? `${origin.left + origin.width / 2 - (vw - width) / 2}px ${origin.top + origin.height / 2 - vh * 0.12}px`
    : "50% 0%";

  return (
    <>
      <motion.div
        className="scrim"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={fade}
        onClick={() => dismissable && doClose()}
      />
      <motion.div
        className="sheet"
        role="dialog"
        aria-modal="true"
        style={{ width, transformOrigin: originStyle }}
        initial={reduced ? { opacity: 0 } : { opacity: 0, scale: origin ? 0.6 : 0.94, y: origin ? 0 : -16, filter: "blur(10px)" }}
        animate={{ opacity: 1, scale: 1, y: 0, filter: "blur(0px)" }}
        exit={reduced ? { opacity: 0 } : { opacity: 0, scale: origin ? 0.7 : 0.96, y: origin ? 0 : -10, filter: "blur(8px)" }}
        transition={reduced ? fade : sheetSpring}
        onKeyDown={(e) => {
          if (e.key === "Escape" && dismissable) doClose();
        }}
      >
        <header className="sheet-head">
          {icon && <div className="sheet-icon">{icon}</div>}
          <div className="sheet-titles">
            <h2>{title}</h2>
            {subtitle && <p>{subtitle}</p>}
          </div>
          {dismissable && (
            <button className="sheet-close" onClick={doClose} aria-label="Close">
              <X size={14} strokeWidth={2.4} />
            </button>
          )}
        </header>
        <div className="sheet-body">{children}</div>
        {footer && <footer className="sheet-foot">{footer}</footer>}
      </motion.div>
    </>
  );
}

export function Loading() {
  return (
    <div className="loading">
      <span className="spinner" />
    </div>
  );
}
