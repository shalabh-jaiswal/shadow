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
    Unchanged,
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
            Some(stored_bytes) if stored_bytes.as_ref() == new_hash.as_bytes() => {
                Ok(HashCheckResult::Unchanged)
            }
            _ => Ok(HashCheckResult::Changed(new_hash)),
        }
    })
    .await?
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

pub fn record_hash(db: &Db, path: &Path, hash: blake3::Hash) -> Result<()> {
    let key = path_key(path);
    db.insert(key, hash.as_bytes().to_vec())?;
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
            record_hash(&db, &file, hash).unwrap();
        }

        let result2 = check_and_hash(&db, &file).await.unwrap();
        assert!(matches!(result2, HashCheckResult::Unchanged));
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
            record_hash(&db, &file, hash).unwrap();
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
        record_hash(&db, &old, hash).unwrap();
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
            record_hash(&db, &file, hash).unwrap();
        }

        std::fs::write(&file, b"version 2").unwrap();
        let result = check_and_hash(&db, &file).await.unwrap();
        assert!(matches!(result, HashCheckResult::Changed(_)));
    }
}
