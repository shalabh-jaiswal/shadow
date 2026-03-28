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
