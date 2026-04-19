import { useEffect, useState } from 'react';
import { events } from '../ipc';
import type { ScanProgressPayload, ScanCompletePayload } from '../types';

export function useScanProgress() {
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgressPayload | null>(null);
  const [lastScanSummary, setLastScanSummary] = useState<ScanCompletePayload | null>(null);

  useEffect(() => {
    const unlistenProgress = events.onScanProgress((payload) => {
      setIsScanning(true);
      setScanProgress(payload);
    });

    const unlistenComplete = events.onScanComplete((payload) => {
      // The scanner might emit scan_complete multiple times if multiple folders are watched.
      // However, for UI purposes, we'll just track the last folder's completion.
      // In a real app, you might want to aggregate these or show when ALL folders finish.
      // For now, we will clear the scanning state so the progress bar hides,
      // and show the last scan's summary.
      setIsScanning(false);
      setScanProgress(null);
      setLastScanSummary(payload);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, []);

  return { isScanning, scanProgress, lastScanSummary, setIsScanning, setLastScanSummary };
}