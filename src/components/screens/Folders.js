import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useFoldersStore } from '../../store/foldersStore';
import { useScanProgress } from '../../hooks/useScanProgress';
import { ConfirmModal } from '../shared/ConfirmModal';
export function Folders() {
    const { folders, isLoading, error, fetchFolders, addFolder, removeFolder, scanProgress } = useFoldersStore();
    const [removePath, setRemovePath] = useState(null);
    const [isAdding, setIsAdding] = useState(false);
    // Listen for scan progress events
    useScanProgress();
    useEffect(() => {
        fetchFolders();
    }, [fetchFolders]);
    const handleAddFolder = async () => {
        setIsAdding(true);
        try {
            const selected = await open({
                directory: true,
                multiple: false,
            });
            if (selected) {
                await addFolder(selected);
            }
        }
        catch (e) {
            console.error('Failed to add folder:', e);
        }
        finally {
            setIsAdding(false);
        }
    };
    const handleRemoveFolder = async () => {
        if (!removePath)
            return;
        try {
            await removeFolder(removePath);
            setRemovePath(null);
        }
        catch (e) {
            console.error('Failed to remove folder:', e);
        }
    };
    return (_jsxs("div", { className: "space-y-6", children: [_jsxs("div", { className: "flex items-center justify-between", children: [_jsxs("div", { children: [_jsx("h1", { className: "text-2xl font-bold text-gray-900 dark:text-white", children: "Watched Folders" }), _jsx("p", { className: "text-gray-600 dark:text-gray-400 mt-1", children: "Manage directories to backup automatically" })] }), _jsx("button", { onClick: handleAddFolder, disabled: isAdding, className: "px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed", children: isAdding ? 'Adding...' : 'Add Folder' })] }), error && (_jsx("div", { className: "p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md", children: _jsx("p", { className: "text-red-800 dark:text-red-400", children: error }) })), _jsx("div", { className: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700", children: isLoading ? (_jsx("div", { className: "p-8 text-center text-gray-500 dark:text-gray-400", children: "Loading folders..." })) : folders.length === 0 ? (_jsx("div", { className: "p-8 text-center text-gray-500 dark:text-gray-400", children: "No folders are being watched yet. Click \"Add Folder\" to get started." })) : (_jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "w-full", children: [_jsx("thead", { className: "border-b border-gray-200 dark:border-gray-700", children: _jsxs("tr", { children: [_jsx("th", { className: "px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider", children: "Path" }), _jsx("th", { className: "px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider", children: "Status" }), _jsx("th", { className: "px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider", children: "Last Backup" }), _jsx("th", { className: "px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider", children: "Actions" })] }) }), _jsx("tbody", { className: "divide-y divide-gray-200 dark:divide-gray-700", children: folders.map((folder) => (_jsxs("tr", { children: [_jsx("td", { className: "px-6 py-4 text-sm text-gray-900 dark:text-white font-mono", children: folder.path }), _jsx("td", { className: "px-6 py-4 text-sm", children: scanProgress[folder.path] !== undefined ? (_jsxs("div", { className: "space-y-1", children: [_jsxs("div", { className: "text-xs text-gray-600 dark:text-gray-400", children: ["Scanning... ", scanProgress[folder.path], "%"] }), _jsx("div", { className: "w-full bg-gray-200 rounded-full h-1.5 dark:bg-gray-700", children: _jsx("div", { className: "bg-blue-500 h-1.5 rounded-full transition-all", style: { width: `${scanProgress[folder.path]}%` } }) })] })) : (_jsx("span", { className: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300", children: folder.status })) }), _jsx("td", { className: "px-6 py-4 text-sm text-gray-600 dark:text-gray-400", children: "Never" }), _jsx("td", { className: "px-6 py-4 text-right text-sm", children: _jsx("button", { onClick: () => setRemovePath(folder.path), className: "text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300", children: "Remove" }) })] }, folder.path))) })] }) })) }), _jsx(ConfirmModal, { isOpen: removePath !== null, title: "Remove Folder", message: `Are you sure you want to stop watching "${removePath}"? This will not delete any existing backups.`, danger: true, confirmText: "Remove", onConfirm: handleRemoveFolder, onCancel: () => setRemovePath(null) })] }));
}
