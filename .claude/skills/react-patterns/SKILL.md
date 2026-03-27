---
name: react-patterns
description: |
  Shadow-specific React and TypeScript patterns. Activates when writing
  frontend components, Zustand stores, custom hooks, or TypeScript types.
  Covers: component structure, Zustand store patterns, activity feed with
  circular buffer, status badges, Tailwind conventions, type definitions.
allowed-tools:
  - Read
---

# React Patterns for Shadow

## Zustand Store Pattern

```typescript
// src/store/activityStore.ts
import { create } from 'zustand';
import type { LogEntry } from '../types';

const MAX_ENTRIES = 200;

interface ActivityState {
  entries: LogEntry[];
  addEntry: (entry: LogEntry) => void;
  clearEntries: () => void;
}

export const useActivityStore = create<ActivityState>((set) => ({
  entries: [],

  addEntry: (entry) => set((state) => {
    // Circular buffer — keep newest 200 entries
    const updated = [entry, ...state.entries];
    return { entries: updated.slice(0, MAX_ENTRIES) };
  }),

  clearEntries: () => set({ entries: [] }),
}));
```

```typescript
// src/store/foldersStore.ts
import { create } from 'zustand';
import type { FolderStatus } from '../types';
import { ipc } from '../ipc';

interface FoldersState {
  folders: FolderStatus[];
  isLoading: boolean;
  fetchFolders: () => Promise<void>;
  addFolder: (path: string) => Promise<void>;
  removeFolder: (path: string) => Promise<void>;
}

export const useFoldersStore = create<FoldersState>((set) => ({
  folders: [],
  isLoading: false,

  fetchFolders: async () => {
    set({ isLoading: true });
    const folders = await ipc.getWatchedFolders();
    set({ folders, isLoading: false });
  },

  addFolder: async (path) => {
    await ipc.addFolder(path);
    // Refresh from daemon as source of truth
    const folders = await ipc.getWatchedFolders();
    set({ folders });
  },

  removeFolder: async (path) => {
    await ipc.removeFolder(path);
    set((state) => ({
      folders: state.folders.filter(f => f.path !== path)
    }));
  },
}));
```

## Status Badge Component

```tsx
// src/components/shared/StatusBadge.tsx
interface StatusBadgeProps {
  status: 'scanning' | 'active' | 'error' | 'paused';
}

const statusConfig = {
  scanning: { dot: 'bg-blue-400 animate-pulse', text: 'Scanning' },
  active:   { dot: 'bg-green-400',              text: 'Active'   },
  error:    { dot: 'bg-red-400',                text: 'Error'    },
  paused:   { dot: 'bg-gray-400',               text: 'Paused'   },
} as const;

export function StatusBadge({ status }: StatusBadgeProps) {
  const { dot, text } = statusConfig[status];
  return (
    <span className="inline-flex items-center gap-1.5 text-sm">
      <span className={`w-2 h-2 rounded-full ${dot}`} />
      <span className="text-gray-600 dark:text-gray-400">{text}</span>
    </span>
  );
}
```

## Confirm Modal Pattern

```tsx
// src/components/shared/ConfirmModal.tsx
interface ConfirmModalProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
  danger?: boolean;
}

export function ConfirmModal({
  isOpen, title, message, confirmLabel = 'Confirm',
  onConfirm, onCancel, danger = false
}: ConfirmModalProps) {
  if (!isOpen) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full shadow-xl">
        <h2 className="text-lg font-semibold mb-2">{title}</h2>
        <p className="text-gray-600 dark:text-gray-400 mb-6">{message}</p>
        <div className="flex justify-end gap-3">
          <button
            onClick={onCancel}
            className="px-4 py-2 rounded-md border border-gray-300 hover:bg-gray-50"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className={`px-4 py-2 rounded-md text-white ${
              danger ? 'bg-red-600 hover:bg-red-700' : 'bg-blue-600 hover:bg-blue-700'
            }`}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
```

## Scrolling Activity Feed (auto-scroll unless user scrolled up)

```tsx
import { useRef, useEffect, useState } from 'react';

export function ActivityFeed({ entries }: { entries: LogEntry[] }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [userScrolled, setUserScrolled] = useState(false);

  useEffect(() => {
    if (!userScrolled && containerRef.current) {
      containerRef.current.scrollTop = 0; // newest is at top
    }
  }, [entries, userScrolled]);

  const handleScroll = () => {
    const el = containerRef.current;
    if (el) setUserScrolled(el.scrollTop > 20);
  };

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      className="overflow-y-auto h-full"
    >
      {entries.map(entry => <FeedRow key={entry.id} entry={entry} />)}
    </div>
  );
}
```

## Provider Card Pattern (Providers screen)

```tsx
interface ProviderCardProps {
  title: string;
  enabled: boolean;
  onToggle: (v: boolean) => void;
  onTest: () => Promise<void>;
  children: React.ReactNode; // config fields
}

export function ProviderCard({ title, enabled, onToggle, onTest, children }: ProviderCardProps) {
  const [testStatus, setTestStatus] = useState<'idle'|'testing'|'ok'|'error'>('idle');
  const [testMessage, setTestMessage] = useState('');

  const handleTest = async () => {
    setTestStatus('testing');
    try {
      const msg = await onTest();
      setTestStatus('ok');
      setTestMessage(msg);
    } catch (e) {
      setTestStatus('error');
      setTestMessage(String(e));
    }
  };

  return (
    <div className="rounded-lg border border-gray-200 dark:border-gray-700 p-5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="font-semibold text-lg">{title}</h3>
        <Toggle checked={enabled} onChange={onToggle} />
      </div>
      <div className={enabled ? '' : 'opacity-40 pointer-events-none'}>
        {children}
        <button onClick={handleTest} disabled={testStatus === 'testing'}
          className="mt-4 px-3 py-1.5 text-sm rounded border hover:bg-gray-50">
          {testStatus === 'testing' ? 'Testing…' : 'Test Connection'}
        </button>
        {testStatus === 'ok' && <span className="ml-2 text-green-600 text-sm">✓ Connected</span>}
        {testStatus === 'error' && <span className="ml-2 text-red-600 text-sm">✗ {testMessage}</span>}
      </div>
    </div>
  );
}
```
