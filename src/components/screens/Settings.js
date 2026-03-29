import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useEffect, useState } from 'react';
import { ipc } from '../../ipc';
import { ConfirmModal } from '../shared/ConfirmModal';
function NumberInput({ label, value, onChange, min, max, suffix, description }) {
    return (_jsxs("div", { children: [_jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: label }), _jsxs("div", { className: "flex items-center gap-2", children: [_jsx("input", { type: "number", value: value, onChange: (e) => onChange(Number(e.target.value)), min: min, max: max, className: "w-32 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white" }), suffix && (_jsx("span", { className: "text-sm text-gray-600 dark:text-gray-400", children: suffix }))] }), description && (_jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mt-1", children: description }))] }));
}
function TextInput({ label, value, onChange, placeholder, description }) {
    return (_jsxs("div", { children: [_jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: label }), _jsx("input", { type: "text", value: value, onChange: (e) => onChange(e.target.value), placeholder: placeholder, className: "w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white" }), description && (_jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mt-1", children: description }))] }));
}
export function Settings() {
    const [config, setConfig] = useState(null);
    const [isSaving, setIsSaving] = useState(false);
    const [showClearModal, setShowClearModal] = useState(false);
    const [isClearing, setIsClearing] = useState(false);
    const [error, setError] = useState(null);
    useEffect(() => {
        loadConfig();
    }, []);
    const loadConfig = async () => {
        try {
            const cfg = await ipc.getConfig();
            setConfig(cfg);
            setError(null);
        }
        catch (e) {
            setError(String(e));
        }
    };
    const saveConfig = async () => {
        if (!config)
            return;
        setIsSaving(true);
        setError(null);
        try {
            await ipc.setDaemonConfig(config.daemon, config.machine);
        }
        catch (e) {
            setError(String(e));
        }
        finally {
            setIsSaving(false);
        }
    };
    const clearHashStore = async () => {
        setIsClearing(true);
        try {
            await ipc.clearHashStore();
            setShowClearModal(false);
        }
        catch (e) {
            setError(String(e));
        }
        finally {
            setIsClearing(false);
        }
    };
    const updateDaemon = (updates) => {
        if (!config)
            return;
        setConfig({
            ...config,
            daemon: { ...config.daemon, ...updates },
        });
    };
    const updateMachine = (updates) => {
        if (!config)
            return;
        setConfig({
            ...config,
            machine: { ...config.machine, ...updates },
        });
    };
    if (!config) {
        return (_jsx("div", { className: "flex items-center justify-center h-64", children: _jsx("div", { className: "text-gray-500 dark:text-gray-400", children: "Loading settings..." }) }));
    }
    return (_jsxs("div", { className: "space-y-6", children: [_jsxs("div", { children: [_jsx("h1", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: "Settings" }), _jsx("p", { className: "text-gray-600 dark:text-gray-400 mt-1", children: "Configure daemon behavior and machine settings" })] }), error && (_jsx("div", { className: "p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md", children: _jsx("p", { className: "text-red-800 dark:text-red-400", children: error }) })), _jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6", children: [_jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-white mb-4", children: "Machine Configuration" }), _jsx("div", { className: "space-y-4", children: _jsx(TextInput, { label: "Machine Name", value: config.machine.name, onChange: (name) => updateMachine({ name }), placeholder: "Leave empty to use OS hostname", description: "Used in remote backup paths to identify this machine" }) })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6", children: [_jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-white mb-4", children: "Daemon Configuration" }), _jsxs("div", { className: "space-y-6", children: [_jsx(NumberInput, { label: "Debounce Time", value: config.daemon.debounce_ms, onChange: (debounce_ms) => updateDaemon({ debounce_ms }), min: 50, max: 2000, suffix: "ms", description: "Wait time before processing file changes (prevents rapid duplicate uploads)" }), _jsx(NumberInput, { label: "Upload Concurrency", value: config.daemon.upload_workers, onChange: (upload_workers) => updateDaemon({ upload_workers }), min: 1, max: 16, description: "Maximum number of simultaneous uploads" }), _jsxs("div", { children: [_jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: "Log Level" }), _jsxs("select", { value: config.daemon.log_level, onChange: (e) => updateDaemon({ log_level: e.target.value }), className: "w-48 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white", children: [_jsx("option", { value: "error", children: "Error" }), _jsx("option", { value: "warn", children: "Warning" }), _jsx("option", { value: "info", children: "Info" }), _jsx("option", { value: "debug", children: "Debug" }), _jsx("option", { value: "trace", children: "Trace" })] })] }), _jsxs("div", { className: "flex items-center", children: [_jsx("input", { type: "checkbox", checked: config.daemon.follow_symlinks, onChange: (e) => updateDaemon({ follow_symlinks: e.target.checked }), className: "h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded" }), _jsx("label", { className: "ml-2 block text-sm text-gray-700 dark:text-gray-300", children: "Follow symbolic links" })] })] })] }), _jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6", children: [_jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-white mb-4", children: "Maintenance" }), _jsx("div", { className: "space-y-4", children: _jsxs("div", { children: [_jsx("button", { onClick: () => setShowClearModal(true), className: "px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700", children: "Clear Hash Store" }), _jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mt-1", children: "Remove all cached file hashes. This will cause all files to be re-scanned on next startup." })] }) })] }), _jsx("div", { className: "flex justify-end", children: _jsx("button", { onClick: saveConfig, disabled: isSaving, className: "px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50", children: isSaving ? 'Saving...' : 'Save Settings' }) }), _jsx(ConfirmModal, { isOpen: showClearModal, title: "Clear Hash Store", message: "Are you sure you want to clear the hash store? This will cause all files to be re-scanned and potentially re-uploaded on next startup.", danger: true, confirmText: isClearing ? 'Clearing...' : 'Clear', onConfirm: clearHashStore, onCancel: () => setShowClearModal(false) })] }));
}
