import { create } from 'zustand';
import type { GcsConfig, GdriveConfig, NasConfig, S3Config } from '../types';
import { ipc, providerConfig } from '../ipc';

export type TestStatus = 'idle' | 'testing' | 'ok' | 'error';

export interface TestResult {
  status: TestStatus;
  message: string;
}

const DEFAULT_S3: S3Config = {
  enabled: false,
  bucket: '',
  region: 'us-east-1',
  profile: 'shadow',
  prefix: '',
};
const DEFAULT_GCS: GcsConfig = {
  enabled: false,
  bucket: '',
  project_id: '',
  credentials_path: '',
  prefix: '',
};
const DEFAULT_NAS: NasConfig = { enabled: false, mount_path: '' };
const DEFAULT_GDRIVE: GdriveConfig = { enabled: false, root_folder_id: '', prefix: '' };

interface ProviderState {
  s3: S3Config;
  gcs: GcsConfig;
  nas: NasConfig;
  gdrive: GdriveConfig;
  testResults: Record<string, TestResult>;
  isSaving: Record<string, boolean>;

  /** Load current provider configs from the daemon. */
  load: () => Promise<void>;

  setS3: (cfg: S3Config) => void;
  setGcs: (cfg: GcsConfig) => void;
  setNas: (cfg: NasConfig) => void;
  setGdrive: (cfg: GdriveConfig) => void;

  saveS3: (cfg?: S3Config) => Promise<void>;
  saveGcs: (cfg?: GcsConfig) => Promise<void>;
  saveNas: (cfg?: NasConfig) => Promise<void>;
  saveGdrive: (cfg?: GdriveConfig) => Promise<void>;

  connectGdrive: () => Promise<void>;
  disconnectGdrive: () => Promise<void>;

  testProvider: (name: 's3' | 'gcs' | 'nas' | 'gdrive') => Promise<void>;

  /** Update live connection status from provider_status events. */
  setLiveStatus: (provider: string, status: 'ok' | 'error', message?: string) => void;
}

export const useProviderStore = create<ProviderState>((set, get) => ({
  s3: DEFAULT_S3,
  gcs: DEFAULT_GCS,
  nas: DEFAULT_NAS,
  gdrive: DEFAULT_GDRIVE,
  testResults: {},
  isSaving: {},

  load: async () => {
    const config = await ipc.getConfig();
    set({ s3: config.s3, gcs: config.gcs, nas: config.nas, gdrive: config.gdrive });
  },

  setS3: (cfg) => set({ s3: cfg }),
  setGcs: (cfg) => set({ gcs: cfg }),
  setNas: (cfg) => set({ nas: cfg }),
  setGdrive: (cfg) => set({ gdrive: cfg }),

  saveS3: async (cfg?: S3Config) => {
    const config = cfg ?? get().s3;
    set((s) => ({ isSaving: { ...s.isSaving, s3: true } }));
    try {
      await providerConfig.saveS3(config);
    } finally {
      set((s) => ({ isSaving: { ...s.isSaving, s3: false } }));
    }
  },

  saveGcs: async (cfg?: GcsConfig) => {
    const config = cfg ?? get().gcs;
    set((s) => ({ isSaving: { ...s.isSaving, gcs: true } }));
    try {
      await providerConfig.saveGcs(config);
    } finally {
      set((s) => ({ isSaving: { ...s.isSaving, gcs: false } }));
    }
  },

  saveNas: async (cfg?: NasConfig) => {
    const config = cfg ?? get().nas;
    set((s) => ({ isSaving: { ...s.isSaving, nas: true } }));
    try {
      await providerConfig.saveNas(config);
    } finally {
      set((s) => ({ isSaving: { ...s.isSaving, nas: false } }));
    }
  },

  saveGdrive: async (cfg?: GdriveConfig) => {
    const config = cfg ?? get().gdrive;
    set((s) => ({ isSaving: { ...s.isSaving, gdrive: true } }));
    try {
      await providerConfig.saveGdrive(config);
    } finally {
      set((s) => ({ isSaving: { ...s.isSaving, gdrive: false } }));
    }
  },

  connectGdrive: async () => {
    set((s) => ({ isSaving: { ...s.isSaving, gdrive: true } }));
    try {
      await ipc.startGdriveAuth();
      const config = await ipc.getConfig();
      set({ gdrive: config.gdrive });
    } catch (e) {
      set((s) => ({ isSaving: { ...s.isSaving, gdrive: false } }));
      throw e; // re-throw so the UI can display the error
    } finally {
      set((s) => ({ isSaving: { ...s.isSaving, gdrive: false } }));
    }
  },

  disconnectGdrive: async () => {
    set((s) => ({ isSaving: { ...s.isSaving, gdrive: true } }));
    try {
      await ipc.disconnectGdrive();
      const config = await ipc.getConfig();
      set({ gdrive: config.gdrive });
    } finally {
      set((s) => ({ isSaving: { ...s.isSaving, gdrive: false } }));
    }
  },

  testProvider: async (name) => {
    set((s) => ({
      testResults: { ...s.testResults, [name]: { status: 'testing', message: '' } },
    }));
    try {
      const msg = await ipc.testProvider(name);
      set((s) => ({
        testResults: { ...s.testResults, [name]: { status: 'ok', message: msg } },
      }));
    } catch (e) {
      set((s) => ({
        testResults: {
          ...s.testResults,
          [name]: { status: 'error', message: String(e) },
        },
      }));
    }
  },

  setLiveStatus: (provider, status, message = '') => {
    set((s) => ({
      testResults: {
        ...s.testResults,
        [provider]: { status, message },
      },
    }));
  },
}));
