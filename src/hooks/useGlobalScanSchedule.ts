import { useEffect } from 'react';
import { events, ipc } from '../ipc';
import { useScanScheduleStore } from '../store/scanScheduleStore';

export function useGlobalScanSchedule() {
  const setLastScanTime = useScanScheduleStore((s) => s.setLastScanTime);
  const setLastScanResult = useScanScheduleStore((s) => s.setLastScanResult);
  const setNextScanTime = useScanScheduleStore((s) => s.setNextScanTime);

  // Initialize the next scan time on startup based on the config
  useEffect(() => {
    ipc.getConfig().then((config) => {
      const intervalMins = config.daemon.scan_interval_mins;
      if (intervalMins > 0) {
        // Only set it if it hasn't been set yet (e.g. by a hot reload)
        if (!useScanScheduleStore.getState().nextScanTime) {
          setNextScanTime(Date.now() + intervalMins * 60 * 1000);
        }
      } else {
        setNextScanTime(null);
      }
    }).catch(console.error);
  }, [setNextScanTime]);

  // When daemon config changes, reset nextScanTime with the new interval from now
  useEffect(() => {
    const unlisten = events.onConfigChanged(() => {
      ipc.getConfig().then((config) => {
        const intervalMins = config.daemon.scan_interval_mins;
        if (intervalMins > 0) {
          setNextScanTime(Date.now() + intervalMins * 60 * 1000);
        } else {
          setNextScanTime(null);
        }
      }).catch(console.error);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [setNextScanTime]);

  // Listen for scheduled scans completing to reset the timer
  useEffect(() => {
    const unlistenComplete = events.onScanComplete((payload) => {
      if (payload.trigger === 'scheduled') {
        setLastScanTime(Date.now());
        setLastScanResult(payload);
        
        // Re-fetch config to get the latest interval
        ipc.getConfig().then((config) => {
          const intervalMins = config.daemon.scan_interval_mins;
          if (intervalMins > 0) {
            setNextScanTime(Date.now() + intervalMins * 60 * 1000);
          } else {
            setNextScanTime(null);
          }
        }).catch(console.error);
        
      } else if (payload.trigger === 'manual') {
        // Manual scans update the history but do NOT reset the timer
        setLastScanTime(Date.now());
        setLastScanResult(payload);
      }
    });

    return () => {
      unlistenComplete.then((fn) => fn());
    };
  }, [setLastScanTime, setLastScanResult, setNextScanTime]);
}
