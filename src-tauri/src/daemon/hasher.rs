use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

pub fn open_db() -> Result<Db> {
    let db_path = dirs::data_local_dir()
        .context("cannot find local data dir")?
        .join("shadow")
        .join("hashdb");
    std::fs::create_dir_all(&db_path)?;
    let db = sled::open(&db_path).context("failed to open sled hash DB")?;
    Ok(db)
}

pub enum HashCheckResult {
    Changed(blake3::Hash),
    #[allow(dead_code)]
    Unchanged(u64),
}

pub async fn check_and_hash(db: &Db, path: &Path) -> Result<HashCheckResult> {
    let path_buf = path.to_path_buf();
    let db = db.clone();

    tokio::task::spawn_blocking(move || -> Result<HashCheckResult> {
        let bytes = std::fs::read(&path_buf)
            .with_context(|| format!("failed to read file: {}", path_buf.display()))?;
        let new_hash = blake3::hash(&bytes);

        let key = path_key(&path_buf);
        let stored = db.get(&key)?;

        match stored {
            Some(stored_bytes) if stored_bytes.len() == 40 => {
                let (stored_hash_bytes, mtime_bytes) = stored_bytes.split_at(32);
                if stored_hash_bytes == new_hash.as_bytes() {
                    let mtime = u64::from_le_bytes(mtime_bytes.try_into().unwrap());
                    Ok(HashCheckResult::Unchanged(mtime))
                } else {
                    Ok(HashCheckResult::Changed(new_hash))
                }
            }
            // Fallback for legacy 32-byte entries
            Some(stored_bytes) if stored_bytes.len() == 32 => {
                if stored_bytes.as_ref() == new_hash.as_bytes() {
                    Ok(HashCheckResult::Unchanged(0))
                } else {
                    Ok(HashCheckResult::Changed(new_hash))
                }
            }
            _ => Ok(HashCheckResult::Changed(new_hash)),
        }
    })
    .await?
}

pub fn get_stored_mtime_and_hash(db: &Db, path: &Path) -> Result<Option<(u64, [u8; 32])>> {
    let key = path_key(path);
    if let Some(stored_bytes) = db.get(key)? {
        if stored_bytes.len() == 40 {
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&stored_bytes[0..32]);
            let mtime_bytes: [u8; 8] = stored_bytes[32..40].try_into().unwrap();
            let mtime = u64::from_le_bytes(mtime_bytes);
            Ok(Some((mtime, hash_bytes)))
        } else if stored_bytes.len() == 32 {
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&stored_bytes);
            Ok(Some((0, hash_bytes))) // Legacy fallback
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub fn has_entry(db: &Db, path: &Path) -> Result<bool> {
    let key = path_key(path);
    Ok(db.get(key)?.is_some())
}

pub fn rename_hash_entry(db: &Db, old_path: &Path, new_path: &Path) -> Result<()> {
    let old_key = path_key(old_path);
    if let Some(hash_value) = db.get(&old_key)? {
        let new_key = path_key(new_path);
        db.insert(new_key, hash_value)?;
        db.remove(&old_key)?;
    }
    Ok(())
}

pub fn record_hash(db: &Db, path: &Path, hash: blake3::Hash, mtime_millis: u64) -> Result<()> {
    let key = path_key(path);
    let mut value = Vec::with_capacity(40);
    value.extend_from_slice(hash.as_bytes());
    value.extend_from_slice(&mtime_millis.to_le_bytes());
    db.insert(key, value)?;
    Ok(())
}

fn path_key(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_test_db() -> Db {
        let dir = tempdir().unwrap();
        sled::open(dir.path().join("testdb")).unwrap()
    }

    #[tokio::test]
    async fn changed_for_new_file() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"hello world").unwrap();

        let result = check_and_hash(&db, &file).await.unwrap();
        assert!(matches!(result, HashCheckResult::Changed(_)));
    }

    #[tokio::test]
    async fn unchanged_after_record() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"hello world").unwrap();

        let result = check_and_hash(&db, &file).await.unwrap();
        if let HashCheckResult::Changed(hash) = result {
            record_hash(&db, &file, hash, 100).unwrap();
        }

        let result2 = check_and_hash(&db, &file).await.unwrap();
        match result2 {
            HashCheckResult::Unchanged(mtime) => assert_eq!(mtime, 100),
            _ => panic!("Expected Unchanged"),
        }
    }

    #[test]
    fn has_entry_false_for_unknown_path() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("missing.txt");
        assert!(!has_entry(&db, &file).unwrap());
    }

    #[tokio::test]
    async fn has_entry_true_after_record() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"hello").unwrap();

        if let HashCheckResult::Changed(hash) = check_and_hash(&db, &file).await.unwrap() {
            record_hash(&db, &file, hash, 0).unwrap();
        }

        assert!(has_entry(&db, &file).unwrap());
    }

    #[test]
    fn rename_hash_entry_moves_key() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let old = dir.path().join("old.txt");
        let new = dir.path().join("new.txt");

        let hash = blake3::hash(b"content");
        record_hash(&db, &old, hash, 0).unwrap();
        assert!(has_entry(&db, &old).unwrap());

        rename_hash_entry(&db, &old, &new).unwrap();

        assert!(!has_entry(&db, &old).unwrap(), "old key must be removed");
        assert!(has_entry(&db, &new).unwrap(), "new key must be present");
    }

    #[test]
    fn rename_hash_entry_no_op_for_missing_key() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let old = dir.path().join("ghost.txt");
        let new = dir.path().join("new.txt");

        // Should not error even if old path was never recorded
        rename_hash_entry(&db, &old, &new).unwrap();
        assert!(!has_entry(&db, &new).unwrap());
    }

    #[tokio::test]
    async fn changed_after_content_change() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"version 1").unwrap();

        if let HashCheckResult::Changed(hash) = check_and_hash(&db, &file).await.unwrap() {
            record_hash(&db, &file, hash, 0).unwrap();
        }

        std::fs::write(&file, b"version 2").unwrap();
        let result = check_and_hash(&db, &file).await.unwrap();
        assert!(matches!(result, HashCheckResult::Changed(_)));
    }

    #[tokio::test]
    async fn stored_mtime_mismatch_detected() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"content").unwrap();

        if let HashCheckResult::Changed(hash) = check_and_hash(&db, &file).await.unwrap() {
            record_hash(&db, &file, hash, 100).unwrap();
        }

        // File is unchanged in content, but let's say the scanner sees mtime 200
        let (stored_mtime, _hash) = get_stored_mtime_and_hash(&db, &file).unwrap().unwrap();
        assert_eq!(stored_mtime, 100);
        assert_ne!(stored_mtime, 200);
    }
}
