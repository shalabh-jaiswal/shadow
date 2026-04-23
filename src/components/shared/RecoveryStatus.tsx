import { useEffect, useState } from 'react';
import { useScanScheduleStore } from '../../store/scanScheduleStore';
import { ipc } from '../../ipc';

function timeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return 'just now';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return `${Math.floor(diff / 86_400_000)}d ago`;
}

function timeUntil(ts: number): string {
  const diff = ts - Date.now();
  if (diff < 60_000) return 'less than 1m';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  return `${Math.floor(diff / 86_400_000)}d`;
}

export function RecoveryStatus() {
  const [intervalMins, setIntervalMins] = useState<number | null>(null);
  const [, setNow] = useState(Date.now());

  const lastScanTime = useScanScheduleStore((s) => s.lastScanTime);
  const lastScanResult = useScanScheduleStore((s) => s.lastScanResult);
  const nextScanTime = useScanScheduleStore((s) => s.nextScanTime);

  // Update "now" every minute to drive the countdown UI
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 60000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    ipc.getConfig().then((config) => {
      setIntervalMins(config.daemon.scan_interval_mins);
    });
  }, []);

  if (intervalMins === null) {
    return null; // Loading
  }

  const isDisabled = intervalMins === 0;

  return (
    <div className="bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700/50 rounded-lg p-4">
      <div className="flex items-center gap-2 mb-3">
        <span className="text-lg">⚙️</span>
        <h2 className="text-sm font-semibold text-gray-900 dark:text-white">
          System Sync Status
        </h2>
      </div>

      <div className="space-y-3">
        <div className="flex items-start gap-2">
          <div className="mt-0.5 text-green-500">🟢</div>
          <div>
            <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
              Live Watcher: Active
            </div>
            <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              Monitoring and backing up file changes instantly as they happen.
            </div>
          </div>
        </div>

        <div className="flex items-start gap-2">
          <div className="mt-0.5 text-blue-500">🕒</div>
          <div>
            <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
              Fallback Scanner:{' '}
              {isDisabled ? (
                <span className="text-gray-500 font-normal">Disabled (Live sync only)</span>
              ) : (
                <span className="text-blue-600 dark:text-blue-400">
                  Next scan in {nextScanTime ? timeUntil(nextScanTime) : `${intervalMins}m`}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              Emergency sweep for files modified while Shadow was closed or offline.
            </div>
            {lastScanTime && lastScanResult && (
              <div className="text-[11px] text-gray-400 dark:text-gray-500 mt-1 font-mono bg-gray-100 dark:bg-gray-800/80 rounded px-1.5 py-0.5 inline-block">
                Last sweep: {timeAgo(lastScanTime)} • {lastScanResult.files_uploaded} files needed recovery
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
