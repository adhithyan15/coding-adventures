//! # skill-store
//!
//! `skill-store` keeps reusable agent behavior packages in storage rather than
//! on a particular filesystem. A skill installation is two things:
//!
//! - a manifest that explains the skill
//! - a bundle of opaque assets addressed by logical path

use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{parse as parse_json, JsonValue};
use coding_adventures_sha256::sha256;
use storage_core::{StorageBackend, StorageError, StorageListOptions, StoragePutInput};

const NAMESPACE: &str = "skills";

/// Installed skill manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillManifest {
    pub skill_id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub entrypoints: Vec<String>,
    pub required_tools: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub assets: Vec<String>,
    pub source: JsonValue,
    pub active: bool,
}

/// Descriptor and bytes for one stored asset.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillAssetRecord {
    pub skill_id: String,
    pub version: String,
    pub asset_path: String,
    pub content_type: String,
    pub checksum: [u8; 32],
    pub body: Vec<u8>,
}

/// Input used when installing one asset.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallSkillAssetInput {
    pub asset_path: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Typed skill store layered on `storage-core`.
pub struct SkillStore<S: StorageBackend> {
    backend: S,
}

impl<S: StorageBackend> SkillStore<S> {
    pub fn new(backend: S) -> Self {
        Self { backend }
    }

    pub fn install_skill(
        &self,
        manifest: SkillManifest,
        assets: Vec<InstallSkillAssetInput>,
    ) -> Result<SkillManifest, StorageError> {
        validate_manifest(&manifest)?;
        for asset in &assets {
            validate_asset_path(&asset.asset_path)?;
            validate_content_type(&asset.content_type)?;
        }

        self.backend.initialize()?;
        for asset in assets {
            let checksum = sha256(&asset.body);
            self.backend.put(StoragePutInput::new(
                NAMESPACE,
                asset_key(&manifest.skill_id, &manifest.version, &asset.asset_path),
                &asset.content_type,
                asset_record_metadata(
                    &manifest.skill_id,
                    &manifest.version,
                    &asset.asset_path,
                    &asset.content_type,
                    checksum,
                ),
                asset.body,
            )?)?;
        }

        let persisted = self.persist_manifest(&manifest, None)?;
        if persisted.active {
            self.set_active_version(&persisted.skill_id, &persisted.version)?;
        }
        Ok(persisted)
    }

    pub fn load_manifest(
        &self,
        skill_id: &str,
        version: &str,
    ) -> Result<Option<SkillManifest>, StorageError> {
        validate_id("skill_id", skill_id)?;
        validate_id("version", version)?;
        self.backend.initialize()?;
        let Some(record) = self
            .backend
            .get(NAMESPACE, &manifest_key(skill_id, version))?
        else {
            return Ok(None);
        };
        decode_manifest(&record.body).map(Some)
    }

    pub fn list_installed_skills(&self) -> Result<Vec<SkillManifest>, StorageError> {
        self.backend.initialize()?;
        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some("manifests/".to_string()),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;

        page.records
            .iter()
            .map(|record| decode_manifest(&record.body))
            .collect()
    }

    pub fn read_asset(
        &self,
        skill_id: &str,
        version: &str,
        asset_path: &str,
    ) -> Result<Option<SkillAssetRecord>, StorageError> {
        validate_id("skill_id", skill_id)?;
        validate_id("version", version)?;
        validate_asset_path(asset_path)?;

        self.backend.initialize()?;
        let Some(record) = self
            .backend
            .get(NAMESPACE, &asset_key(skill_id, version, asset_path))?
        else {
            return Ok(None);
        };
        decode_asset_record(&record.metadata, &record.body).map(Some)
    }

    pub fn activate_version(
        &self,
        skill_id: &str,
        version: &str,
    ) -> Result<SkillManifest, StorageError> {
        self.set_active_version(skill_id, version)?;
        self.load_manifest(skill_id, version)?
            .ok_or_else(|| StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: manifest_key(skill_id, version),
            })
    }

    pub fn deactivate_version(
        &self,
        skill_id: &str,
        version: &str,
    ) -> Result<SkillManifest, StorageError> {
        let Some((mut manifest, revision)) =
            self.fetch_manifest_with_revision(skill_id, version)?
        else {
            return Err(StorageError::NotFound {
                namespace: NAMESPACE.to_string(),
                key: manifest_key(skill_id, version),
            });
        };
        manifest.active = false;
        self.persist_manifest(&manifest, Some(revision))
    }

    pub fn uninstall_skill(&self, skill_id: &str, version: &str) -> Result<(), StorageError> {
        validate_id("skill_id", skill_id)?;
        validate_id("version", version)?;
        self.backend.initialize()?;

        let page = self.backend.list(
            NAMESPACE,
            StorageListOptions {
                prefix: Some(format!("assets/{skill_id}/{version}/")),
                recursive: true,
                page_size: None,
                cursor: None,
            },
        )?;
        for record in page.records {
            self.backend.delete(NAMESPACE, &record.key, None)?;
        }
        self.backend
            .delete(NAMESPACE, &manifest_key(skill_id, version), None)?;
        Ok(())
    }

    fn set_active_version(&self, skill_id: &str, version: &str) -> Result<(), StorageError> {
        let manifests = self.list_installed_skills()?;
        for manifest in manifests
            .into_iter()
            .filter(|manifest| manifest.skill_id == skill_id)
        {
            let Some((mut stored, revision)) =
                self.fetch_manifest_with_revision(&manifest.skill_id, &manifest.version)?
            else {
                continue;
            };
            stored.active = stored.version == version;
            let _ = self.persist_manifest(&stored, Some(revision))?;
        }
        Ok(())
    }

    fn fetch_manifest_with_revision(
        &self,
        skill_id: &str,
        version: &str,
    ) -> Result<Option<(SkillManifest, storage_core::Revision)>, StorageError> {
        self.backend.initialize()?;
        let Some(record) = self
            .backend
            .get(NAMESPACE, &manifest_key(skill_id, version))?
        else {
            return Ok(None);
        };
        let manifest = decode_manifest(&record.body)?;
        Ok(Some((manifest, record.revision)))
    }

    fn persist_manifest(
        &self,
        manifest: &SkillManifest,
        if_revision: Option<storage_core::Revision>,
    ) -> Result<SkillManifest, StorageError> {
        let record = self.backend.put(
            StoragePutInput::new(
                NAMESPACE,
                manifest_key(&manifest.skill_id, &manifest.version),
                "application/json",
                manifest_record_metadata(manifest),
                encode_json(&manifest_to_json(manifest))?,
            )?
            .with_if_revision(if_revision),
        )?;
        decode_manifest(&record.body)
    }
}

fn manifest_key(skill_id: &str, version: &str) -> String {
    format!("manifests/{skill_id}/{version}.json")
}

fn asset_key(skill_id: &str, version: &str, asset_path: &str) -> String {
    format!("assets/{skill_id}/{version}/{asset_path}")
}

fn manifest_record_metadata(manifest: &SkillManifest) -> JsonValue {
    JsonValue::Object(vec![
        (
            "skill_id".to_string(),
            JsonValue::String(manifest.skill_id.clone()),
        ),
        (
            "version".to_string(),
            JsonValue::String(manifest.version.clone()),
        ),
        ("active".to_string(), JsonValue::Bool(manifest.active)),
    ])
}

fn asset_record_metadata(
    skill_id: &str,
    version: &str,
    asset_path: &str,
    content_type: &str,
    checksum: [u8; 32],
) -> JsonValue {
    JsonValue::Object(vec![
        (
            "skill_id".to_string(),
            JsonValue::String(skill_id.to_string()),
        ),
        (
            "version".to_string(),
            JsonValue::String(version.to_string()),
        ),
        (
            "asset_path".to_string(),
            JsonValue::String(asset_path.to_string()),
        ),
        (
            "content_type".to_string(),
            JsonValue::String(content_type.to_string()),
        ),
        (
            "checksum_hex".to_string(),
            JsonValue::String(hex_encode(&checksum)),
        ),
    ])
}

fn manifest_to_json(manifest: &SkillManifest) -> JsonValue {
    JsonValue::Object(vec![
        (
            "skill_id".to_string(),
            JsonValue::String(manifest.skill_id.clone()),
        ),
        (
            "version".to_string(),
            JsonValue::String(manifest.version.clone()),
        ),
        ("name".to_string(), JsonValue::String(manifest.name.clone())),
        (
            "description".to_string(),
            JsonValue::String(manifest.description.clone()),
        ),
        (
            "entrypoints".to_string(),
            string_array_json(&manifest.entrypoints),
        ),
        (
            "required_tools".to_string(),
            string_array_json(&manifest.required_tools),
        ),
        (
            "required_capabilities".to_string(),
            string_array_json(&manifest.required_capabilities),
        ),
        ("assets".to_string(), string_array_json(&manifest.assets)),
        ("source".to_string(), manifest.source.clone()),
        ("active".to_string(), JsonValue::Bool(manifest.active)),
    ])
}

fn decode_manifest(body: &[u8]) -> Result<SkillManifest, StorageError> {
    let value = decode_json(body)?;
    let object = expect_object("manifest", &value)?;
    Ok(SkillManifest {
        skill_id: required_string(object, "skill_id")?,
        version: required_string(object, "version")?,
        name: required_string(object, "name")?,
        description: required_string(object, "description")?,
        entrypoints: required_string_array(object, "entrypoints")?,
        required_tools: required_string_array(object, "required_tools")?,
        required_capabilities: required_string_array(object, "required_capabilities")?,
        assets: required_string_array(object, "assets")?,
        source: required_value(object, "source")?.clone(),
        active: required_bool(object, "active")?,
    })
}

fn decode_asset_record(
    metadata: &JsonValue,
    body: &[u8],
) -> Result<SkillAssetRecord, StorageError> {
    let object = expect_object("asset_metadata", metadata)?;
    let checksum_hex = required_string(object, "checksum_hex")?;
    Ok(SkillAssetRecord {
        skill_id: required_string(object, "skill_id")?,
        version: required_string(object, "version")?,
        asset_path: required_string(object, "asset_path")?,
        content_type: required_string(object, "content_type")?,
        checksum: hex_decode_32(&checksum_hex)?,
        body: body.to_vec(),
    })
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>, StorageError> {
    let text = serialize(value).map_err(|error| validation("json", error.message))?;
    Ok(text.into_bytes())
}

fn decode_json(bytes: &[u8]) -> Result<JsonValue, StorageError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| validation("body", "skill manifest must be UTF-8"))?;
    parse_json(text).map_err(|error| validation("body", error.message))
}

fn expect_object<'a>(
    label: &str,
    value: &'a JsonValue,
) -> Result<&'a Vec<(String, JsonValue)>, StorageError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(validation(label, format!("{label} must be a JSON object"))),
    }
}

fn required_value<'a>(
    object: &'a [(String, JsonValue)],
    field: &str,
) -> Result<&'a JsonValue, StorageError> {
    object
        .iter()
        .find(|(name, _)| name == field)
        .map(|(_, value)| value)
        .ok_or_else(|| validation(field, "required field was missing"))
}

fn required_string(object: &[(String, JsonValue)], field: &str) -> Result<String, StorageError> {
    match required_value(object, field)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(validation(field, "field must be a string")),
    }
}

fn required_bool(object: &[(String, JsonValue)], field: &str) -> Result<bool, StorageError> {
    match required_value(object, field)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(validation(field, "field must be a boolean")),
    }
}

fn required_string_array(
    object: &[(String, JsonValue)],
    field: &str,
) -> Result<Vec<String>, StorageError> {
    match required_value(object, field)? {
        JsonValue::Array(values) => values
            .iter()
            .map(|value| match value {
                JsonValue::String(string) => Ok(string.clone()),
                _ => Err(validation(field, "array elements must be strings")),
            })
            .collect(),
        _ => Err(validation(field, "field must be an array")),
    }
}

fn validate_manifest(manifest: &SkillManifest) -> Result<(), StorageError> {
    validate_id("skill_id", &manifest.skill_id)?;
    validate_id("version", &manifest.version)?;
    validate_name("name", &manifest.name)?;
    validate_name("description", &manifest.description)?;
    validate_id_list("entrypoints", &manifest.entrypoints)?;
    validate_id_list("required_tools", &manifest.required_tools)?;
    validate_id_list("required_capabilities", &manifest.required_capabilities)?;
    for asset in &manifest.assets {
        validate_asset_path(asset)?;
    }
    Ok(())
}

fn validate_id(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(validation(field, "must not be empty"));
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
    {
        return Err(validation(
            field,
            "must use only ASCII letters, digits, dots, underscores, or hyphens",
        ));
    }
    Ok(())
}

fn validate_id_list(field: &str, values: &[String]) -> Result<(), StorageError> {
    for value in values {
        validate_id(field, value)?;
    }
    Ok(())
}

fn validate_asset_path(value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(validation("asset_path", "must not be empty"));
    }
    if value.starts_with('/') || value.ends_with('/') || value.contains("//") {
        return Err(validation("asset_path", "must be a relative logical path"));
    }
    for segment in value.split('/') {
        validate_id("asset_path_segment", segment)?;
    }
    Ok(())
}

fn validate_name(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(validation(field, "must not be empty"));
    }
    Ok(())
}

fn validate_content_type(value: &str) -> Result<(), StorageError> {
    if !value.contains('/') {
        return Err(validation(
            "content_type",
            "must contain a slash like a MIME type",
        ));
    }
    Ok(())
}

fn string_array_json(values: &[String]) -> JsonValue {
    JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::String(value.clone()))
            .collect(),
    )
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode_32(value: &str) -> Result<[u8; 32], StorageError> {
    if value.len() != 64 {
        return Err(validation("checksum_hex", "must be 64 lowercase hex chars"));
    }
    let mut bytes = [0u8; 32];
    for (index, chunk) in value.as_bytes().chunks(2).enumerate() {
        let text =
            std::str::from_utf8(chunk).map_err(|_| validation("checksum_hex", "invalid UTF-8"))?;
        bytes[index] = u8::from_str_radix(text, 16)
            .map_err(|_| validation("checksum_hex", "must be valid hexadecimal"))?;
    }
    Ok(bytes)
}

fn validation(field: &str, message: impl Into<String>) -> StorageError {
    StorageError::Validation {
        field: field.to_string(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage_core::InMemoryStorageBackend;

    fn manifest(active: bool) -> SkillManifest {
        SkillManifest {
            skill_id: "planner".to_string(),
            version: if active { "v1" } else { "v2" }.to_string(),
            name: "Planner".to_string(),
            description: "Plans work".to_string(),
            entrypoints: vec!["main".to_string()],
            required_tools: vec!["shell".to_string()],
            required_capabilities: vec!["write".to_string()],
            assets: vec!["SKILL.md".to_string()],
            source: JsonValue::Object(vec![(
                "kind".to_string(),
                JsonValue::String("local".to_string()),
            )]),
            active,
        }
    }

    #[test]
    fn install_and_read_asset_round_trip() {
        let store = SkillStore::new(InMemoryStorageBackend::new());
        let _ = store
            .install_skill(
                manifest(true),
                vec![InstallSkillAssetInput {
                    asset_path: "SKILL.md".to_string(),
                    content_type: "text/markdown".to_string(),
                    body: b"# hi".to_vec(),
                }],
            )
            .unwrap();

        let asset = store
            .read_asset("planner", "v1", "SKILL.md")
            .unwrap()
            .unwrap();
        assert_eq!(asset.body, b"# hi".to_vec());
    }

    #[test]
    fn activating_new_version_deactivates_old_version() {
        let store = SkillStore::new(InMemoryStorageBackend::new());
        let _ = store
            .install_skill(
                manifest(true),
                vec![InstallSkillAssetInput {
                    asset_path: "SKILL.md".to_string(),
                    content_type: "text/markdown".to_string(),
                    body: b"# hi".to_vec(),
                }],
            )
            .unwrap();
        let _ = store
            .install_skill(
                manifest(false),
                vec![InstallSkillAssetInput {
                    asset_path: "SKILL.md".to_string(),
                    content_type: "text/markdown".to_string(),
                    body: b"# v2".to_vec(),
                }],
            )
            .unwrap();

        let activated = store.activate_version("planner", "v2").unwrap();
        assert!(activated.active);
        assert!(
            !store
                .load_manifest("planner", "v1")
                .unwrap()
                .unwrap()
                .active
        );
    }

    #[test]
    fn deactivate_and_uninstall_remove_skill_material() {
        let store = SkillStore::new(InMemoryStorageBackend::new());
        let _ = store
            .install_skill(
                manifest(true),
                vec![InstallSkillAssetInput {
                    asset_path: "SKILL.md".to_string(),
                    content_type: "text/markdown".to_string(),
                    body: b"# hi".to_vec(),
                }],
            )
            .unwrap();

        let inactive = store.deactivate_version("planner", "v1").unwrap();
        assert!(!inactive.active);

        store.uninstall_skill("planner", "v1").unwrap();
        assert_eq!(store.load_manifest("planner", "v1").unwrap(), None);
        assert_eq!(store.read_asset("planner", "v1", "SKILL.md").unwrap(), None);
    }

    #[test]
    fn helper_validations_cover_error_paths() {
        assert!(validate_asset_path("/bad").is_err());
        assert!(validate_content_type("text").is_err());
        assert!(hex_decode_32("zz").is_err());
        assert!(expect_object("manifest", &JsonValue::String("bad".to_string())).is_err());
    }
}
