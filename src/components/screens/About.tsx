import { useState } from 'react';
import { ipc } from '../../ipc';

const APP_VERSION = '0.3.0';

export function About() {
  const [isCheckingUpdates, setIsCheckingUpdates] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const checkForUpdates = async () => {
    setIsCheckingUpdates(true);
    setUpdateStatus(null);
    setError(null);
    try {
      const newVersion = await ipc.checkForUpdates();
      if (newVersion) {
        setUpdateStatus(`Version ${newVersion} available`);
      } else {
        setUpdateStatus('Up to date');
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsCheckingUpdates(false);
    }
  };

  const openConfigFolder = async () => {
    try {
      await ipc.openConfigFolder();
    } catch (e) {
      setError(String(e));
    }
  };

  const openDataFolder = async () => {
    try {
      await ipc.openDataFolder();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
          Shadow
        </h1>
        <p className="text-lg text-gray-600 dark:text-gray-400 mt-2">
          Version {APP_VERSION}
        </p>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Real-time file backup to S3, GCS, and NAS.
        </p>
      </div>

      {error && (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-red-800 dark:text-red-400">{error}</p>
        </div>
      )}

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <div>
          <button
            onClick={checkForUpdates}
            disabled={isCheckingUpdates}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {isCheckingUpdates ? 'Checking...' : 'Check for Updates'}
          </button>
          {updateStatus && (
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-2">
              {updateStatus}
            </p>
          )}
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Folders
        </h2>
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-700 dark:text-gray-300">Config folder</p>
              <p className="text-sm text-gray-600 dark:text-gray-400">~/Library/Application Support/shadow</p>
            </div>
            <button
              onClick={openConfigFolder}
              className="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600"
            >
              Open
            </button>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-700 dark:text-gray-300">Data folder</p>
              <p className="text-sm text-gray-600 dark:text-gray-400">~/.shadow</p>
            </div>
            <button
              onClick={openDataFolder}
              className="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600"
            >
              Open
            </button>
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Links
        </h2>
        <div className="space-y-2">
          <div>
            <button
              onClick={() => ipc.openUrl('https://github.com/shalabh-jaiswal/shadow')}
              className="text-blue-600 dark:text-blue-400 hover:underline inline-flex items-center gap-1 text-sm"
            >
              GitHub
              <span className="text-xs">↗</span>
            </button>
          </div>
          <div>
            <button
              onClick={() => ipc.openUrl('https://github.com/shalabh-jaiswal/shadow/issues/new')}
              className="text-blue-600 dark:text-blue-400 hover:underline inline-flex items-center gap-1 text-sm"
            >
              Report a Bug
              <span className="text-xs">↗</span>
            </button>
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <p className="text-sm text-gray-600 dark:text-gray-400">
          MIT License · © 2026 Shalabh Jaiswal
        </p>
      </div>
    </div>
  );
}