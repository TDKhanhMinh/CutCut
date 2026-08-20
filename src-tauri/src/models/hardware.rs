use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfile {
    pub cpu_name: String,
    pub cpu_logical_cores: u32,
    pub total_memory_mb: u64,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub has_gpu: bool,
    pub gpu_names: Vec<String>,
    pub supported_acceleration: String, // "CPU_AVX2" or "CPU_BASIC" for the V1 CPU bundle
    pub runtime_available: bool,
    pub runtime_version: Option<String>,
    pub runtime_backends: Vec<String>,
    pub recommended_model_ids: Vec<String>,
    pub fallback_reason: Option<String>,
}
