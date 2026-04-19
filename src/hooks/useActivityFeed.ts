import { useEffect } from 'react';
import { events } from '../ipc';
import { useActivityStore } from '../store/activityStore';
import type { ActivityEntry, ActivityStatus, FileEvent, FileRenameErrorEvent, FileRenamedEvent } from '../types';

function fileEventToActivityEntry(
  event: FileEvent,
  status: ActivityStatus
): ActivityEntry {
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

function renamedEventToActivityEntry(event: FileRenamedEvent): ActivityEntry {
  const filename = event.new_path.split('/').pop() || event.new_path.split('\\').pop() || event.new_path;
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
    timestamp: Date.now(),
    status: 'renamed',
    path: event.new_path,
    filename,
    provider: event.provider,
    error: null,
    old_path: event.old_path,
    new_path: event.new_path,
  };
}

function renameErrorEventToActivityEntry(event: FileRenameErrorEvent): ActivityEntry {
  const filename = event.new_path.split('/').pop() || event.new_path.split('\\').pop() || event.new_path;
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
    timestamp: Date.now(),
    status: 'rename_error',
    path: event.new_path,
    filename,
    provider: event.provider,
    error: event.error,
    old_path: event.old_path,
    new_path: event.new_path,
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
      events.onFileRenamed((e) => addEntry(renamedEventToActivityEntry(e))),
      events.onFileRenameError((e) => addEntry(renameErrorEventToActivityEntry(e))),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, [addEntry]);
}