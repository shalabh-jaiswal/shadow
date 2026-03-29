import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useFoldersStore } from '../store/foldersStore';
export function useScanProgress() {
    const setScanProgress = useFoldersStore((s) => s.setScanProgress);
    const clearScanProgress = useFoldersStore((s) => s.clearScanProgress);
    useEffect(() => {
        const unlistenProgress = listen('scan_progress', (e) => {
            const { folder, scanned, total } = e.payload;
            const pct = total > 0 ? Math.round((scanned / total) * 100) : 0;
            setScanProgress(folder, pct);
        });
        const unlistenComplete = listen('scan_complete', (e) => {
            clearScanProgress(e.payload.folder);
        });
        return () => {
            unlistenProgress.then((fn) => fn());
            unlistenComplete.then((fn) => fn());
        };
    }, [setScanProgress, clearScanProgress]);
}
