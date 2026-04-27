import { useEffect, useState } from 'react';
import { useScanScheduleStore } from '../../store/scanScheduleStore';
import { events, ipc } from '../../ipc';

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

interface RecoveryStatusProps {
  isScanning: boolean;
  onBackupNow: () => Promise<void>;
}

export function RecoveryStatus({ isScanning, onBackupNow }: RecoveryStatusProps) {
  const [intervalMins, setIntervalMins] = useState<number | null>(null);
  const [, setNow] = useState(Date.now());

  const lastScanTime = useScanScheduleStore((s) => s.lastScanTime);
  const lastScanResult = useScanScheduleStore((s) => s.lastScanResult);
  const nextScanTime = useScanScheduleStore((s) => s.nextScanTime);

  // Update "now" every 10s so the countdown stays responsive near the boundary
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 10000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    ipc.getConfig().then((config) => {
      setIntervalMins(config.daemon.scan_interval_mins);
    });

    const unlisten = events.onConfigChanged(() => {
      ipc.getConfig().then((config) => {
        setIntervalMins(config.daemon.scan_interval_mins);
      });
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  if (intervalMins === null) {
    return null; // Loading
  }

  const isDisabled = intervalMins === 0;

  return (
    <div className="bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700/50 rounded-lg p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <span className="text-lg">⚙️</span>
          <h2 className="text-sm font-semibold text-gray-900 dark:text-white">
            System Sync Status
          </h2>
        </div>
        
        <button
          onClick={onBackupNow}
          disabled={isScanning}
          className={`px-3 py-1 rounded text-[11px] font-bold uppercase tracking-wider transition-colors ${
            isScanning
              ? 'bg-blue-100 text-blue-400 cursor-not-allowed dark:bg-blue-900/20 dark:text-blue-700'
              : 'bg-blue-600 text-white hover:bg-blue-700 shadow-sm'
          }`}
        >
          {isScanning ? 'Scanning...' : 'Backup Now'}
        </button>
      </div>

      <div className="space-y-4">
        <div className="flex items-start gap-2">
          <div className="mt-0.5 text-green-500 shrink-0">🟢</div>
          <div>
            <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
              Live Watcher: Active
            </div>
            <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              Monitoring and backing up file changes instantly as they happen.
            </div>
          </div>
        </div>

        <div className="flex items-start gap-2 border-t border-gray-200 dark:border-gray-700/50 pt-3">
          <div className={`mt-0.5 shrink-0 ${isScanning ? 'animate-spin' : 'text-blue-500'}`}>
            {isScanning ? '🔄' : '🕒'}
          </div>
          <div className="flex-1">
            <div className="text-sm font-medium text-gray-900 dark:text-gray-100 flex items-center gap-2">
              Fallback Scanner:
              {isScanning ? (
                <span className="text-blue-600 dark:text-blue-400 animate-pulse font-bold">
                  Running emergency sweep...
                </span>
              ) : isDisabled ? (
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
              <div className="text-[11px] text-gray-400 dark:text-gray-500 mt-1.5 font-mono bg-white dark:bg-gray-800 border border-gray-100 dark:border-gray-700 rounded px-1.5 py-0.5 inline-block shadow-sm">
                Last sweep: {timeAgo(lastScanTime)} • {lastScanResult.files_uploaded} files recovered
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
