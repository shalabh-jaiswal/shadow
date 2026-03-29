import { create } from 'zustand';
import type { FolderStatus } from '../types';
import { ipc } from '../ipc';

interface FoldersState {
  folders: FolderStatus[];
  isLoading: boolean;
  error: string | null;
  scanProgress: Record<string, number>; // folder path → % complete (0-100), absent = not scanning
  fetchFolders: () => Promise<void>;
  addFolder: (path: string) => Promise<void>;
  removeFolder: (path: string) => Promise<void>;
  setScanProgress: (folder: string, pct: number) => void;
  clearScanProgress: (folder: string) => void;
  setLastBackup: (folderPath: string, ts: number) => void;
}

export const useFoldersStore = create<FoldersState>((set) => ({
  folders: [],
  isLoading: false,
  error: null,
  scanProgress: {},

  fetchFolders: async () => {
    set({ isLoading: true, error: null });
    try {
      const folders = await ipc.getWatchedFolders();
      set({ folders, isLoading: false });
    } catch (e) {
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

  setScanProgress: (folder, pct) =>
    set((state) => ({
      scanProgress: { ...state.scanProgress, [folder]: pct },
    })),

  clearScanProgress: (folder) =>
    set((state) => {
      const newProgress = { ...state.scanProgress };
      delete newProgress[folder];
      return { scanProgress: newProgress };
    }),

  setLastBackup: (folderPath, ts) =>
    set((state) => ({
      folders: state.folders.map((f) =>
        f.path === folderPath ? { ...f, last_backup: ts } : f
      ),
    })),
}));
