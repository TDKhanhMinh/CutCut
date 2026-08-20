use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    WhisperModel,
    VadModel,
    RuntimeAsset,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceItem {
    pub id: String,
    pub resource_type: ResourceType,
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub url: String,
    pub checksum: String,
    pub compatibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceState {
    NotInstalled,
    Downloading {
        progress: f64,
        downloaded: u64,
        total: u64,
    },
    Installed,
    Corrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManifest {
    pub id: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub installed_at: u64,
}
