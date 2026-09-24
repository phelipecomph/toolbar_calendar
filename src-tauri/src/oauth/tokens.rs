use keyring::Entry;

// Refresh tokens live in the Windows Credential Manager, never in a file.
const SERVICE: &str = "agenda-strip";

pub fn store_refresh(account_id: &str, token: &str) -> Result<(), String> {
    Entry::new(SERVICE, account_id)
        .map_err(|e| e.to_string())?
        .set_password(token)
        .map_err(|e| e.to_string())
}

pub fn load_refresh(account_id: &str) -> Result<String, String> {
    Entry::new(SERVICE, account_id)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn delete_refresh(account_id: &str) -> Result<(), String> {
    Entry::new(SERVICE, account_id)
        .map_err(|e| e.to_string())?
        .delete_credential()
        .map_err(|e| e.to_string())
}
