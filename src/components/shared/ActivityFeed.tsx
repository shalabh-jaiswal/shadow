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
    uploading: { icon: '↑', color: 'text-blue-500 animate-pulse' },
    queued: { icon: '…', color: 'text-gray-500' },
    renamed: { icon: '↔', color: 'text-blue-400' },
    rename_error: { icon: '✗', color: 'text-red-500' },
  } as const;

  const { icon, color } = config[status];
  return <span className={`text-lg font-bold leading-none ${color}`}>{icon}</span>;
}

function ProviderPill({ name, status }: { name: string; status: ActivityEntry['status'] }) {
  const colors = {
    uploaded: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 border-green-200 dark:border-green-800',
    skipped: 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400 border-gray-200 dark:border-gray-700',
    error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 border-red-200 dark:border-red-800',
    failed: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 border-red-200 dark:border-red-800',
    uploading: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400 border-blue-200 dark:border-blue-800 animate-pulse',
    queued: 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-500 border-gray-200 dark:border-gray-700',
    renamed: 'bg-blue-50 text-blue-600 dark:bg-blue-900/20 dark:text-blue-400 border-blue-100 dark:border-blue-800',
    rename_error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 border-red-200 dark:border-red-800',
  } as const;

  return (
    <span className={`px-2 py-0.5 rounded border text-[10px] font-bold uppercase tracking-tight ${colors[status]}`}>
      {name}
    </span>
  );
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
    <div className="space-y-1 max-h-96 overflow-y-auto pr-1">
      {entries.map((entry) => (
        <div
          key={entry.id}
          className="flex items-center gap-3 px-3 py-2 text-sm bg-white dark:bg-gray-800 border border-gray-100 dark:border-gray-700/50 rounded-lg shadow-sm"
        >
          <div className="w-6 flex justify-center">
            <StatusIcon status={entry.status} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-gray-900 dark:text-white flex items-center gap-2 truncate">
              <span className="truncate">{entry.filename}</span>
              <span className="text-[10px] uppercase tracking-wider font-semibold text-gray-400 dark:text-gray-500">
                {entry.status.replace('_', ' ')}
              </span>
            </div>
            <div className="text-gray-500 dark:text-gray-400 truncate text-[11px] mt-0.5 font-mono opacity-80">
              {(entry.status === 'renamed' || entry.status === 'rename_error') && entry.old_path ? (
                <>
                  <span className="line-through opacity-60 mr-1">{entry.old_path}</span>
                  → {entry.new_path || entry.path}
                </>
              ) : (
                entry.path
              )}
            </div>
            {entry.error && (
              <div className="text-red-500 dark:text-red-400 text-[10px] mt-1 italic">
                {entry.error}
              </div>
            )}
          </div>
          <div className="flex flex-col items-end gap-1.5 min-w-[80px]">
            <div className="flex flex-wrap justify-end gap-1">
              {Object.entries(entry.providers).map(([name, p]) => (
                <ProviderPill key={name} name={name} status={p.status} />
              ))}
            </div>
            <span className="text-[10px] text-gray-400 dark:text-gray-500 font-medium whitespace-nowrap">
              {relativeTime(entry.timestamp)}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}