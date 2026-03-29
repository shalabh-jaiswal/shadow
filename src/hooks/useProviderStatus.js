import { useEffect } from 'react';
import { events } from '../ipc';
import { useProviderStore } from '../store/providerStore';
export function useProviderStatus() {
    const setLiveStatus = useProviderStore((s) => s.setLiveStatus);
    useEffect(() => {
        const unlisten = events.onProviderStatus((e) => {
            setLiveStatus(e.provider, e.status, e.error ?? '');
        });
        return () => {
            unlisten.then((fn) => fn());
        };
    }, [setLiveStatus]);
}
