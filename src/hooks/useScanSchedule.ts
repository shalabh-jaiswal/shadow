import { useEffect, useState } from 'react';
import { events } from '../ipc';
import type { ScanCompletePayload } from '../types';

export function useScanSchedule(intervalMins: number) {
  const [lastScanTime, setLastScanTime] = useState<number | null>(null);
  const [lastScanResult, setLastScanResult] = useState<ScanCompletePayload | null>(null);
  const [nextScanTime, setNextScanTime] = useState<number | null>(null);
  const [now, setNow] = useState(Date.now());

  // Update "now" every minute to drive the countdown UI
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 60000);
    return () => clearInterval(interval);
  }, []);

  // Set initial next scan time based on when the hook mounts (approximate for first launch)
  useEffect(() => {
    if (intervalMins > 0) {
      if (!nextScanTime) {
        setNextScanTime(Date.now() + intervalMins * 60 * 1000);
      }
    } else {
      setNextScanTime(null);
    }
  }, [intervalMins]);

  // Listen for completed scheduled scans to reset the timer
  useEffect(() => {
    const unlistenComplete = events.onScanComplete((payload) => {
      // Only reset the periodic timer if it was a scheduled scan.
      // (Manual or initial scans don't interrupt the periodic interval).
      if (payload.trigger === 'scheduled') {
        setLastScanTime(Date.now());
        setLastScanResult(payload);
        if (intervalMins > 0) {
          setNextScanTime(Date.now() + intervalMins * 60 * 1000);
        }
      } else if (payload.trigger === 'manual') {
        // We can optionally update the "last scan result" for manual scans too,
        // but it doesn't reset the scheduled interval timer.
        setLastScanTime(Date.now());
        setLastScanResult(payload);
      }
    });

    return () => {
      unlistenComplete.then((fn) => fn());
    };
  }, [intervalMins]);

  return { lastScanTime, lastScanResult, nextScanTime, now };
}
