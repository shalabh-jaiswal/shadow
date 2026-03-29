import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Sidebar } from './Sidebar';
export function Layout({ current, onNavigate, children }) {
    return (_jsxs("div", { className: "flex h-screen bg-white dark:bg-gray-900", children: [_jsx(Sidebar, { current: current, onNavigate: onNavigate }), _jsx("main", { className: "flex-1 overflow-y-auto", children: _jsx("div", { className: "p-6", children: children }) })] }));
}
