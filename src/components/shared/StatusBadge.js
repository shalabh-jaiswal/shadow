import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
const statusConfig = {
    scanning: { dot: 'bg-blue-400 animate-pulse', text: 'Scanning' },
    active: { dot: 'bg-green-400', text: 'Active' },
    error: { dot: 'bg-red-400', text: 'Error' },
    paused: { dot: 'bg-gray-400', text: 'Paused' },
    idle: { dot: 'bg-gray-300', text: 'Idle' },
};
export function StatusBadge({ status }) {
    const { dot, text } = statusConfig[status];
    return (_jsxs("span", { className: "inline-flex items-center gap-1.5 text-sm", children: [_jsx("span", { className: `w-2 h-2 rounded-full ${dot}` }), _jsx("span", { className: "text-gray-600 dark:text-gray-400", children: text })] }));
}
