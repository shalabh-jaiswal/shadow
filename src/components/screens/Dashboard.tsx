import { useEffect } from 'react';
import { useActivityStore, selectFiltered, type FilterStatus } from '../../store/activityStore';
import { useStatsStore } from '../../store/statsStore';
import { useActivityFeed } from '../../hooks/useActivityFeed';
import { useReconcileStatus } from '../../hooks/useReconcileStatus';
import { ActivityFeed } from '../shared/ActivityFeed';

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
  const { isReconciling, lastReconcile } = useReconcileStatus();

  // Subscribe to activity events
  useActivityFeed();

  // Poll stats every 5 seconds
  useEffect(() => {
    fetchStats(); // Initial fetch
    const interval = setInterval(fetchStats, 5000);
    return () => clearInterval(interval);
  }, [fetchStats]);

  const filterTabs: { key: FilterStatus; label: string }[] = [
    { key: 'all', label: 'All' },
    { key: 'uploaded', label: 'Uploaded' },
    { key: 'skipped', label: 'Skipped' },
    { key: 'error', label: 'Errors' },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Dashboard
        </h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Monitor backup activity and performance
        </p>
      </div>

      {/* Reconciliation Status */}
      {(isReconciling || (lastReconcile && lastReconcile.filesQueued > 0)) && (
        <div className="text-sm text-gray-600 dark:text-gray-400">
          {isReconciling ? (
            'Reconciling watched folders...'
          ) : lastReconcile && lastReconcile.filesQueued > 0 ? (
            `Last reconciliation queued ${lastReconcile.filesQueued} files`
          ) : null}
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