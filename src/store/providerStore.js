import { create } from 'zustand';
import { ipc, providerConfig } from '../ipc';
const DEFAULT_S3 = {
    enabled: false,
    bucket: '',
    region: 'us-east-1',
    profile: 'shadow',
    prefix: '',
};
const DEFAULT_GCS = {
    enabled: false,
    bucket: '',
    project_id: '',
    credentials_path: '',
    prefix: '',
};
const DEFAULT_NAS = { enabled: false, mount_path: '' };
export const useProviderStore = create((set, get) => ({
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
    saveS3: async () => {
        set((s) => ({ isSaving: { ...s.isSaving, s3: true } }));
        await providerConfig.saveS3(get().s3);
        set((s) => ({ isSaving: { ...s.isSaving, s3: false } }));
    },
    saveGcs: async () => {
        set((s) => ({ isSaving: { ...s.isSaving, gcs: true } }));
        await providerConfig.saveGcs(get().gcs);
        set((s) => ({ isSaving: { ...s.isSaving, gcs: false } }));
    },
    saveNas: async () => {
        set((s) => ({ isSaving: { ...s.isSaving, nas: true } }));
        await providerConfig.saveNas(get().nas);
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
        }
        catch (e) {
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
