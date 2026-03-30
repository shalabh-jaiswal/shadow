import { useEffect, useState } from 'react';
import { events } from '../ipc';

export function useReconcileStatus() {
  const [isReconciling, setIsReconciling] = useState(false);
  const [lastReconcile, setLastReconcile] = useState<{ filesQueued: number; timestamp: number } | null>(null);

  useEffect(() => {
    const unlistenStarted = events.onReconcileStarted(() => {
      setIsReconciling(true);
    });

    const unlistenComplete = events.onReconcileComplete((payload) => {
      setIsReconciling(false);
      setLastReconcile({
        filesQueued: payload.files_queued,
        timestamp: Date.now(),
      });
    });

    return () => {
      unlistenStarted.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, []);

  return { isReconciling, lastReconcile };
}