import { useEffect, useState } from 'react';
import { ipc } from '../../ipc';
import { ConfirmModal } from '../shared/ConfirmModal';
import type { DaemonConfig, MachineConfig, AppConfig } from '../../types';

interface NumberInputProps {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  suffix?: string;
  description?: string;
}

function NumberInput({ label, value, onChange, min, max, suffix, description }: NumberInputProps) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        {label}
      </label>
      <div className="flex items-center gap-2">
        <input
          type="number"
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          min={min}
          max={max}
          className="w-32 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
        />
        {suffix && (
          <span className="text-sm text-gray-600 dark:text-gray-400">{suffix}</span>
        )}
      </div>
      {description && (
        <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{description}</p>
      )}
    </div>
  );
}

interface TextInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  description?: string;
}

function TextInput({ label, value, onChange, placeholder, description }: TextInputProps) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        {label}
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
      />
      {description && (
        <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{description}</p>
      )}
    </div>
  );
}

const SCAN_INTERVAL_OPTIONS = [
  { label: 'Disabled', value: 0 },
  { label: 'Every 15 minutes', value: 15 },
  { label: 'Every 30 minutes', value: 30 },
  { label: 'Every hour', value: 60 },
  { label: 'Every 6 hours', value: 360 },
  { label: 'Every 24 hours', value: 1440 },
] as const;

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [showClearModal, setShowClearModal] = useState(false);
  const [isClearing, setIsClearing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isCheckingUpdates, setIsCheckingUpdates] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [logPath, setLogPath] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
    ipc.getLogPath().then(setLogPath).catch(() => {});
  }, []);

  const loadConfig = async () => {
    try {
      const cfg = await ipc.getConfig();
      setConfig(cfg);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const saveConfig = async () => {
    if (!config) return;

    setIsSaving(true);
    setError(null);
    try {
      await ipc.setDaemonConfig(config.daemon, config.machine);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsSaving(false);
    }
  };

  const setupOsIntegration = async () => {
    try {
      await ipc.setupOsIntegration();
      // Show a temporary success message via error state (misnamed but works for simple feedback)
      setError('OS Integration repaired successfully');
      setTimeout(() => setError(null), 3000);
    } catch (e) {
      setError(String(e));
    }
  };

  const clearHashStore = async () => {
    setIsClearing(true);
    try {
      await ipc.clearHashStore();
      setShowClearModal(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsClearing(false);
    }
  };

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

  const toggleAutostart = async (enabled: boolean) => {
    try {
      await ipc.setAutostart(enabled);
      updateDaemon({ start_on_login: enabled });
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const updateDaemon = (updates: Partial<DaemonConfig>) => {
    if (!config) return;
    setConfig({
      ...config,
      daemon: { ...config.daemon, ...updates },
    });
  };

  const updateMachine = (updates: Partial<MachineConfig>) => {
    if (!config) return;
    setConfig({
      ...config,
      machine: { ...config.machine, ...updates },
    });
  };

  if (!config) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-500 dark:text-gray-400">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Settings
        </h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Configure daemon behavior and machine settings
        </p>
      </div>

      {error && (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-red-800 dark:text-red-400">{error}</p>
        </div>
      )}

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Machine Configuration
        </h2>
        <div className="space-y-4">
          <TextInput
            label="Machine Name"
            value={config.machine.name}
            onChange={(name) => updateMachine({ name })}
            placeholder="Leave empty to use OS hostname"
            description="Used in remote backup paths to identify this machine"
          />
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Application Settings
        </h2>
        <div className="space-y-6">
          <div className="flex items-center">
            <input
              type="checkbox"
              checked={config.daemon.start_on_login}
              onChange={(e) => toggleAutostart(e.target.checked)}
              className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
              Launch at login
            </label>
          </div>

          <div>
            <button
              onClick={checkForUpdates}
              disabled={isCheckingUpdates}
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
            >
              {isCheckingUpdates ? 'Checking...' : 'Check for Updates'}
            </button>
            {updateStatus && (
              <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                {updateStatus}
              </p>
            )}
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Daemon Configuration
        </h2>
        <div className="space-y-6">
          <NumberInput
            label="Debounce Time"
            value={config.daemon.debounce_ms}
            onChange={(debounce_ms) => updateDaemon({ debounce_ms })}
            min={50}
            max={2000}
            suffix="ms"
            description="Wait time before processing file changes (prevents rapid duplicate uploads)"
          />

          <NumberInput
            label="Upload Concurrency"
            value={config.daemon.upload_workers}
            onChange={(upload_workers) => updateDaemon({ upload_workers })}
            min={1}
            max={16}
            description="Maximum number of simultaneous uploads"
          />

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Log Level
            </label>
            <select
              value={config.daemon.log_level}
              onChange={(e) => updateDaemon({ log_level: e.target.value })}
              className="w-48 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
            >
              <option value="error">Error</option>
              <option value="warn">Warning</option>
              <option value="info">Info</option>
              <option value="debug">Debug</option>
              <option value="trace">Trace</option>
            </select>
          </div>

          <div className="flex items-center">
            <input
              type="checkbox"
              checked={config.daemon.follow_symlinks}
              onChange={(e) => updateDaemon({ follow_symlinks: e.target.checked })}
              className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label className="ml-2 block text-sm text-gray-700 dark:text-gray-300">
              Follow symbolic links
            </label>
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Backup Schedule
        </h2>
        <div className="space-y-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Periodic Scan Interval
            </label>
            <select
              value={config.daemon.scan_interval_mins}
              onChange={(e) => updateDaemon({ scan_interval_mins: Number(e.target.value) })}
              className="w-64 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
            >
              {SCAN_INTERVAL_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
            {config.daemon.scan_interval_mins > 0 && (
              <p className="text-sm text-gray-600 dark:text-gray-400 mt-2">
                Scheduled scan: Next scan in {config.daemon.scan_interval_mins} minutes (approximate)
              </p>
            )}
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              How often Shadow runs a full recovery scan across all watched folders.
            </p>
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Maintenance
        </h2>
        <div className="space-y-4">
          <div>
            <button
              onClick={() => setShowClearModal(true)}
              className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
            >
              Clear Hash Store
            </button>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              Remove all cached file hashes. This will cause all files to be re-scanned on next startup.
            </p>
          </div>

          <div>
            <button
              onClick={() => ipc.openLogFolder().catch((e) => setError(String(e)))}
              className="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700"
            >
              Open Log Folder
            </button>
            {logPath && (
              <p className="text-sm text-gray-600 dark:text-gray-400 mt-1 font-mono break-all">
                {logPath}
              </p>
            )}
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              Daily rotating log files. Log level changes take effect on next restart.
            </p>
          </div>

          <div>
            <button
              onClick={setupOsIntegration}
              className="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700"
            >
              Repair OS Integration
            </button>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              Re-install right-click "Send to Shadow" (Windows) or "Quick Actions" (macOS) integration.
            </p>
          </div>
        </div>
      </div>

      <div className="flex justify-end">
        <button
          onClick={saveConfig}
          disabled={isSaving}
          className="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSaving ? 'Saving...' : 'Save Settings'}
        </button>
      </div>

      <ConfirmModal
        isOpen={showClearModal}
        title="Clear Hash Store"
        message="Are you sure you want to clear the hash store? This will cause all files to be re-scanned and potentially re-uploaded on next startup."
        danger
        confirmText={isClearing ? 'Clearing...' : 'Clear'}
        onConfirm={clearHashStore}
        onCancel={() => setShowClearModal(false)}
      />
    </div>
  );
}