use serde::Serialize;
use std::sync::{Arc, Mutex};

/// Point-in-time snapshot serialized and sent to the frontend via `get_stats`.
#[derive(Debug, Serialize, Clone)]
pub struct StatsSnapshot {
    pub files_uploaded: u64,
    pub bytes_uploaded: u64,
    pub active_uploads: u64,
    /// Placeholder — proper queue-depth tracking is added in M5.
    pub queue_depth: u64,
}

#[derive(Debug, Default)]
struct Inner {
    files_uploaded: u64,
    bytes_uploaded: u64,
    active_uploads: u64,
}

/// Thread-safe upload counters shared between the queue worker tasks and the
/// `get_stats` IPC command.  Cloning produces a handle to the same data.
#[derive(Clone, Debug, Default)]
pub struct DaemonStats(Arc<Mutex<Inner>>);

impl DaemonStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that one file was successfully uploaded to all providers.
    /// `bytes` is the file size on disk.
    pub fn record_upload(&self, bytes: u64) {
        let mut g = self.0.lock().unwrap();
        g.files_uploaded += 1;
        g.bytes_uploaded += bytes;
    }

    /// Call when a worker task begins processing a file.
    pub fn upload_started(&self) {
        self.0.lock().unwrap().active_uploads += 1;
    }

    /// Call when a worker task finishes (success or permanent failure).
    pub fn upload_finished(&self) {
        let mut g = self.0.lock().unwrap();
        g.active_uploads = g.active_uploads.saturating_sub(1);
    }

    /// Return a serializable snapshot of current counters.
    pub fn snapshot(&self) -> StatsSnapshot {
        let g = self.0.lock().unwrap();
        StatsSnapshot {
            files_uploaded: g.files_uploaded,
            bytes_uploaded: g.bytes_uploaded,
            active_uploads: g.active_uploads,
            queue_depth: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_snapshot_is_zero() {
        let s = DaemonStats::new();
        let snap = s.snapshot();
        assert_eq!(snap.files_uploaded, 0);
        assert_eq!(snap.bytes_uploaded, 0);
        assert_eq!(snap.active_uploads, 0);
    }

    #[test]
    fn record_upload_increments_files_and_bytes() {
        let s = DaemonStats::new();
        s.record_upload(1_024);
        s.record_upload(2_048);
        let snap = s.snapshot();
        assert_eq!(snap.files_uploaded, 2);
        assert_eq!(snap.bytes_uploaded, 3_072);
    }

    #[test]
    fn active_uploads_tracks_start_and_finish() {
        let s = DaemonStats::new();
        s.upload_started();
        s.upload_started();
        assert_eq!(s.snapshot().active_uploads, 2);
        s.upload_finished();
        assert_eq!(s.snapshot().active_uploads, 1);
        s.upload_finished();
        assert_eq!(s.snapshot().active_uploads, 0);
    }

    #[test]
    fn upload_finished_does_not_underflow() {
        let s = DaemonStats::new();
        s.upload_finished(); // no matching start — must not panic
        assert_eq!(s.snapshot().active_uploads, 0);
    }

    #[test]
    fn clone_shares_underlying_state() {
        let s = DaemonStats::new();
        let clone = s.clone();
        s.record_upload(500);
        assert_eq!(clone.snapshot().files_uploaded, 1);
        assert_eq!(clone.snapshot().bytes_uploaded, 500);
    }
}
