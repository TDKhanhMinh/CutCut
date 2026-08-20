use crate::models::hardware::RuntimeProfile;
use crate::models::preset::{PresetResolution, PresetType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const PRESET_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy)]
struct PresetSpec {
    model_id: &'static str,
    tradeoff_description: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePresetPreference {
    pub schema_version: u32,
    pub preset: PresetType,
    pub user_override_model: Option<String>,
}

impl Default for RuntimePresetPreference {
    fn default() -> Self {
        Self {
            schema_version: PRESET_CONFIG_VERSION,
            preset: PresetType::Balanced,
            user_override_model: None,
        }
    }
}

pub struct RuntimePresetService;

impl RuntimePresetService {
    pub fn resolve_preset(
        preset: PresetType,
        profile: &RuntimeProfile,
        installed_model_ids: &[String],
        user_override_model: Option<String>,
    ) -> PresetResolution {
        let target_backend = profile.supported_acceleration.clone();
        let ideal_model = preset_spec(preset).model_id;
        let tradeoff_description = preset_spec(preset).tradeoff_description.to_string();

        if !profile.runtime_available || !runtime_backend_available(profile) {
            let target_model_id = user_override_model.unwrap_or_else(|| ideal_model.to_string());
            return PresetResolution {
                preset,
                target_model_id,
                target_backend,
                is_model_installed: false,
                fallback_reason: Some(
                    "Whisper runtime/backend is unavailable; repair the packaged runtime before running STT."
                        .to_string(),
                ),
                tradeoff_description,
            };
        }

        let eligible_models = |id: &str| {
            installed_model_ids.iter().any(|installed| installed == id)
                && (profile.recommended_model_ids.is_empty()
                    || profile
                        .recommended_model_ids
                        .iter()
                        .any(|recommended| recommended == id))
        };

        if let Some(user_override) = user_override_model {
            let is_eligible = eligible_models(&user_override);
            return PresetResolution {
                preset: PresetType::Custom,
                target_model_id: user_override.clone(),
                target_backend,
                is_model_installed: is_eligible,
                fallback_reason: if is_eligible {
                    None
                } else {
                    Some(format!(
                        "Model {user_override} is not installed or is incompatible with this runtime."
                    ))
                },
                tradeoff_description:
                    "Custom model override; compatibility is rechecked at runtime.".to_string(),
            };
        }

        let mut target_model_id = ideal_model.to_string();
        let mut is_model_installed = eligible_models(ideal_model);
        let mut fallback_reason = None;

        if !is_model_installed {
            for candidate in ["ggml-tiny", "ggml-base", "ggml-small"] {
                if eligible_models(candidate) {
                    target_model_id = candidate.to_string();
                    is_model_installed = true;
                    fallback_reason = Some(format!(
                        "Ideal model {ideal_model} is not installed or compatible; falling back to {candidate}."
                    ));
                    break;
                }
            }
            if !is_model_installed {
                fallback_reason = Some(format!(
                    "Ideal model {ideal_model} is not installed or compatible and no safe fallback is available."
                ));
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

    pub fn load_preference(app: &AppHandle) -> Result<RuntimePresetPreference, String> {
        let path = preference_path(app)?;
        if !path.is_file() {
            return Ok(RuntimePresetPreference::default());
        }
        let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let preference = serde_json::from_str::<RuntimePresetPreference>(&content)
            .map_err(|error| format!("Invalid runtime preset preference: {error}"))?;
        if preference.schema_version != PRESET_CONFIG_VERSION {
            return Ok(RuntimePresetPreference::default());
        }
        Ok(preference)
    }

    pub fn save_preference(
        app: &AppHandle,
        mut preference: RuntimePresetPreference,
    ) -> Result<RuntimePresetPreference, String> {
        preference.schema_version = PRESET_CONFIG_VERSION;
        if let Some(model_id) = &preference.user_override_model {
            if model_id.trim().is_empty() {
                preference.user_override_model = None;
            } else {
                preference.user_override_model = Some(model_id.trim().to_string());
            }
        }

        let path = preference_path(app)?;
        let temp_path = path.with_file_name("runtime-preset.json.tmp");
        let serialized =
            serde_json::to_vec_pretty(&preference).map_err(|error| error.to_string())?;
        fs::write(&temp_path, serialized).map_err(|error| error.to_string())?;
        let _ = fs::remove_file(&path);
        fs::rename(&temp_path, &path).map_err(|error| error.to_string())?;
        Ok(preference)
    }
}

fn preset_spec(preset: PresetType) -> PresetSpec {
    match preset {
        PresetType::Fast => PresetSpec {
            model_id: "ggml-tiny",
            tradeoff_description:
                "Fastest speed and lowest memory; lower accuracy than larger models.",
        },
        PresetType::Balanced => PresetSpec {
            model_id: "ggml-base",
            tradeoff_description: "Balanced speed and accuracy for common talking-head projects.",
        },
        PresetType::Accurate => PresetSpec {
            model_id: "ggml-small",
            tradeoff_description: "Higher accuracy with higher memory use and transcription time.",
        },
        PresetType::Custom => PresetSpec {
            model_id: "ggml-tiny",
            tradeoff_description: "Custom settings with runtime compatibility checks.",
        },
    }
}

fn runtime_backend_available(profile: &RuntimeProfile) -> bool {
    profile
        .runtime_backends
        .iter()
        .any(|backend| backend == &profile.supported_acceleration)
}

fn preference_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("runtime-preset.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> RuntimeProfile {
        RuntimeProfile {
            cpu_name: "Intel".to_string(),
            cpu_logical_cores: 8,
            total_memory_mb: 16_000,
            has_avx2: true,
            has_avx512: false,
            has_gpu: false,
            gpu_names: vec![],
            supported_acceleration: "CPU_AVX2".to_string(),
            runtime_available: true,
            runtime_version: Some("1.9.2".to_string()),
            runtime_backends: vec!["CPU_BASIC".to_string(), "CPU_AVX2".to_string()],
            recommended_model_ids: vec![
                "ggml-tiny".to_string(),
                "ggml-base".to_string(),
                "ggml-small".to_string(),
            ],
            fallback_reason: None,
        }
    }

    #[test]
    fn config_is_canonical_and_versioned() {
        assert_eq!(PRESET_CONFIG_VERSION, 1);
        assert_eq!(preset_spec(PresetType::Fast).model_id, "ggml-tiny");
        assert_eq!(preset_spec(PresetType::Balanced).model_id, "ggml-base");
        assert_eq!(preset_spec(PresetType::Accurate).model_id, "ggml-small");
    }

    #[test]
    fn resolver_uses_installed_compatible_ideal() {
        let resolution = RuntimePresetService::resolve_preset(
            PresetType::Balanced,
            &profile(),
            &["ggml-base".to_string()],
            None,
        );
        assert_eq!(resolution.target_model_id, "ggml-base");
        assert!(resolution.is_model_installed);
        assert!(resolution.fallback_reason.is_none());
    }

    #[test]
    fn resolver_falls_back_when_ideal_is_missing() {
        let resolution = RuntimePresetService::resolve_preset(
            PresetType::Accurate,
            &profile(),
            &["ggml-tiny".to_string()],
            None,
        );
        assert_eq!(resolution.target_model_id, "ggml-tiny");
        assert!(resolution.is_model_installed);
        assert!(resolution
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("falling back")));
    }

    #[test]
    fn resolver_rejects_model_outside_memory_policy() {
        let mut low_memory = profile();
        low_memory.recommended_model_ids = vec!["ggml-tiny".to_string()];
        let resolution = RuntimePresetService::resolve_preset(
            PresetType::Accurate,
            &low_memory,
            &["ggml-small".to_string()],
            None,
        );
        assert!(!resolution.is_model_installed);
        assert!(resolution.fallback_reason.is_some());
    }

    #[test]
    fn resolver_does_not_silently_use_unavailable_runtime() {
        let mut unavailable = profile();
        unavailable.runtime_available = false;
        unavailable.runtime_backends.clear();
        let resolution = RuntimePresetService::resolve_preset(
            PresetType::Balanced,
            &unavailable,
            &["ggml-base".to_string()],
            None,
        );
        assert!(!resolution.is_model_installed);
        assert!(resolution
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("unavailable")));
    }

    #[test]
    fn preference_serialization_is_forward_versioned() {
        let preference = RuntimePresetPreference::default();
        let json = serde_json::to_string(&preference).expect("serialize preference");
        let restored: RuntimePresetPreference =
            serde_json::from_str(&json).expect("deserialize preference");
        assert_eq!(restored.schema_version, PRESET_CONFIG_VERSION);
        assert_eq!(restored.preset, PresetType::Balanced);
    }
}
