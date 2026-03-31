import { create } from 'zustand';
import type { GcsConfig, NasConfig, S3Config } from '../types';
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

interface ProviderState {
  s3: S3Config;
  gcs: GcsConfig;
  nas: NasConfig;
  testResults: Record<string, TestResult>;
  isSaving: Record<string, boolean>;

  /** Load current provider configs from the daemon. */
  load: () => Promise<void>;

  setS3: (cfg: S3Config) => void;
  setGcs: (cfg: GcsConfig) => void;
  setNas: (cfg: NasConfig) => void;

  saveS3: (cfg?: S3Config) => Promise<void>;
  saveGcs: (cfg?: GcsConfig) => Promise<void>;
  saveNas: (cfg?: NasConfig) => Promise<void>;

  testProvider: (name: 's3' | 'gcs' | 'nas') => Promise<void>;

  /** Update live connection status from provider_status events. */
  setLiveStatus: (provider: string, status: 'ok' | 'error', message?: string) => void;
}

export const useProviderStore = create<ProviderState>((set, get) => ({
  s3: DEFAULT_S3,
  gcs: DEFAULT_GCS,
  nas: DEFAULT_NAS,
  testResults: {},
  isSaving: {},

  load: async () => {
    const config = await ipc.getConfig();
    set({ s3: config.s3, gcs: config.gcs, nas: config.nas });
  },

  setS3: (cfg) => set({ s3: cfg }),
  setGcs: (cfg) => set({ gcs: cfg }),
  setNas: (cfg) => set({ nas: cfg }),

  saveS3: async (cfg?: S3Config) => {
    const config = cfg ?? get().s3;
    set((s) => ({ isSaving: { ...s.isSaving, s3: true } }));
    await providerConfig.saveS3(config);
    set((s) => ({ isSaving: { ...s.isSaving, s3: false } }));
  },

  saveGcs: async (cfg?: GcsConfig) => {
    const config = cfg ?? get().gcs;
    set((s) => ({ isSaving: { ...s.isSaving, gcs: true } }));
    await providerConfig.saveGcs(config);
    set((s) => ({ isSaving: { ...s.isSaving, gcs: false } }));
  },

  saveNas: async (cfg?: NasConfig) => {
    const config = cfg ?? get().nas;
    set((s) => ({ isSaving: { ...s.isSaving, nas: true } }));
    await providerConfig.saveNas(config);
    set((s) => ({ isSaving: { ...s.isSaving, nas: false } }));
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
