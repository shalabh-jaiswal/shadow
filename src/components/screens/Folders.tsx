import { useEffect, useState } from 'react';

function relativeTime(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return 'just now';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return `${Math.floor(diff / 86_400_000)}d ago`;
}
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { useFoldersStore } from '../../store/foldersStore';
import { useScanProgress } from '../../hooks/useScanProgress';
import { ConfirmModal } from '../shared/ConfirmModal';


export function Folders() {
  const { folders, isLoading, error, fetchFolders, addFolder, removeFolder, scanProgress, setLastBackup } = useFoldersStore();
  const [removePath, setRemovePath] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  // Listen for scan progress events
  useScanProgress();

  useEffect(() => {
    fetchFolders();
  }, [fetchFolders]);

  // Update last_backup in store directly when folder_updated fires.
  useEffect(() => {
    const unlisten = listen<{ folder: string }>('folder_updated', (e) => {
      setLastBackup(e.payload.folder, Date.now());
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [setLastBackup]);

  const handleAddFolder = async () => {
    setIsAdding(true);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected) {
        setSelectedPath(selected);
      }
    } catch (e) {
      // Handle error silently for now
    } finally {
      setIsAdding(false);
    }
  };

  const handleAddWithScanMode = async (scanExisting: boolean) => {
    if (!selectedPath) return;

    try {
      await addFolder(selectedPath, scanExisting);
      setSelectedPath(null);
    } catch (e) {
      // Handle error silently for now
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
                      <div className="flex items-center gap-2">
                        {folder.path}
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400">
                          {folder.scan_mode === 'full' ? 'All files' : 'New files only'}
                        </span>
                      </div>
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
                      {folder.last_backup ? relativeTime(folder.last_backup) : 'Never'}
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

      {/* Add Folder Modal */}
      {selectedPath && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
          <div className="w-full max-w-lg rounded-lg bg-white p-6 shadow-xl dark:bg-gray-800">
            <h3 className="mb-3 text-lg font-semibold text-gray-900 dark:text-white">
              Add Watched Folder
            </h3>
            <p className="mb-2 text-sm text-gray-700 dark:text-gray-300">
              Selected folder:
            </p>
            <p className="mb-4 text-sm font-mono bg-gray-50 dark:bg-gray-700 p-2 rounded text-gray-900 dark:text-gray-100">
              {selectedPath}
            </p>
            <p className="mb-6 text-sm text-gray-700 dark:text-gray-300">
              How would you like to back up this folder?
            </p>
            <div className="flex flex-col gap-3">
              <button
                onClick={() => handleAddWithScanMode(true)}
                className="px-4 py-3 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 text-left"
              >
                <div className="font-semibold">Back up existing files</div>
                <div className="text-xs text-blue-100 mt-1">
                  Scan and upload all files currently in the folder
                </div>
              </button>
              <button
                onClick={() => handleAddWithScanMode(false)}
                className="px-4 py-3 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 dark:bg-gray-600 dark:text-gray-300 dark:hover:bg-gray-500 text-left"
              >
                <div className="font-semibold">Going forward only</div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Only back up new or modified files from now on
                </div>
              </button>
              <button
                onClick={() => setSelectedPath(null)}
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

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