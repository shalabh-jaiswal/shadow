use keyring::Entry;

const SERVICE_NAME: &str = "shadow-backup-gdrive";
const ACCOUNT_NAME: &str = "user-refresh-token";

pub fn save_refresh_token(token: &str) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    entry.set_password(token)?;
    Ok(())
}

pub fn get_refresh_token() -> anyhow::Result<String> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    Ok(entry.get_password()?)
}

pub fn delete_refresh_token() -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    // If the credential doesn't exist, ignore the error or handle it gracefully
    match entry.delete_password() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}
