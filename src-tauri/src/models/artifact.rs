use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Number, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Versioned canonical encoding used by every generated-artifact signature.
/// Bump this when the canonicalization rules themselves change.
pub const ARTIFACT_SIGNATURE_ALGORITHM: &str = "sha256-canonical-json-v1";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactType {
    MediaMetadata,
    ExtractedAudio,
    Transcript,
    SilenceAnalysis,
    LocalAnalysis,
    AiAnalysis,
    Caption,
    Preview,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MediaMetadata => "mediaMetadata",
            Self::ExtractedAudio => "extractedAudio",
            Self::Transcript => "transcript",
            Self::SilenceAnalysis => "silenceAnalysis",
            Self::LocalAnalysis => "localAnalysis",
            Self::AiAnalysis => "aiAnalysis",
            Self::Caption => "caption",
            Self::Preview => "preview",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSignature {
    pub artifact_type: ArtifactType,
    pub artifact_version: u32,
    pub signature: String,
    pub depends_on: Vec<String>,
    pub inputs: Value,
}

/// `ArtifactSignature` is the persisted descriptor consumed by the registry.
pub type ArtifactDescriptor = ArtifactSignature;

impl ArtifactSignature {
    pub fn new(
        artifact_type: ArtifactType,
        artifact_version: u32,
        depends_on: Vec<String>,
        inputs: Value,
    ) -> Self {
        let depends_on = canonical_dependencies(depends_on);
        let inputs = canonicalize_inputs(&artifact_type, &inputs);
        let signature = generate_signature_with_dependencies(
            &artifact_type,
            artifact_version,
            &depends_on,
            &inputs,
        );

        Self {
            artifact_type,
            artifact_version,
            signature,
            depends_on,
            inputs,
        }
    }
}

/// Canonicalization deliberately excludes fields that describe a running job,
/// not the generated output. They must never make a reusable artifact stale.
const VOLATILE_INPUT_KEYS: [&str; 7] = [
    "progress",
    "progressTimestamp",
    "tempPath",
    "temporaryPath",
    "randomId",
    "jobId",
    "generatedAt",
];

/// Ensures stable key ordering, explicit artifact defaults, canonical numbers,
/// and removal of volatile job metadata before hashing.
pub fn canonicalize_inputs(artifact_type: &ArtifactType, inputs: &Value) -> Value {
    let mut value = match inputs {
        Value::Object(map) => {
            let mut object = map.clone();
            apply_explicit_defaults(artifact_type, &mut object);
            Value::Object(object)
        }
        _ => inputs.clone(),
    };

    value = canonicalize(&value);
    value
}

/// Canonical Project/analysis time unit: milliseconds since the source start.
/// Callers should convert seconds at the boundary and persist only `*Ms` fields.
pub fn seconds_to_milliseconds(seconds: f64) -> Result<u64, &'static str> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("timestamp must be a finite, non-negative number");
    }

    let milliseconds = (seconds * 1000.0).round();
    if milliseconds > u64::MAX as f64 {
        return Err("timestamp exceeds u64 milliseconds");
    }

    Ok(milliseconds as u64)
}

fn apply_explicit_defaults(artifact_type: &ArtifactType, object: &mut Map<String, Value>) {
    let defaults = match artifact_type {
        ArtifactType::ExtractedAudio => json!({
            "sampleRate": 16000,
            "channels": 1,
            "codec": "pcm_s16le"
        }),
        ArtifactType::Transcript => json!({
            "language": "auto",
            "options": {}
        }),
        ArtifactType::SilenceAnalysis | ArtifactType::LocalAnalysis => json!({
            "algorithmVersion": 1
        }),
        ArtifactType::AiAnalysis => json!({
            "provider": "none",
            "schemaVersion": 1
        }),
        ArtifactType::Caption => json!({
            "style": "default"
        }),
        ArtifactType::Preview => json!({
            "rangeStartMs": 0
        }),
        ArtifactType::MediaMetadata => json!({}),
    };

    if let Value::Object(defaults) = defaults {
        for (key, value) in defaults {
            object.entry(key).or_insert(value);
        }
    }
}

/// Recursively sorts object keys and normalizes JSON numbers.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut btree = BTreeMap::new();
            for (key, value) in map {
                if VOLATILE_INPUT_KEYS.contains(&key.as_str()) {
                    continue;
                }
                btree.insert(key.clone(), canonicalize(value));
            }
            let mut result_map = Map::new();
            for (key, value) in btree {
                result_map.insert(key, value);
            }
            Value::Object(result_map)
        }
        Value::Array(array) => Value::Array(array.iter().map(canonicalize).collect()),
        Value::Number(number) => Value::Number(canonicalize_number(number)),
        _ => value.clone(),
    }
}

fn canonicalize_number(number: &Number) -> Number {
    if let Some(value) = number.as_i64() {
        return Number::from(value);
    }
    if let Some(value) = number.as_u64() {
        return Number::from(value);
    }

    let value = number.as_f64().unwrap_or_default();
    if value == 0.0 {
        return Number::from(0);
    }
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        return Number::from(value as i64);
    }

    Number::from_f64(value).unwrap_or_else(|| Number::from(0))
}

fn canonical_dependencies(mut dependencies: Vec<String>) -> Vec<String> {
    dependencies.retain(|dependency| !dependency.is_empty());
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

/// Backward-compatible helper for callers that do not have dependencies yet.
/// New artifact producers should call `generate_signature_with_dependencies`.
pub fn generate_signature(artifact_type: &ArtifactType, version: u32, inputs: &Value) -> String {
    generate_signature_with_dependencies(artifact_type, version, &[], inputs)
}

/// Binds type, algorithm version, dependency fingerprints and canonical inputs.
pub fn generate_signature_with_dependencies(
    artifact_type: &ArtifactType,
    version: u32,
    depends_on: &[String],
    inputs: &Value,
) -> String {
    let dependencies = canonical_dependencies(depends_on.to_vec());
    let payload = json!({
        "algorithm": ARTIFACT_SIGNATURE_ALGORITHM,
        "artifactType": artifact_type.as_str(),
        "artifactVersion": version,
        "dependsOn": dependencies,
        "inputs": canonicalize_inputs(artifact_type, inputs),
    });
    let canonical_payload = canonicalize(&payload);
    let encoded = serde_json::to_vec(&canonical_payload).unwrap_or_default();

    let mut hasher = Sha256::new();
    hasher.update(encoded);
    let result = hasher.finalize();
    result.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Path-independent metadata fingerprint. Use content fingerprint when mtime
/// granularity or external file synchronization makes metadata insufficient.
pub fn get_file_fingerprint<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let metadata = fs::metadata(path.as_ref())?;
    let size = metadata.len();
    let mtime = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut hasher = Sha256::new();
    hasher.update(size.to_le_bytes());
    hasher.update(mtime.to_le_bytes());
    Ok(format_digest(hasher.finalize()))
}

/// Optional stronger fingerprint for relinked files with unreliable mtime.
pub fn get_content_fingerprint<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format_digest(hasher.finalize()))
}

fn format_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn canonical_signature_is_order_independent_and_has_explicit_defaults() {
        let input1 = json!({
            "model": "ggml-tiny",
            "thresholdDb": -35,
            "language": "vi"
        });
        let input2 = json!({
            "language": "vi",
            "thresholdDb": -35.0,
            "model": "ggml-tiny",
            "algorithmVersion": 1
        });

        let sig1 = generate_signature(&ArtifactType::SilenceAnalysis, 1, &input1);
        let sig2 = generate_signature(&ArtifactType::SilenceAnalysis, 1, &input2);
        assert_eq!(sig1, sig2);
        assert_eq!(
            sig1,
            "11be3e76f488cfb1c4950da8a76010408c9e31ecc0d7905a3521af28e4721933"
        );
    }

    #[test]
    fn dependency_fingerprint_changes_the_signature() {
        let inputs = json!({"language": "vi"});
        let first = generate_signature_with_dependencies(
            &ArtifactType::Transcript,
            1,
            &["audio:a".to_string()],
            &inputs,
        );
        let second = generate_signature_with_dependencies(
            &ArtifactType::Transcript,
            1,
            &["audio:b".to_string()],
            &inputs,
        );
        assert_ne!(first, second);
    }

    #[test]
    fn volatile_job_fields_do_not_change_the_signature() {
        let stable = json!({"model": "ggml-tiny", "language": "vi"});
        let with_job_fields = json!({
            "model": "ggml-tiny",
            "language": "vi",
            "progress": 0.75,
            "progressTimestamp": 123,
            "tempPath": "C:/temp/job.wav",
            "randomId": "different"
        });

        assert_eq!(
            generate_signature(&ArtifactType::Transcript, 1, &stable),
            generate_signature(&ArtifactType::Transcript, 1, &with_job_fields)
        );
    }

    #[test]
    fn timestamp_conversion_is_canonical_milliseconds() {
        assert_eq!(seconds_to_milliseconds(1.2345).unwrap(), 1235);
        assert!(seconds_to_milliseconds(-1.0).is_err());
        assert!(seconds_to_milliseconds(f64::NAN).is_err());
    }

    #[test]
    fn file_fingerprints_are_available_at_metadata_and_content_strengths() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"dummy data").unwrap();
        temp_file.flush().unwrap();

        assert_eq!(get_file_fingerprint(temp_file.path()).unwrap().len(), 64);
        assert_eq!(get_content_fingerprint(temp_file.path()).unwrap().len(), 64);
    }
}
