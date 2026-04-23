import { create } from 'zustand';
import type { ScanCompletePayload } from '../types';

interface ScanScheduleState {
  lastScanTime: number | null;
  lastScanResult: ScanCompletePayload | null;
  nextScanTime: number | null;
  setLastScanTime: (time: number | null) => void;
  setLastScanResult: (result: ScanCompletePayload | null) => void;
  setNextScanTime: (time: number | null) => void;
}

export const useScanScheduleStore = create<ScanScheduleState>((set) => ({
  lastScanTime: null,
  lastScanResult: null,
  nextScanTime: null,
  setLastScanTime: (lastScanTime) => set({ lastScanTime }),
  setLastScanResult: (lastScanResult) => set({ lastScanResult }),
  setNextScanTime: (nextScanTime) => set({ nextScanTime }),
}));
