use crate::models::hardware::RuntimeProfile;
use crate::models::whisper::WhisperRuntimeInfo;
use crate::services::whisper_service::WhisperService;
use std::process::Command;
use std::sync::Arc;
use sysinfo::System;
use tauri::AppHandle;
use tokio::sync::RwLock;

const CPU_BASIC: &str = "CPU_BASIC";
const CPU_AVX2: &str = "CPU_AVX2";

#[derive(Default)]
pub struct RuntimeProfileCache {
    profile: Arc<RwLock<Option<RuntimeProfile>>>,
}

impl RuntimeProfileCache {
    pub async fn get_or_detect(&self, app: &AppHandle, refresh: bool) -> RuntimeProfile {
        if !refresh {
            if let Some(profile) = self.profile.read().await.clone() {
                return profile;
            }
        }

        let hardware = HardwareDetectionService::detect_profile();
        let runtime = WhisperService::check_runtime(app).await;
        let profile = HardwareDetectionService::with_runtime_probe(hardware, runtime);
        *self.profile.write().await = Some(profile.clone());
        profile
    }
}

pub struct HardwareDetectionService;

impl HardwareDetectionService {
    /// Detect host hardware and apply the packaged CPU-first runtime contract.
    /// This synchronous path is used by resource validation, so it must never
    /// infer CUDA/GPU support from a device name alone.
    pub fn detect_profile() -> RuntimeProfile {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_name = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());
        let cpu_logical_cores = sys.cpus().len() as u32;
        let total_memory_mb = sys.total_memory() / 1024 / 1024;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let has_avx2 = std::is_x86_feature_detected!("avx2");
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        let has_avx2 = false;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let has_avx512 = std::is_x86_feature_detected!("avx512f");
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        let has_avx512 = false;

        let (gpu_names, gpu_probe_warning) = detect_gpu_names();
        let has_gpu = !gpu_names.is_empty();
        let runtime_backends = vec![CPU_BASIC.to_string(), CPU_AVX2.to_string()];
        let supported_acceleration = select_cpu_backend(has_avx2);
        let fallback_reason = if has_gpu {
            Some(
                "GPU detected, but the packaged whisper runtime exposes CPU backends only; using the safe CPU path."
                    .to_string(),
            )
        } else if let Some(warning) = gpu_probe_warning {
            Some(format!("{warning}; using the packaged CPU path."))
        } else if supported_acceleration == CPU_BASIC {
            Some("AVX2 is unavailable; using the basic CPU path.".to_string())
        } else {
            Some(
                "The packaged whisper runtime is CPU-first; no GPU backend is advertised."
                    .to_string(),
            )
        };

        RuntimeProfile {
            cpu_name,
            cpu_logical_cores,
            total_memory_mb,
            has_avx2,
            has_avx512,
            has_gpu,
            gpu_names,
            supported_acceleration,
            runtime_available: true,
            runtime_version: None,
            runtime_backends,
            recommended_model_ids: recommended_model_ids(total_memory_mb),
            fallback_reason,
        }
    }

    pub fn with_runtime_probe(
        mut profile: RuntimeProfile,
        runtime: Result<WhisperRuntimeInfo, crate::services::whisper_service::WhisperError>,
    ) -> RuntimeProfile {
        match runtime {
            Ok(info) if info.available => {
                profile.runtime_available = true;
                profile.runtime_version = Some(info.version);
                profile.runtime_backends = runtime_backends(&info.backend);
                profile.supported_acceleration =
                    select_available_backend(profile.has_avx2, &profile.runtime_backends);
                if profile.supported_acceleration == CPU_BASIC
                    && !profile
                        .runtime_backends
                        .iter()
                        .any(|backend| backend == CPU_BASIC)
                {
                    profile.fallback_reason = Some(
                        "Packaged runtime probe did not expose a supported CPU backend."
                            .to_string(),
                    );
                } else if profile.has_gpu {
                    profile.fallback_reason = Some(
                        "GPU detected, but runtime probe did not expose a GPU backend; using CPU."
                            .to_string(),
                    );
                }
            }
            Ok(_) => {
                profile.runtime_available = false;
                profile.runtime_version = None;
                profile.runtime_backends.clear();
                profile.supported_acceleration = CPU_BASIC.to_string();
                profile.fallback_reason = Some(
                    "Whisper runtime reported unavailable; repair the packaged runtime before starting STT."
                        .to_string(),
                );
            }
            Err(error) => {
                profile.runtime_available = false;
                profile.runtime_version = None;
                profile.runtime_backends.clear();
                profile.supported_acceleration = CPU_BASIC.to_string();
                profile.fallback_reason = Some(format!(
                    "Whisper runtime probe failed; CPU fallback is selected but STT is unavailable until repair: {error}"
                ));
            }
        }
        profile
    }
}

fn detect_gpu_names() -> (Vec<String>, Option<String>) {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ])
            .output();
        match output {
            Ok(output) if output.status.success() => (
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect(),
                None,
            ),
            Ok(output) => (
                Vec::new(),
                Some(format!("GPU inventory probe exited with {}", output.status)),
            ),
            Err(error) => (
                Vec::new(),
                Some(format!("GPU inventory probe failed: {error}")),
            ),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        (Vec::new(), None)
    }
}

fn runtime_backends(backend: &str) -> Vec<String> {
    if backend.eq_ignore_ascii_case("cpu") {
        vec![CPU_BASIC.to_string(), CPU_AVX2.to_string()]
    } else if backend.trim().is_empty() {
        Vec::new()
    } else {
        vec![backend.to_ascii_uppercase()]
    }
}

fn select_cpu_backend(has_avx2: bool) -> String {
    if has_avx2 {
        CPU_AVX2.to_string()
    } else {
        CPU_BASIC.to_string()
    }
}

fn select_available_backend(has_avx2: bool, backends: &[String]) -> String {
    if has_avx2 && backends.iter().any(|backend| backend == CPU_AVX2) {
        CPU_AVX2.to_string()
    } else {
        CPU_BASIC.to_string()
    }
}

fn recommended_model_ids(total_memory_mb: u64) -> Vec<String> {
    let mut ids = vec!["ggml-tiny".to_string()];
    if total_memory_mb >= 4_000 {
        ids.push("ggml-base".to_string());
    }
    if total_memory_mb >= 8_000 {
        ids.push("ggml-small".to_string());
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline_profile() -> RuntimeProfile {
        RuntimeProfile {
            cpu_name: "test".to_string(),
            cpu_logical_cores: 8,
            total_memory_mb: 16_000,
            has_avx2: true,
            has_avx512: false,
            has_gpu: true,
            gpu_names: vec!["NVIDIA Test GPU".to_string()],
            supported_acceleration: CPU_AVX2.to_string(),
            runtime_available: true,
            runtime_version: None,
            runtime_backends: vec![CPU_BASIC.to_string(), CPU_AVX2.to_string()],
            recommended_model_ids: recommended_model_ids(16_000),
            fallback_reason: None,
        }
    }

    #[test]
    fn gpu_name_never_promotes_cpu_bundle_to_cuda() {
        let profile = baseline_profile();
        assert_eq!(profile.supported_acceleration, CPU_AVX2);
        assert!(profile.runtime_backends.contains(&CPU_AVX2.to_string()));
    }

    #[test]
    fn runtime_probe_failure_is_safe_and_actionable() {
        let profile = HardwareDetectionService::with_runtime_probe(
            baseline_profile(),
            Err(
                crate::services::whisper_service::WhisperError::SidecarUnavailable(
                    "test failure".to_string(),
                ),
            ),
        );
        assert!(!profile.runtime_available);
        assert_eq!(profile.supported_acceleration, CPU_BASIC);
        assert!(profile
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("test failure")));
    }

    #[test]
    fn successful_cpu_probe_records_version_and_backends() {
        let profile = HardwareDetectionService::with_runtime_probe(
            baseline_profile(),
            Ok(WhisperRuntimeInfo {
                available: true,
                version: "whisper.cpp 1.9.2".to_string(),
                backend: "cpu".to_string(),
            }),
        );
        assert_eq!(
            profile.runtime_version.as_deref(),
            Some("whisper.cpp 1.9.2")
        );
        assert_eq!(profile.supported_acceleration, CPU_AVX2);
        assert_eq!(
            profile.runtime_backends,
            vec![CPU_BASIC.to_string(), CPU_AVX2.to_string()]
        );
        assert!(profile
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("GPU detected")));
    }

    #[test]
    fn model_recommendations_follow_memory_policy() {
        assert_eq!(recommended_model_ids(2_000), vec!["ggml-tiny"]);
        assert_eq!(recommended_model_ids(4_000), vec!["ggml-tiny", "ggml-base"]);
        assert_eq!(
            recommended_model_ids(8_000),
            vec!["ggml-tiny", "ggml-base", "ggml-small"]
        );
    }

    #[test]
    fn hardware_detection_returns_a_safe_profile() {
        let profile = HardwareDetectionService::detect_profile();
        assert!(profile.cpu_logical_cores > 0);
        assert!(profile.total_memory_mb > 0);
        assert!(profile.runtime_available);
        assert!(profile
            .runtime_backends
            .iter()
            .all(|backend| backend == CPU_BASIC || backend == CPU_AVX2));
        assert!(!profile.recommended_model_ids.is_empty());
    }
}
