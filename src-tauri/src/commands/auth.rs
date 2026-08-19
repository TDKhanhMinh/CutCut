use keyring::Entry;

#[tauri::command]
pub fn set_secure_token(key: String, value: String) -> Result<(), String> {
    let target = format!("cutcut_app_{}", key);
    let entry = Entry::new(&target, "cutcut_user").map_err(|e| e.to_string())?;
    entry.set_password(&value).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_secure_token(key: String) -> Result<Option<String>, String> {
    let target = format!("cutcut_app_{}", key);
    let entry = Entry::new(&target, "cutcut_user").map_err(|e| e.to_string())?;
    
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn delete_secure_token(key: String) -> Result<(), String> {
    let target = format!("cutcut_app_{}", key);
    let entry = Entry::new(&target, "cutcut_user").map_err(|e| e.to_string())?;
    
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted or doesn't exist
        Err(e) => Err(e.to_string()),
    }
}
