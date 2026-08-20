use crate::models::hardware::RuntimeProfile;
use std::process::Command;
use sysinfo::System;

pub struct HardwareDetectionService;

impl HardwareDetectionService {
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

        let mut gpu_names = Vec::new();
        let mut has_gpu = false;

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
                ])
                .output()
            {
                if output.status.success() {
                    let out_str = String::from_utf8_lossy(&output.stdout);
                    for line in out_str.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            gpu_names.push(trimmed.to_string());
                            has_gpu = true;
                        }
                    }
                }
            }
        }

        let has_nvidia = gpu_names
            .iter()
            .any(|name| name.to_lowercase().contains("nvidia"));

        let (supported_acceleration, fallback_reason) = if has_nvidia {
            ("CUDA".to_string(), None)
        } else if has_avx2 {
            (
                "CPU_AVX2".to_string(),
                Some("No NVIDIA GPU detected. Falling back to CPU AVX2.".to_string()),
            )
        } else {
            (
                "CPU_BASIC".to_string(),
                Some("No GPU or AVX2 detected. Falling back to basic CPU.".to_string()),
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
            fallback_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let profile = HardwareDetectionService::detect_profile();

        // Basic assertions that should pass on any real machine or CI
        assert!(
            profile.cpu_logical_cores > 0,
            "Should have at least 1 logical core"
        );
        assert!(profile.total_memory_mb > 0, "Should have some memory");
        assert!(!profile.cpu_name.is_empty(), "Should have a CPU name");
        assert!(
            !profile.supported_acceleration.is_empty(),
            "Should have a supported acceleration value"
        );
    }
}
