import { create } from 'zustand';
import type { ActivityEntry, ActivityStatus } from '../types';

const MAX_ENTRIES = 200;

export type FilterStatus = 'all' | 'uploaded' | 'skipped' | 'error';

interface ActivityState {
  entries: ActivityEntry[];
  filter: FilterStatus;
  addEntry: (entry: ActivityEntry) => void;
  setFilter: (filter: FilterStatus) => void;
  clear: () => void;
}

export const useActivityStore = create<ActivityState>((set) => ({
  entries: [],
  filter: 'all',

  addEntry: (entry) =>
    set((state) => ({
      // Prepend newest; evict oldest beyond the cap
      entries: [entry, ...state.entries].slice(0, MAX_ENTRIES),
    })),

  setFilter: (filter) => set({ filter }),

  clear: () => set({ entries: [] }),
}));

// ── Selectors ─────────────────────────────────────────────────────────────────

/** Return entries matching the active filter. */
export function selectFiltered(state: ActivityState): ActivityEntry[] {
  const { entries, filter } = state;
  if (filter === 'all') return entries;
  if (filter === 'error') {
    return entries.filter(
      (e) => e.status === 'error' || e.status === 'failed',
    );
  }
  return entries.filter((e) => e.status === (filter as ActivityStatus));
}
