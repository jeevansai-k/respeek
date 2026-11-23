import { create } from "zustand";
import type { Snapshot } from "./api";

export type SheetState =
  | { kind: "process"; pid: number; origin?: DOMRect }
  | { kind: "service"; id: number; origin?: DOMRect }
  | { kind: "smart"; device: string }
  | { kind: "eject"; device: string }
  | { kind: "confirm-quit"; title: string; pids: number[]; isApp: boolean }
  | { kind: "preferences"; tab?: string }
  | { kind: "about-system" }
  | { kind: "about" }
  | { kind: "shortcuts" }
  | { kind: "first-run" }
  | { kind: "fatal"; message: string };

interface UiStore {
  snap: Snapshot | null;
  /** Time the latest snapshot arrived (graphs slide relative to it). */
  receivedAt: number;
  sheet: SheetState | null;
  sidebarCollapsed: boolean;
  /** Row keys expanded in the last ~400 ms — their children animate in. */
  justExpanded: { key: string; at: number } | null;
  toast: string | null;
  setSnap: (s: Snapshot) => void;
  openSheet: (s: SheetState) => void;
  closeSheet: () => void;
  toggleSidebar: () => void;
  markExpanded: (key: string) => void;
  showToast: (t: string) => void;
}

export const useUi = create<UiStore>((set) => ({
  snap: null,
  receivedAt: 0,
  sheet: null,
  sidebarCollapsed: false,
  justExpanded: null,
  toast: null,
  setSnap: (snap) => set({ snap, receivedAt: performance.now() }),
  openSheet: (sheet) => set({ sheet }),
  closeSheet: () => set({ sheet: null }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  markExpanded: (key) => set({ justExpanded: { key, at: performance.now() } }),
  showToast: (toast) => {
    set({ toast });
    setTimeout(() => set((s) => (s.toast === toast ? { toast: null } : s)), 1800);
  },
}));

export const useSnap = () => useUi((s) => s.snap);
