import { create } from "zustand";
import type { ScanProgress, ScanReport } from "../api/types";

interface LibraryState {
  /// Wordt verhoogd als de bibliotheek verandert, zodat pagina's opnieuw laden.
  version: number;
  scanning: boolean;
  progress: ScanProgress | null;
  lastReports: { folder: string; report: ScanReport }[];
  bump: () => void;
  setScanning: (v: boolean) => void;
  setProgress: (p: ScanProgress | null) => void;
  setReports: (r: { folder: string; report: ScanReport }[]) => void;
}

export const useLibrary = create<LibraryState>((set, get) => ({
  version: 0,
  scanning: false,
  progress: null,
  lastReports: [],
  bump: () => set({ version: get().version + 1 }),
  setScanning: (scanning) => set({ scanning }),
  setProgress: (progress) => set({ progress }),
  setReports: (lastReports) => set({ lastReports }),
}));
