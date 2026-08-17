use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    // Add tracks, state, etc.
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditAction {
    pub id: String,
    pub action_type: String,
}
