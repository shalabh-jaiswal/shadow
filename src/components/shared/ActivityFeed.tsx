import type { ActivityEntry } from '../../types';

interface ActivityFeedProps {
  entries: ActivityEntry[];
}

function relativeTime(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return 'just now';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return `${Math.floor(diff / 86_400_000)}d ago`;
}

function StatusIcon({ status }: { status: ActivityEntry['status'] }) {
  const config = {
    uploaded: { icon: '✓', color: 'text-green-500' },
    skipped: { icon: '—', color: 'text-gray-500' },
    error: { icon: '✗', color: 'text-red-500' },
    failed: { icon: '✗', color: 'text-red-500' },
    uploading: { icon: '↑', color: 'text-blue-500' },
    queued: { icon: '…', color: 'text-gray-500' },
  } as const;

  const { icon, color } = config[status];
  return <span className={`text-sm font-mono ${color}`}>{icon}</span>;
}

export function ActivityFeed({ entries }: ActivityFeedProps) {
  if (entries.length === 0) {
    return (
      <div className="flex items-center justify-center h-32 text-gray-500 dark:text-gray-400">
        No activity yet
      </div>
    );
  }

  return (
    <div className="space-y-1 max-h-96 overflow-y-auto">
      {entries.map((entry) => (
        <div
          key={entry.id}
          className="flex items-center gap-3 px-3 py-2 text-sm bg-gray-50 dark:bg-gray-800 rounded-md"
        >
          <StatusIcon status={entry.status} />
          <div className="flex-1 min-w-0">
            <div className="font-medium text-gray-900 dark:text-white truncate">
              {entry.filename}
            </div>
            <div className="text-gray-600 dark:text-gray-400 truncate">
              {entry.path}
            </div>
            {entry.error && (
              <div className="text-red-600 dark:text-red-400 text-xs mt-1">
                {entry.error}
              </div>
            )}
          </div>
          <div className="flex flex-col items-end text-xs text-gray-500 dark:text-gray-400">
            {entry.provider && (
              <span className="px-1.5 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs font-mono">
                {entry.provider}
              </span>
            )}
            <span className="mt-1">{relativeTime(entry.timestamp)}</span>
          </div>
        </div>
      ))}
    </div>
  );
}