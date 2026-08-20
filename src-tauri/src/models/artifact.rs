use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactType {
    Transcript,
    SilenceAnalysis,
    Preview,
    Caption,
    ExtractedAudio,
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

/// Đảm bảo thứ tự key JSON luôn đồng nhất
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut btree = BTreeMap::new();
            for (k, v) in map {
                btree.insert(k.clone(), canonicalize(v));
            }
            let mut result_map = Map::new();
            for (k, v) in btree {
                result_map.insert(k, v);
            }
            Value::Object(result_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize).collect()),
        _ => value.clone(),
    }
}

/// Băm deterministic các input để sinh ra SHA-256 signature
pub fn generate_signature(artifact_type: &ArtifactType, version: u32, inputs: &Value) -> String {
    let canonical_inputs = canonicalize(inputs);
    let mut hasher = Sha256::new();

    let type_str = match artifact_type {
        ArtifactType::Transcript => "Transcript",
        ArtifactType::SilenceAnalysis => "SilenceAnalysis",
        ArtifactType::Preview => "Preview",
        ArtifactType::Caption => "Caption",
        ArtifactType::ExtractedAudio => "ExtractedAudio",
    };

    hasher.update(type_str.as_bytes());
    hasher.update(version.to_le_bytes());

    let input_str = serde_json::to_string(&canonical_inputs).unwrap_or_default();
    hasher.update(input_str.as_bytes());

    let result = hasher.finalize();
    result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Lấy thông tin cơ bản của file (size, mtime) để băm thành fingerprint
pub fn get_file_fingerprint<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
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

    let result = hasher.finalize();
    Ok(result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_canonicalize_and_hash() {
        let input1 = json!({
            "model": "ggml-tiny",
            "thresholdDb": -35,
            "language": "vi"
        });

        let input2 = json!({
            "thresholdDb": -35,
            "language": "vi",
            "model": "ggml-tiny"
        });

        let input3 = json!({
            "thresholdDb": -30, // Changed
            "language": "vi",
            "model": "ggml-tiny"
        });

        let sig1 = generate_signature(&ArtifactType::SilenceAnalysis, 1, &input1);
        let sig2 = generate_signature(&ArtifactType::SilenceAnalysis, 1, &input2);
        let sig3 = generate_signature(&ArtifactType::SilenceAnalysis, 1, &input3);
        let sig4 = generate_signature(&ArtifactType::SilenceAnalysis, 2, &input1); // Changed version

        assert_eq!(
            sig1, sig2,
            "Reordered JSON keys should produce the same signature"
        );
        assert_ne!(
            sig1, sig3,
            "Different JSON values should produce different signatures"
        );
        assert_ne!(
            sig1, sig4,
            "Different artifact versions should produce different signatures"
        );
    }

    #[test]
    fn test_file_fingerprint() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"dummy data").unwrap();
        temp_file.flush().unwrap();

        let fp1 = get_file_fingerprint(temp_file.path()).unwrap();

        // Wait a bit to ensure mtime changes if we update the file, but we can't easily wait.
        // We can just verify it returns a valid hex string.
        assert_eq!(fp1.len(), 64);
    }
}
