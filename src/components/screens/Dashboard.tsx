import { useEffect } from 'react';
import { useActivityStore, selectFiltered, type FilterStatus } from '../../store/activityStore';
import { useStatsStore } from '../../store/statsStore';
import { useScanProgress } from '../../hooks/useScanProgress';
import { ActivityFeed } from '../shared/ActivityFeed';
import { RecoveryStatus } from '../shared/RecoveryStatus';
import { ipc } from '../../ipc';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function Dashboard() {
  const stats = useStatsStore((s) => s.stats);
  const fetchStats = useStatsStore((s) => s.fetch);
  const filteredEntries = useActivityStore(selectFiltered);
  const filter = useActivityStore((s) => s.filter);
  const setFilter = useActivityStore((s) => s.setFilter);
  const { isScanning, scanProgress, lastScanSummary, setIsScanning, setLastScanSummary } = useScanProgress();

  // Poll stats every 5 seconds
  useEffect(() => {
    fetchStats(); // Initial fetch
    const interval = setInterval(fetchStats, 5000);
    return () => clearInterval(interval);
  }, [fetchStats]);

  const handleBackupNow = async () => {
    if (isScanning) return;
    try {
      setIsScanning(true);
      setLastScanSummary(null);
      await ipc.triggerRecoveryScan();
    } catch (e) {
      console.error(e);
      setIsScanning(false);
    }
  };

  const filterTabs: { key: FilterStatus; label: string }[] = [
    { key: 'all', label: 'All' },
    { key: 'uploaded', label: 'Uploaded' },
    { key: 'skipped', label: 'Skipped' },
    { key: 'error', label: 'Errors' },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Dashboard
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Monitor backup activity and performance
          </p>
        </div>
        <button
          onClick={handleBackupNow}
          disabled={isScanning}
          className={`px-4 py-2 rounded-md text-white text-sm font-medium transition-colors ${
            isScanning
              ? 'bg-blue-400 cursor-not-allowed'
              : 'bg-blue-600 hover:bg-blue-700'
          }`}
        >
          {isScanning ? 'Scanning...' : 'Backup Now'}
        </button>
      </div>

      {/* Progress & Summary Banners */}
      {isScanning && scanProgress && (
        <div className="p-3 bg-blue-50 border border-blue-200 rounded-md">
          <div className="flex justify-between text-sm text-blue-800 mb-1">
            <span>
              {scanProgress.trigger === 'manual' ? 'Manual scan' : scanProgress.trigger === 'scheduled' ? 'Scheduled scan' : 'Scanning'} - {scanProgress.folder}
            </span>
            <span>
              {scanProgress.scanned.toLocaleString()} / {scanProgress.total ? scanProgress.total.toLocaleString() : '?'} files
            </span>
          </div>
          {scanProgress.total > 0 && (
            <div className="w-full bg-blue-200 rounded-full h-2">
              <div
                className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                style={{ width: `${Math.min(100, (scanProgress.scanned / scanProgress.total) * 100)}%` }}
              ></div>
            </div>
          )}
        </div>
      )}

      {!isScanning && lastScanSummary && lastScanSummary.trigger === 'manual' && (
        <div className="p-3 bg-green-50 border border-green-200 rounded-md text-sm text-green-800 flex items-center gap-2">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 13l4 4L19 7"></path>
          </svg>
          Scan complete — {lastScanSummary.files_uploaded.toLocaleString()} uploaded, {lastScanSummary.files_skipped.toLocaleString()} skipped
        </div>
      )}

      {/* Stats Row */}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.files_uploaded.toLocaleString()}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">
            Files Uploaded
          </div>
        </div>
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {formatBytes(stats.bytes_uploaded)}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">
            Bytes Uploaded
          </div>
        </div>
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.active_uploads}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">
            Active Uploads
          </div>
        </div>
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.queue_depth}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">
            Queue Depth
          </div>
        </div>
      </div>

      {/* System Sync Status */}
      <RecoveryStatus />

      {/* Activity Feed */}
      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
        <div className="p-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Activity Feed
          </h2>

          {/* Filter Tabs */}
          <div className="flex gap-2 mt-3">
            {filterTabs.map((tab) => (
              <button
                key={tab.key}
                onClick={() => setFilter(tab.key)}
                className={`px-3 py-1 text-sm font-medium rounded-md transition-colors ${
                  filter === tab.key
                    ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                    : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        <div className="p-4">
          <ActivityFeed entries={filteredEntries} />
        </div>
      </div>
    </div>
  );
}