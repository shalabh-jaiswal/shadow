import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
function relativeTime(ts) {
    const diff = Date.now() - ts;
    if (diff < 60_000)
        return 'just now';
    if (diff < 3_600_000)
        return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000)
        return `${Math.floor(diff / 3_600_000)}h ago`;
    return `${Math.floor(diff / 86_400_000)}d ago`;
}
function StatusIcon({ status }) {
    const config = {
        uploaded: { icon: '✓', color: 'text-green-500' },
        skipped: { icon: '—', color: 'text-gray-500' },
        error: { icon: '✗', color: 'text-red-500' },
        failed: { icon: '✗', color: 'text-red-500' },
        uploading: { icon: '↑', color: 'text-blue-500' },
        queued: { icon: '…', color: 'text-gray-500' },
    };
    const { icon, color } = config[status];
    return _jsx("span", { className: `text-sm font-mono ${color}`, children: icon });
}
export function ActivityFeed({ entries }) {
    if (entries.length === 0) {
        return (_jsx("div", { className: "flex items-center justify-center h-32 text-gray-500 dark:text-gray-400", children: "No activity yet" }));
    }
    return (_jsx("div", { className: "space-y-1 max-h-96 overflow-y-auto", children: entries.map((entry) => (_jsxs("div", { className: "flex items-center gap-3 px-3 py-2 text-sm bg-gray-50 dark:bg-gray-800 rounded-md", children: [_jsx(StatusIcon, { status: entry.status }), _jsxs("div", { className: "flex-1 min-w-0", children: [_jsx("div", { className: "font-medium text-gray-900 dark:text-white truncate", children: entry.filename }), _jsx("div", { className: "text-gray-600 dark:text-gray-400 truncate", children: entry.path }), entry.error && (_jsx("div", { className: "text-red-600 dark:text-red-400 text-xs mt-1", children: entry.error }))] }), _jsxs("div", { className: "flex flex-col items-end text-xs text-gray-500 dark:text-gray-400", children: [entry.provider && (_jsx("span", { className: "px-1.5 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs font-mono", children: entry.provider })), _jsx("span", { className: "mt-1", children: relativeTime(entry.timestamp) })] })] }, entry.id))) }));
}
