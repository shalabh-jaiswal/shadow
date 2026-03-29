import { create } from 'zustand';
import type { DaemonStats } from '../types';
import { ipc } from '../ipc';

const DEFAULT_STATS: DaemonStats = {
  files_uploaded: 0,
  bytes_uploaded: 0,
  active_uploads: 0,
  queue_depth: 0,
};

interface StatsState {
  stats: DaemonStats;
  fetch: () => Promise<void>;
}

export const useStatsStore = create<StatsState>((set) => ({
  stats: DEFAULT_STATS,

  fetch: async () => {
    try {
      const stats = await ipc.getStats();
      set({ stats });
    } catch {
      // Silently ignore — UI keeps showing last known values
    }
  },
}));
