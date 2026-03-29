import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function ConfirmModal({ isOpen, title, message, danger = false, confirmText = 'Confirm', cancelText = 'Cancel', onConfirm, onCancel, }) {
    if (!isOpen)
        return null;
    return (_jsx("div", { className: "fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50", children: _jsxs("div", { className: "w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800", children: [_jsx("h3", { className: "mb-3 text-lg font-semibold text-gray-900 dark:text-white", children: title }), _jsx("p", { className: "mb-6 text-sm text-gray-700 dark:text-gray-300", children: message }), _jsxs("div", { className: "flex justify-end gap-3", children: [_jsx("button", { onClick: onCancel, className: "px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 dark:bg-gray-600 dark:text-gray-300 dark:hover:bg-gray-500", children: cancelText }), _jsx("button", { onClick: onConfirm, className: `px-4 py-2 text-sm font-medium text-white rounded-md ${danger
                                ? 'bg-red-600 hover:bg-red-700'
                                : 'bg-blue-600 hover:bg-blue-700'}`, children: confirmText })] })] }) }));
}
