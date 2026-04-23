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

pub async fn check_and_hash(
    db: &Db,
    path: &Path,
    providers: &[String],
) -> Result<(blake3::Hash, Vec<String>)> {
    let path_buf = path.to_path_buf();
    let db = db.clone();
    let providers = providers.to_vec();

    tokio::task::spawn_blocking(move || -> Result<(blake3::Hash, Vec<String>)> {
        let bytes = std::fs::read(&path_buf)
            .with_context(|| format!("failed to read file: {}", path_buf.display()))?;
        let new_hash = blake3::hash(&bytes);

        let path_str = path_buf.to_string_lossy();
        let mut missing = Vec::new();

        for provider in providers {
            let key = format!("{}:{}", path_str, provider);
            let stored = db.get(key.as_bytes())?;

            let is_match = match stored {
                Some(stored_bytes) if stored_bytes.len() == 40 => {
                    let (stored_hash_bytes, _mtime_bytes) = stored_bytes.split_at(32);
                    stored_hash_bytes == new_hash.as_bytes()
                }
                Some(stored_bytes) if stored_bytes.len() == 32 => {
                    stored_bytes.as_ref() == new_hash.as_bytes()
                }
                _ => false,
            };

            if !is_match {
                missing.push(provider);
            }
        }
        Ok((new_hash, missing))
    })
    .await?
}

pub fn needs_upload_for_providers(
    db: &Db,
    path: &Path,
    providers: &[&str],
    current_mtime: u64,
) -> Result<bool> {
    let path_str = path.to_string_lossy();
    for provider in providers {
        let key = format!("{}:{}", path_str, provider);
        if let Some(stored_bytes) = db.get(key.as_bytes())? {
            if stored_bytes.len() == 40 {
                let mtime_bytes: [u8; 8] = stored_bytes[32..40].try_into().unwrap();
                let stored_mtime = u64::from_le_bytes(mtime_bytes);
                if stored_mtime != current_mtime || stored_mtime == 0 {
                    return Ok(true);
                }
            } else {
                return Ok(true);
            }
        } else {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn has_any_entry(db: &Db, path: &Path, providers: &[String]) -> Result<bool> {
    let path_str = path.to_string_lossy();
    for provider in providers {
        let key = format!("{}:{}", path_str, provider);
        if db.get(key.as_bytes())?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn rename_hash_entry(db: &Db, old_path: &Path, new_path: &Path, providers: &[String]) -> Result<()> {
    let old_str = old_path.to_string_lossy();
    let new_str = new_path.to_string_lossy();
    for provider in providers {
        let old_key = format!("{}:{}", old_str, provider);
        if let Some(hash_value) = db.get(old_key.as_bytes())? {
            let new_key = format!("{}:{}", new_str, provider);
            db.insert(new_key.as_bytes(), hash_value)?;
            db.remove(old_key.as_bytes())?;
        }
    }
    Ok(())
}

pub fn record_hash(
    db: &Db,
    path: &Path,
    provider: &str,
    hash: blake3::Hash,
    mtime_millis: u64,
) -> Result<()> {
    let key = format!("{}:{}", path.to_string_lossy(), provider);
    let mut value = Vec::with_capacity(40);
    value.extend_from_slice(hash.as_bytes());
    value.extend_from_slice(&mtime_millis.to_le_bytes());
    db.insert(key.as_bytes(), value)?;
    Ok(())
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

        let providers = vec!["s3".to_string()];
        let (_hash, missing) = check_and_hash(&db, &file, &providers).await.unwrap();
        assert_eq!(missing, vec!["s3"]);
    }

    #[tokio::test]
    async fn unchanged_after_record() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"hello world").unwrap();

        let providers = vec!["s3".to_string()];
        let (hash, missing) = check_and_hash(&db, &file, &providers).await.unwrap();
        assert_eq!(missing.len(), 1);
        
        record_hash(&db, &file, "s3", hash, 100).unwrap();

        let (_hash, missing2) = check_and_hash(&db, &file, &providers).await.unwrap();
        assert!(missing2.is_empty());
    }

    #[test]
    fn needs_upload_false_for_unknown_path() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("missing.txt");
        assert!(needs_upload_for_providers(&db, &file, &["s3"], 100).unwrap());
    }

    #[tokio::test]
    async fn needs_upload_true_after_record() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"hello").unwrap();

        let (hash, _) = check_and_hash(&db, &file, &["s3".to_string()]).await.unwrap();
        record_hash(&db, &file, "s3", hash, 0).unwrap();

        assert!(needs_upload_for_providers(&db, &file, &["s3"], 100).unwrap());
    }

    #[test]
    fn rename_hash_entry_moves_key() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let old = dir.path().join("old.txt");
        let new = dir.path().join("new.txt");

        let hash = blake3::hash(b"content");
        record_hash(&db, &old, "s3", hash, 0).unwrap();
        assert!(has_any_entry(&db, &old, &["s3".to_string()]).unwrap());

        rename_hash_entry(&db, &old, &new, &["s3".to_string()]).unwrap();

        assert!(!has_any_entry(&db, &old, &["s3".to_string()]).unwrap(), "old key must be removed");
        assert!(has_any_entry(&db, &new, &["s3".to_string()]).unwrap(), "new key must be present");
    }

    #[test]
    fn rename_hash_entry_no_op_for_missing_key() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let old = dir.path().join("ghost.txt");
        let new = dir.path().join("new.txt");

        rename_hash_entry(&db, &old, &new, &["s3".to_string()]).unwrap();
        assert!(!has_any_entry(&db, &new, &["s3".to_string()]).unwrap());
    }

    #[tokio::test]
    async fn changed_after_content_change() {
        let db = open_test_db();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"version 1").unwrap();

        let providers = vec!["s3".to_string()];
        let (hash, _) = check_and_hash(&db, &file, &providers).await.unwrap();
        record_hash(&db, &file, "s3", hash, 0).unwrap();

        std::fs::write(&file, b"version 2").unwrap();
        let (_hash, missing) = check_and_hash(&db, &file, &providers).await.unwrap();
        assert_eq!(missing, vec!["s3"]);
    }
}
