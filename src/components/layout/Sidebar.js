import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: '📊' },
    { id: 'folders', label: 'Folders', icon: '📁' },
    { id: 'providers', label: 'Providers', icon: '☁️' },
    { id: 'settings', label: 'Settings', icon: '⚙️' },
];
export function Sidebar({ current, onNavigate }) {
    return (_jsxs("div", { className: "w-64 h-screen bg-gray-100 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700", children: [_jsx("div", { className: "p-4", children: _jsx("h1", { className: "text-xl font-bold text-gray-900 dark:text-white", children: "Shadow" }) }), _jsx("nav", { className: "mt-8", children: _jsx("ul", { className: "space-y-1 px-2", children: navItems.map((item) => (_jsx("li", { children: _jsxs("button", { onClick: () => onNavigate(item.id), className: `w-full flex items-center gap-3 px-3 py-2 text-left text-sm font-medium rounded-md transition-colors ${current === item.id
                                ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                                : 'text-gray-700 hover:bg-gray-200 dark:text-gray-300 dark:hover:bg-gray-800'}`, children: [_jsx("span", { className: "text-base", children: item.icon }), item.label] }) }, item.id))) }) })] }));
}
