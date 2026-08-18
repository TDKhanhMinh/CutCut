use crate::models::hardware::RuntimeProfile;
use crate::models::preset::{PresetResolution, PresetType};

pub struct RuntimePresetService;

impl RuntimePresetService {
    pub fn resolve_preset(
        preset: PresetType,
        profile: &RuntimeProfile,
        installed_model_ids: &[String],
        user_override_model: Option<String>,
    ) -> PresetResolution {
        let target_backend = profile.supported_acceleration.clone();

        if let Some(user_override) = user_override_model {
            let is_installed = installed_model_ids.contains(&user_override);
            return PresetResolution {
                preset: PresetType::Custom,
                target_model_id: user_override.clone(),
                target_backend,
                is_model_installed: is_installed,
                fallback_reason: if !is_installed { Some(format!("Model {} is not installed.", user_override)) } else { None },
                tradeoff_description: "Custom settings overridden by user.".to_string(),
            };
        }

        let ideal_model = match preset {
            PresetType::Fast => "ggml-tiny",
            PresetType::Balanced => "ggml-small",
            PresetType::Accurate => "ggml-medium",
            PresetType::Custom => "ggml-base",
        };

        let tradeoff_description = match preset {
            PresetType::Fast => "Fastest speed, low memory, lower accuracy".to_string(),
            PresetType::Balanced => "Balanced speed and accuracy".to_string(),
            PresetType::Accurate => "High accuracy, requires more RAM and time".to_string(),
            PresetType::Custom => "Custom settings".to_string(),
        };

        let mut target_model_id = ideal_model.to_string();
        let mut is_model_installed = installed_model_ids.contains(&target_model_id);
        let mut fallback_reason = None;

        if !is_model_installed {
            let fallback_candidates = vec!["ggml-tiny", "ggml-base", "ggml-small", "ggml-medium", "ggml-large-v3"];
            let mut found_fallback = false;
            for candidate in fallback_candidates {
                if installed_model_ids.contains(&candidate.to_string()) {
                    target_model_id = candidate.to_string();
                    is_model_installed = true;
                    fallback_reason = Some(format!("Ideal model {} is not installed, falling back to {}.", ideal_model, candidate));
                    found_fallback = true;
                    break;
                }
            }
            if !found_fallback {
                fallback_reason = Some(format!("Ideal model {} is not installed and no fallback available.", ideal_model));
            }
        }

        PresetResolution {
            preset,
            target_model_id,
            target_backend,
            is_model_installed,
            fallback_reason,
            tradeoff_description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_preset_with_installed_ideal() {
        let profile = RuntimeProfile {
            cpu_name: "Intel".to_string(),
            cpu_logical_cores: 8,
            total_memory_mb: 16000,
            has_avx2: true,
            has_avx512: false,
            has_gpu: false,
            gpu_names: vec![],
            supported_acceleration: "CPU_AVX2".to_string(),
            fallback_reason: None,
        };

        let installed = vec!["ggml-tiny".to_string(), "ggml-small".to_string()];
        
        let res = RuntimePresetService::resolve_preset(PresetType::Balanced, &profile, &installed, None);
        
        assert_eq!(res.preset, PresetType::Balanced);
        assert_eq!(res.target_model_id, "ggml-small");
        assert_eq!(res.target_backend, "CPU_AVX2");
        assert!(res.is_model_installed);
        assert_eq!(res.fallback_reason, None);
    }

    #[test]
    fn test_resolve_preset_with_fallback() {
        let profile = RuntimeProfile {
            cpu_name: "Intel".to_string(),
            cpu_logical_cores: 8,
            total_memory_mb: 16000,
            has_avx2: true,
            has_avx512: false,
            has_gpu: false,
            gpu_names: vec![],
            supported_acceleration: "CPU_AVX2".to_string(),
            fallback_reason: None,
        };

        // Only tiny is installed
        let installed = vec!["ggml-tiny".to_string()];
        
        let res = RuntimePresetService::resolve_preset(PresetType::Accurate, &profile, &installed, None);
        
        assert_eq!(res.preset, PresetType::Accurate);
        // It should fallback to tiny since medium is not installed
        assert_eq!(res.target_model_id, "ggml-tiny");
        assert!(res.is_model_installed);
        assert!(res.fallback_reason.unwrap().contains("falling back to ggml-tiny"));
    }
}
