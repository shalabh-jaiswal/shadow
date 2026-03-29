import { create } from 'zustand';
import { ipc } from '../ipc';
export const useFoldersStore = create((set) => ({
    folders: [],
    isLoading: false,
    error: null,
    scanProgress: {},
    fetchFolders: async () => {
        set({ isLoading: true, error: null });
        try {
            const folders = await ipc.getWatchedFolders();
            set({ folders, isLoading: false });
        }
        catch (e) {
            set({ isLoading: false, error: String(e) });
        }
    },
    addFolder: async (path) => {
        await ipc.addFolder(path);
        // Re-fetch from daemon as single source of truth
        const folders = await ipc.getWatchedFolders();
        set({ folders });
    },
    removeFolder: async (path) => {
        await ipc.removeFolder(path);
        // Optimistic local removal — daemon has already removed it
        set((state) => ({
            folders: state.folders.filter((f) => f.path !== path),
        }));
    },
    setScanProgress: (folder, pct) => set((state) => ({
        scanProgress: { ...state.scanProgress, [folder]: pct },
    })),
    clearScanProgress: (folder) => set((state) => {
        const newProgress = { ...state.scanProgress };
        delete newProgress[folder];
        return { scanProgress: newProgress };
    }),
}));
