import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useEffect } from 'react';
import { useActivityStore, selectFiltered } from '../../store/activityStore';
import { useStatsStore } from '../../store/statsStore';
import { useActivityFeed } from '../../hooks/useActivityFeed';
import { ActivityFeed } from '../shared/ActivityFeed';
function formatBytes(bytes) {
    if (bytes < 1024)
        return `${bytes} B`;
    if (bytes < 1024 * 1024)
        return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024)
        return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
export function Dashboard() {
    const stats = useStatsStore((s) => s.stats);
    const fetchStats = useStatsStore((s) => s.fetch);
    const filteredEntries = useActivityStore(selectFiltered);
    const filter = useActivityStore((s) => s.filter);
    const setFilter = useActivityStore((s) => s.setFilter);
    // Subscribe to activity events
    useActivityFeed();
    // Poll stats every 5 seconds
    useEffect(() => {
        fetchStats(); // Initial fetch
        const interval = setInterval(fetchStats, 5000);
        return () => clearInterval(interval);
    }, [fetchStats]);
    const filterTabs = [
        { key: 'all', label: 'All' },
        { key: 'uploaded', label: 'Uploaded' },
        { key: 'skipped', label: 'Skipped' },
        { key: 'error', label: 'Errors' },
    ];
    return (_jsxs("div", { className: "space-y-6", children: [_jsxs("div", { children: [_jsx("h1", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: "Dashboard" }), _jsx("p", { className: "text-gray-600 dark:text-gray-400 mt-1", children: "Monitor backup activity and performance" })] }), _jsxs("div", { className: "grid grid-cols-4 gap-4", children: [_jsxs("div", { className: "bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700", children: [_jsx("div", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: stats.files_uploaded.toLocaleString() }), _jsx("div", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Files Uploaded" })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700", children: [_jsx("div", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: formatBytes(stats.bytes_uploaded) }), _jsx("div", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Bytes Uploaded" })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700", children: [_jsx("div", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: stats.active_uploads }), _jsx("div", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Active Uploads" })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700", children: [_jsx("div", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: stats.queue_depth }), _jsx("div", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Queue Depth" })] })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700", children: [_jsxs("div", { className: "p-4 border-b border-gray-200 dark:border-gray-700", children: [_jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-white", children: "Activity Feed" }), _jsx("div", { className: "flex gap-2 mt-3", children: filterTabs.map((tab) => (_jsx("button", { onClick: () => setFilter(tab.key), className: `px-3 py-1 text-sm font-medium rounded-md transition-colors ${filter === tab.key
                                        ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                                        : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'}`, children: tab.label }, tab.key))) })] }), _jsx("div", { className: "p-4", children: _jsx(ActivityFeed, { entries: filteredEntries }) })] })] }));
}
