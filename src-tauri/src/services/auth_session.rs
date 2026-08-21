use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub access_token: String,
    pub expires_at: Option<u64>,
    pub user_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct AuthSessionStore {
    session: Arc<Mutex<Option<AuthSession>>>,
}

impl AuthSessionStore {
    pub async fn set(&self, session: AuthSession) {
        *self.session.lock().await = Some(session);
    }

    pub async fn get(&self) -> Option<AuthSession> {
        self.session.lock().await.clone()
    }

    pub async fn clear(&self) {
        *self.session.lock().await = None;
    }
}
