use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceType {
    WhisperModel,
    VadModel,
    RuntimeAsset,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCompatibility {
    pub min_memory_mb: u64,
    pub requires_avx2: bool,
    pub supported_backends: Vec<String>,
    pub runtime_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceItem {
    pub id: String,
    pub resource_type: ResourceType,
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub url: String,
    pub checksum: String,
    pub compatibility: ResourceCompatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResourceState {
    NotInstalled,
    Downloading {
        progress: f64,
        downloaded: u64,
        total: u64,
    },
    Installed,
    Incompatible {
        reason: String,
    },
    Corrupted {
        reason: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceManifest {
    pub id: String,
    pub resource_type: ResourceType,
    pub version: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub compatibility: ResourceCompatibility,
    pub installed_at: u64,
}
