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

  addEntry: (newEntry) =>
    set((state) => {
      const AGGREGATION_WINDOW_MS = 5 * 60 * 1000; // 5 minutes
      const now = Date.now();

      // Look for a recent entry with the same path
      // We check from the beginning (newest)
      const existingIndex = state.entries.findIndex(
        (e) =>
          e.path === newEntry.path &&
          now - e.timestamp < AGGREGATION_WINDOW_MS &&
          // Don't merge renames with non-renames to avoid confusion
          (e.status === 'renamed' || e.status === 'rename_error') ===
            (newEntry.status === 'renamed' || newEntry.status === 'rename_error'),
      );

      if (existingIndex !== -1) {
        const entries = [...state.entries];
        const existing = entries[existingIndex];

        // Merge providers
        const providers = { ...existing.providers, ...newEntry.providers };

        // Determine aggregated status
        const providerStatuses = Object.values(providers).map((p) => p.status);
        
        let status = existing.status;
        
        // Priority for row status:
        // 1. Final failures/errors (if any provider failed, the whole row is marked)
        // 2. Active operations (if any provider is still uploading)
        // 3. Successes (if all providers are done)
        // 4. Queued/Skipped (initial state)
        if (providerStatuses.some((s) => s === 'failed' || s === 'rename_error')) {
          status = providerStatuses.find((s) => s === 'failed' || s === 'rename_error') as ActivityStatus;
        } else if (providerStatuses.some((s) => s === 'error')) {
          status = 'error';
        } else if (providerStatuses.some((s) => s === 'uploading')) {
          status = 'uploading';
        } else if (providerStatuses.some((s) => s === 'renamed')) {
          status = 'renamed';
        } else if (providerStatuses.length > 0 && providerStatuses.every((s) => s === 'uploaded' || s === 'skipped')) {
          status = providerStatuses.some((s) => s === 'uploaded') ? 'uploaded' : 'skipped';
        } else {
          // If no provider statuses yet or we are just starting, use the newest status
          // BUT only if the existing status isn't already "better" (e.g. don't go from uploading -> queued)
          const priority: Record<ActivityStatus, number> = {
            'failed': 5,
            'rename_error': 5,
            'error': 4,
            'uploading': 3,
            'uploaded': 2,
            'renamed': 2,
            'skipped': 1,
            'queued': 0,
          };
          if (priority[newEntry.status] > priority[existing.status]) {
            status = newEntry.status;
          }
        }

        entries[existingIndex] = {
          ...existing,
          ...newEntry,
          id: existing.id, // Preserve ID
          providers,
          status,
          error: newEntry.error || existing.error,
          timestamp: Math.max(existing.timestamp, newEntry.timestamp),
        };

        return { entries };
      }

      // Prepend newest; evict oldest beyond the cap
      return {
        entries: [newEntry, ...state.entries].slice(0, MAX_ENTRIES),
      };
    }),

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
