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
    pub supported_acceleration: String, // "CUDA", "CPU_AVX2", "CPU_BASIC"
    pub fallback_reason: Option<String>,
}
