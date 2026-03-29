import { create } from 'zustand';
const MAX_ENTRIES = 200;
export const useActivityStore = create((set) => ({
    entries: [],
    filter: 'all',
    addEntry: (entry) => set((state) => ({
        // Prepend newest; evict oldest beyond the cap
        entries: [entry, ...state.entries].slice(0, MAX_ENTRIES),
    })),
    setFilter: (filter) => set({ filter }),
    clear: () => set({ entries: [] }),
}));
// ── Selectors ─────────────────────────────────────────────────────────────────
/** Return entries matching the active filter. */
export function selectFiltered(state) {
    const { entries, filter } = state;
    if (filter === 'all')
        return entries;
    if (filter === 'error') {
        return entries.filter((e) => e.status === 'error' || e.status === 'failed');
    }
    return entries.filter((e) => e.status === filter);
}
