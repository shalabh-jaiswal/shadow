import { useEffect } from 'react';
import { useProviderStore } from '../../store/providerStore';
import { useProviderStatus } from '../../hooks/useProviderStatus';

interface ProviderCardProps {
  title: string;
  description: string;
  enabled: boolean;
  onToggle: (enabled: boolean) => Promise<void>;
  onSave: () => Promise<void>;
  onTest: () => Promise<void>;
  isSaving: boolean;
  testResult?: { status: string; message: string };
  children: React.ReactNode;
}

function ProviderCard({
  title,
  description,
  enabled,
  onToggle,
  onSave,
  onTest,
  isSaving,
  testResult,
  children,
}: ProviderCardProps) {
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            {title}
          </h3>
          <p className="text-sm text-gray-600 dark:text-gray-400">
            {description}
          </p>
        </div>
        <label className="relative inline-flex items-center cursor-pointer">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => onToggle(e.target.checked)}
            className="sr-only peer"
          />
          <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" />
        </label>
      </div>

      {enabled && (
        <div className="space-y-4">
          {children}

          <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
            <div className="flex items-center gap-3">
              {testResult && (
                <span
                  className={`text-sm ${
                    testResult.status === 'ok'
                      ? 'text-green-600 dark:text-green-400'
                      : testResult.status === 'error'
                      ? 'text-red-600 dark:text-red-400'
                      : 'text-gray-600 dark:text-gray-400'
                  }`}
                >
                  {testResult.status === 'testing'
                    ? 'Testing...'
                    : testResult.status === 'ok'
                    ? '✓ Connected'
                    : testResult.message || 'Connection failed'}
                </span>
              )}
            </div>
            <div className="flex gap-2">
              <button
                onClick={onTest}
                disabled={isSaving}
                className="px-3 py-1.5 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600 disabled:opacity-50"
              >
                Test Connection
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Always visible footer with Save button */}
      <div className="flex justify-end pt-4 border-t border-gray-200 dark:border-gray-700">
        <button
          onClick={onSave}
          disabled={isSaving}
          className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSaving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  );
}

interface TextInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  required?: boolean;
}

function TextInput({ label, value, onChange, placeholder, required }: TextInputProps) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        {label}
        {required && <span className="text-red-500 ml-1">*</span>}
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
      />
    </div>
  );
}

export function Providers() {
  const {
    s3,
    gcs,
    nas,
    testResults,
    isSaving,
    load,
    setS3,
    setGcs,
    setNas,
    saveS3,
    saveGcs,
    saveNas,
    testProvider,
  } = useProviderStore();

  // Subscribe to provider status events
  useProviderStatus();

  useEffect(() => {
    load();
  }, [load]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Backup Providers
        </h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Configure where your files are backed up
        </p>
      </div>

      <div className="space-y-6">
        {/* S3 Provider */}
        <ProviderCard
          title="Amazon S3"
          description="Back up to Amazon S3 storage"
          enabled={s3.enabled}
          onToggle={async (enabled) => {
            const updated = { ...s3, enabled };
            setS3(updated);
            await saveS3(updated);
          }}
          onSave={saveS3}
          onTest={() => testProvider('s3')}
          isSaving={isSaving.s3}
          testResult={testResults.s3}
        >
          <div className="grid grid-cols-2 gap-4">
            <TextInput
              label="Bucket"
              value={s3.bucket}
              onChange={(bucket) => setS3({ ...s3, bucket })}
              placeholder="my-backup-bucket"
              required
            />
            <TextInput
              label="Region"
              value={s3.region}
              onChange={(region) => setS3({ ...s3, region })}
              placeholder="us-east-1"
              required
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <TextInput
              label="Profile"
              value={s3.profile}
              onChange={(profile) => setS3({ ...s3, profile })}
              placeholder="shadow"
            />
            <TextInput
              label="Prefix"
              value={s3.prefix}
              onChange={(prefix) => setS3({ ...s3, prefix })}
              placeholder="backups/"
            />
          </div>
        </ProviderCard>

        {/* GCS Provider */}
        <ProviderCard
          title="Google Cloud Storage"
          description="Back up to Google Cloud Storage"
          enabled={gcs.enabled}
          onToggle={async (enabled) => {
            const updated = { ...gcs, enabled };
            setGcs(updated);
            await saveGcs(updated);
          }}
          onSave={saveGcs}
          onTest={() => testProvider('gcs')}
          isSaving={isSaving.gcs}
          testResult={testResults.gcs}
        >
          <div className="grid grid-cols-2 gap-4">
            <TextInput
              label="Bucket"
              value={gcs.bucket}
              onChange={(bucket) => setGcs({ ...gcs, bucket })}
              placeholder="my-backup-bucket"
              required
            />
            <TextInput
              label="Project ID"
              value={gcs.project_id}
              onChange={(project_id) => setGcs({ ...gcs, project_id })}
              placeholder="my-project-12345"
              required
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <TextInput
              label="Credentials Path"
              value={gcs.credentials_path}
              onChange={(credentials_path) => setGcs({ ...gcs, credentials_path })}
              placeholder="/path/to/service-account.json"
              required
            />
            <TextInput
              label="Prefix"
              value={gcs.prefix}
              onChange={(prefix) => setGcs({ ...gcs, prefix })}
              placeholder="backups/"
            />
          </div>
        </ProviderCard>

        {/* NAS Provider */}
        <ProviderCard
          title="Network Attached Storage (NAS)"
          description="Back up to a local NAS or mounted drive"
          enabled={nas.enabled}
          onToggle={async (enabled) => {
            const updated = { ...nas, enabled };
            setNas(updated);
            await saveNas(updated);
          }}
          onSave={saveNas}
          onTest={() => testProvider('nas')}
          isSaving={isSaving.nas}
          testResult={testResults.nas}
        >
          <TextInput
            label="Mount Path"
            value={nas.mount_path}
            onChange={(mount_path) => setNas({ ...nas, mount_path })}
            placeholder="/Volumes/BackupDrive"
            required
          />
        </ProviderCard>
      </div>
    </div>
  );
}