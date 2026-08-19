use keyring::Entry;
use uuid::Uuid;

const INSTALLATION_KEY: &str = "cutcut_app_installation_id";
const USER_NAMESPACE: &str = "cutcut_user";

#[tauri::command]
pub fn get_or_create_installation_id() -> Result<String, String> {
    let entry = Entry::new(INSTALLATION_KEY, USER_NAMESPACE).map_err(|e| e.to_string())?;

    match entry.get_password() {
        Ok(existing_id) => Ok(existing_id),
        Err(keyring::Error::NoEntry) => {
            // Generate a new random UUID v4
            let new_id = Uuid::new_v4().to_string();
            entry.set_password(&new_id).map_err(|e| e.to_string())?;
            Ok(new_id)
        }
        Err(e) => Err(e.to_string()),
    }
}
