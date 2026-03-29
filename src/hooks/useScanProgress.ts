import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useFoldersStore } from '../store/foldersStore';

interface ScanProgressPayload {
  folder: string;
  scanned: number;
  queued: number;
  total: number;
}

interface ScanCompletePayload {
  folder: string;
  total_files: number;
  total_bytes: number;
}

export function useScanProgress() {
  const setScanProgress = useFoldersStore((s) => s.setScanProgress);
  const clearScanProgress = useFoldersStore((s) => s.clearScanProgress);

  useEffect(() => {
    const unlistenProgress = listen<ScanProgressPayload>('scan_progress', (e) => {
      const { folder, scanned, total } = e.payload;
      const pct = total > 0 ? Math.round((scanned / total) * 100) : 0;
      setScanProgress(folder, pct);
    });

    const unlistenComplete = listen<ScanCompletePayload>('scan_complete', (e) => {
      clearScanProgress(e.payload.folder);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, [setScanProgress, clearScanProgress]);
}