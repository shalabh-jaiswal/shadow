import { useEffect } from 'react';
import { events } from '../ipc';
import { useActivityStore } from '../store/activityStore';
function fileEventToActivityEntry(event, status) {
    const filename = event.path.split('/').pop() || event.path.split('\\').pop() || event.path;
    return {
        id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
        timestamp: Date.now(),
        status,
        path: event.path,
        filename,
        provider: event.provider,
        error: event.error,
    };
}
export function useActivityFeed() {
    const addEntry = useActivityStore((s) => s.addEntry);
    useEffect(() => {
        const unlisteners = [
            events.onFileQueued((e) => addEntry(fileEventToActivityEntry(e, 'queued'))),
            events.onFileUploading((e) => addEntry(fileEventToActivityEntry(e, 'uploading'))),
            events.onFileUploaded((e) => addEntry(fileEventToActivityEntry(e, 'uploaded'))),
            events.onFileSkipped((e) => addEntry(fileEventToActivityEntry(e, 'skipped'))),
            events.onFileError((e) => addEntry(fileEventToActivityEntry(e, 'error'))),
            events.onFileFailed((e) => addEntry(fileEventToActivityEntry(e, 'failed'))),
        ];
        return () => {
            unlisteners.forEach((p) => p.then((fn) => fn()));
        };
    }, [addEntry]);
}
