import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useFoldersStore } from '../../store/foldersStore';
import { useScanProgress } from '../../hooks/useScanProgress';
import { ConfirmModal } from '../shared/ConfirmModal';


export function Folders() {
  const { folders, isLoading, error, fetchFolders, addFolder, removeFolder, scanProgress } = useFoldersStore();
  const [removePath, setRemovePath] = useState<string | null>(null);
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
    } catch (e) {
      console.error('Failed to add folder:', e);
    } finally {
      setIsAdding(false);
    }
  };

  const handleRemoveFolder = async () => {
    if (!removePath) return;

    try {
      await removeFolder(removePath);
      setRemovePath(null);
    } catch (e) {
      console.error('Failed to remove folder:', e);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Watched Folders
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Manage directories to backup automatically
          </p>
        </div>
        <button
          onClick={handleAddFolder}
          disabled={isAdding}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isAdding ? 'Adding...' : 'Add Folder'}
        </button>
      </div>

      {error && (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-red-800 dark:text-red-400">{error}</p>
        </div>
      )}

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
        {isLoading ? (
          <div className="p-8 text-center text-gray-500 dark:text-gray-400">
            Loading folders...
          </div>
        ) : folders.length === 0 ? (
          <div className="p-8 text-center text-gray-500 dark:text-gray-400">
            No folders are being watched yet. Click "Add Folder" to get started.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="border-b border-gray-200 dark:border-gray-700">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Path
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Last Backup
                  </th>
                  <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                {folders.map((folder) => (
                  <tr key={folder.path}>
                    <td className="px-6 py-4 text-sm text-gray-900 dark:text-white font-mono">
                      {folder.path}
                    </td>
                    <td className="px-6 py-4 text-sm">
                      {scanProgress[folder.path] !== undefined ? (
                        <div className="space-y-1">
                          <div className="text-xs text-gray-600 dark:text-gray-400">
                            Scanning... {scanProgress[folder.path]}%
                          </div>
                          <div className="w-full bg-gray-200 rounded-full h-1.5 dark:bg-gray-700">
                            <div
                              className="bg-blue-500 h-1.5 rounded-full transition-all"
                              style={{ width: `${scanProgress[folder.path]}%` }}
                            />
                          </div>
                        </div>
                      ) : (
                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300">
                          {folder.status}
                        </span>
                      )}
                    </td>
                    <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-400">
                      Never
                    </td>
                    <td className="px-6 py-4 text-right text-sm">
                      <button
                        onClick={() => setRemovePath(folder.path)}
                        className="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300"
                      >
                        Remove
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <ConfirmModal
        isOpen={removePath !== null}
        title="Remove Folder"
        message={`Are you sure you want to stop watching "${removePath}"? This will not delete any existing backups.`}
        danger
        confirmText="Remove"
        onConfirm={handleRemoveFolder}
        onCancel={() => setRemovePath(null)}
      />
    </div>
  );
}