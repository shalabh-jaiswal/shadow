interface StatusBadgeProps {
  status: 'scanning' | 'active' | 'error' | 'paused' | 'idle';
}

const statusConfig = {
  scanning: { dot: 'bg-blue-400 animate-pulse', text: 'Scanning' },
  active:   { dot: 'bg-green-400',              text: 'Active'   },
  error:    { dot: 'bg-red-400',                text: 'Error'    },
  paused:   { dot: 'bg-gray-400',               text: 'Paused'   },
  idle:     { dot: 'bg-gray-300',               text: 'Idle'     },
} as const;

export function StatusBadge({ status }: StatusBadgeProps) {
  const { dot, text } = statusConfig[status];
  return (
    <span className="inline-flex items-center gap-1.5 text-sm">
      <span className={`w-2 h-2 rounded-full ${dot}`} />
      <span className="text-gray-600 dark:text-gray-400">{text}</span>
    </span>
  );
}