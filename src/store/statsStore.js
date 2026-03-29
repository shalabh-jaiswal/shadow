import { create } from 'zustand';
import { ipc } from '../ipc';
const DEFAULT_STATS = {
    files_uploaded: 0,
    bytes_uploaded: 0,
    active_uploads: 0,
    queue_depth: 0,
};
export const useStatsStore = create((set) => ({
    stats: DEFAULT_STATS,
    fetch: async () => {
        try {
            const stats = await ipc.getStats();
            set({ stats });
        }
        catch {
            // Silently ignore — UI keeps showing last known values
        }
    },
}));
