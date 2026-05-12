//! Immutable capability manifest.
//!
//! A `Manifest` is constructed from a `required_capabilities.json`
//! (or programmatically from a list of [`Capability`]) and answers
//! the single question every secure wrapper asks before performing
//! an OS operation: *does this manifest cover this (category,
//! action, target) triple?*
//!
//! The default — an empty manifest — grants no OS access. This is
//! the expected state for the majority of pure-computation packages.

use std::collections::HashSet;
use std::path::Path;

use coding_adventures_json_value::{parse, JsonNumber, JsonValue};
use read_write_separation::{CapabilityFlavor, CapabilityTrust};

use crate::capability::Capability;
use crate::category::{Action, Category};
#[cfg(test)]
use crate::errors::InvalidCombination;
use crate::errors::{CapabilityViolationError, ManifestError};
use crate::glob::match_target;

/// An immutable capability manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    capabilities: Vec<Capability>,
}

/// Payload-free read-side view of one capability manifest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapabilitySurfaceSummary {
    pub total_capabilities: usize,
    pub unique_categories: usize,
    pub unique_actions: usize,
    pub unique_targets: usize,
    pub filesystem_capabilities: usize,
    pub network_capabilities: usize,
    pub process_capabilities: usize,
    pub environment_capabilities: usize,
    pub ffi_capabilities: usize,
    pub time_capabilities: usize,
    pub stdio_capabilities: usize,
    pub read_like_capabilities: usize,
    pub write_like_capabilities: usize,
    pub external_effect_capabilities: usize,
    pub ingestion_capabilities: usize,
    pub actuation_capabilities: usize,
    pub internal_capabilities: usize,
    pub trusted_capabilities: usize,
    pub untrusted_capabilities: usize,
    pub unannotated_capabilities: usize,
}

impl CapabilitySurfaceSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_capabilities<'a, I>(capabilities: I) -> Self
    where
        I: IntoIterator<Item = &'a Capability>,
    {
        let mut summary = Self::empty();
        let mut categories = HashSet::new();
        let mut actions = HashSet::new();
        let mut targets = HashSet::new();

        for capability in capabilities {
            summary.total_capabilities += 1;
            categories.insert(capability.category);
            actions.insert(capability.action);
            targets.insert(capability.target.as_str());

            match capability.category {
                Category::Fs => summary.filesystem_capabilities += 1,
                Category::Net => summary.network_capabilities += 1,
                Category::Proc => summary.process_capabilities += 1,
                Category::Env => summary.environment_capabilities += 1,
                Category::Ffi => summary.ffi_capabilities += 1,
                Category::Time => summary.time_capabilities += 1,
                Category::Stdin | Category::Stdout => summary.stdio_capabilities += 1,
            }
            if is_read_like_action(capability.action) {
                summary.read_like_capabilities += 1;
            }
            if is_write_like_action(capability.action) {
                summary.write_like_capabilities += 1;
            }
            if is_external_effect_capability(capability) {
                summary.external_effect_capabilities += 1;
            }
            match capability.flavor {
                Some(CapabilityFlavor::Ingestion) => summary.ingestion_capabilities += 1,
                Some(CapabilityFlavor::Actuation) => summary.actuation_capabilities += 1,
                Some(CapabilityFlavor::Internal) => summary.internal_capabilities += 1,
                None => {}
            }
            match capability.trust {
                Some(CapabilityTrust::Trusted) => summary.trusted_capabilities += 1,
                Some(CapabilityTrust::Untrusted) => summary.untrusted_capabilities += 1,
                None => {}
            }
            if capability.flavor.is_none() && capability.trust.is_none() {
                summary.unannotated_capabilities += 1;
            }
        }

        summary.unique_categories = categories.len();
        summary.unique_actions = actions.len();
        summary.unique_targets = targets.len();
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_capabilities == 0
    }

    pub fn has_network_access(&self) -> bool {
        self.network_capabilities > 0
    }

    pub fn has_process_access(&self) -> bool {
        self.process_capabilities > 0
    }

    pub fn has_write_or_actuation(&self) -> bool {
        self.write_like_capabilities > 0 || self.actuation_capabilities > 0
    }

    pub fn crosses_untrusted_boundary(&self) -> bool {
        self.untrusted_capabilities > 0
    }

    pub fn fully_annotated(&self) -> bool {
        self.is_empty() || self.unannotated_capabilities == 0
    }
}

impl Manifest {
    /// Construct a manifest from a list of capabilities.
    ///
    /// Each capability must have already been validated via
    /// [`Capability::new`]; this constructor does not re-check.
    pub fn new(capabilities: Vec<Capability>) -> Self {
        Self { capabilities }
    }

    /// Construct a manifest and validate read/write separation.
    ///
    /// This is the constructor to use for manifests assembled from
    /// package metadata. [`Manifest::new`] remains available for
    /// tests and already-gated in-memory manifests.
    pub fn try_new(capabilities: Vec<Capability>) -> Result<Self, ManifestError> {
        validate_read_write_separation(&capabilities)?;
        Ok(Self::new(capabilities))
    }

    /// The pre-built zero-capability manifest. Equivalent to
    /// `Manifest::new(vec![])`.
    pub fn empty() -> Manifest {
        Manifest::new(vec![])
    }

    /// Load a manifest from a JSON string in the
    /// `required_capabilities.json` shape.
    pub fn load_from_str(s: &str) -> Result<Self, ManifestError> {
        let root = parse(s).map_err(|e| ManifestError::Parse(e.to_string()))?;
        Self::from_json(&root)
    }

    /// Load a manifest from a path to a JSON file.
    pub fn load_from_file(path: &Path) -> Result<Self, ManifestError> {
        let bytes = std::fs::read(path)?;
        let s = std::str::from_utf8(&bytes)
            .map_err(|e| ManifestError::Parse(format!("manifest is not UTF-8: {e}")))?;
        Self::load_from_str(s)
    }

    fn from_json(root: &JsonValue) -> Result<Self, ManifestError> {
        let obj = match root {
            JsonValue::Object(pairs) => pairs,
            _ => {
                return Err(ManifestError::Schema {
                    reason: "top-level value must be a JSON object".into(),
                })
            }
        };

        let version = lookup(obj, "version")
            .and_then(json_as_i64)
            .ok_or_else(|| ManifestError::Schema {
                reason: "missing or non-integer field: version".into(),
            })?;
        if version != 1 {
            return Err(ManifestError::Schema {
                reason: format!(
                    "unsupported manifest version: {version} (only 1 is supported in v1)"
                ),
            });
        }

        let caps_value = lookup(obj, "capabilities").ok_or_else(|| ManifestError::Schema {
            reason: "missing field: capabilities".into(),
        })?;
        let caps_array = match caps_value {
            JsonValue::Array(items) => items,
            _ => {
                return Err(ManifestError::Schema {
                    reason: "capabilities must be an array".into(),
                })
            }
        };

        let mut caps = Vec::with_capacity(caps_array.len());
        for (idx, entry) in caps_array.iter().enumerate() {
            let entry_obj = match entry {
                JsonValue::Object(pairs) => pairs,
                _ => {
                    return Err(ManifestError::Schema {
                        reason: format!("capabilities[{idx}] must be a JSON object"),
                    })
                }
            };
            let category_str = lookup(entry_obj, "category")
                .and_then(json_as_str)
                .ok_or_else(|| ManifestError::Schema {
                    reason: format!("capabilities[{idx}] missing string 'category'"),
                })?;
            let action_str = lookup(entry_obj, "action")
                .and_then(json_as_str)
                .ok_or_else(|| ManifestError::Schema {
                    reason: format!("capabilities[{idx}] missing string 'action'"),
                })?;
            let target = lookup(entry_obj, "target")
                .and_then(json_as_str)
                .ok_or_else(|| ManifestError::Schema {
                    reason: format!("capabilities[{idx}] missing string 'target'"),
                })?
                .to_string();
            let justification = lookup(entry_obj, "justification")
                .and_then(json_as_str)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let flavor = parse_optional_flavor(entry_obj, idx)?;
            let trust = parse_optional_trust(entry_obj, idx)?;

            let category: Category = category_str.parse().map_err(|()| ManifestError::Schema {
                reason: format!(
                    "capabilities[{idx}].category '{category_str}' is not a known category"
                ),
            })?;
            let action: Action = action_str.parse().map_err(|()| ManifestError::Schema {
                reason: format!("capabilities[{idx}].action '{action_str}' is not a known action"),
            })?;
            let mut cap = Capability::new(category, action, target, justification)?;
            if let Some(flavor) = flavor {
                cap = cap.with_flavor(flavor);
            }
            if let Some(trust) = trust {
                cap = cap.with_trust(trust);
            }
            caps.push(cap);
        }

        Manifest::try_new(caps)
    }

    /// Returns true if the manifest declares a capability that covers
    /// the (category, action, target) triple. Glob targets in the
    /// manifest are matched against the literal `target` argument.
    pub fn has(&self, category: Category, action: Action, target: &str) -> bool {
        self.capabilities.iter().any(|cap| {
            cap.category == category && cap.action == action && match_target(&cap.target, target)
        })
    }

    /// Returns Ok(()) if the manifest covers the triple, otherwise
    /// returns [`CapabilityViolationError`] with a diagnostic message.
    pub fn check(
        &self,
        category: Category,
        action: Action,
        target: &str,
    ) -> Result<(), CapabilityViolationError> {
        if self.has(category, action, target) {
            Ok(())
        } else {
            Err(CapabilityViolationError {
                category,
                action,
                target: target.to_string(),
                message: format!(
                    "capability {}:{}:{} not declared; add it to required_capabilities.json",
                    category, action, target,
                ),
            })
        }
    }

    /// Borrow the capability list.
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Return a payload-free summary of the manifest's capability surface.
    pub fn capability_surface_summary(&self) -> CapabilitySurfaceSummary {
        CapabilitySurfaceSummary::from_capabilities(&self.capabilities)
    }

    /// Re-run read/write separation validation for this manifest.
    pub fn validate_read_write_separation(&self) -> Result<(), ManifestError> {
        validate_read_write_separation(&self.capabilities)
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest::empty()
    }
}

fn lookup<'a>(obj: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn json_as_str(v: &JsonValue) -> Option<&str> {
    match v {
        JsonValue::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn json_as_i64(v: &JsonValue) -> Option<i64> {
    match v {
        JsonValue::Number(JsonNumber::Integer(n)) => Some(*n),
        _ => None,
    }
}

fn parse_optional_flavor(
    obj: &[(String, JsonValue)],
    idx: usize,
) -> Result<Option<CapabilityFlavor>, ManifestError> {
    parse_optional_enum(obj, "flavor", idx, |value| match value {
        "ingestion" => Some(CapabilityFlavor::Ingestion),
        "actuation" => Some(CapabilityFlavor::Actuation),
        "internal" => Some(CapabilityFlavor::Internal),
        _ => None,
    })
}

fn parse_optional_trust(
    obj: &[(String, JsonValue)],
    idx: usize,
) -> Result<Option<CapabilityTrust>, ManifestError> {
    parse_optional_enum(obj, "trust", idx, |value| match value {
        "trusted" => Some(CapabilityTrust::Trusted),
        "untrusted" => Some(CapabilityTrust::Untrusted),
        _ => None,
    })
}

fn parse_optional_enum<T>(
    obj: &[(String, JsonValue)],
    field: &str,
    idx: usize,
    parse_value: impl FnOnce(&str) -> Option<T>,
) -> Result<Option<T>, ManifestError> {
    match lookup(obj, field) {
        None => Ok(None),
        Some(value) => {
            let raw = json_as_str(value).ok_or_else(|| ManifestError::Schema {
                reason: format!("capabilities[{idx}].{field} must be a string"),
            })?;
            parse_value(raw)
                .map(Some)
                .ok_or_else(|| ManifestError::Schema {
                    reason: format!("capabilities[{idx}].{field} '{raw}' is not supported"),
                })
        }
    }
}

fn validate_read_write_separation(capabilities: &[Capability]) -> Result<(), ManifestError> {
    let rws_capabilities: Vec<_> = capabilities
        .iter()
        .map(Capability::to_read_write_capability)
        .collect();
    read_write_separation::validate_manifest(&rws_capabilities).map_err(ManifestError::from)
}

fn is_read_like_action(action: Action) -> bool {
    matches!(action, Action::Read | Action::List | Action::Dns)
}

fn is_write_like_action(action: Action) -> bool {
    matches!(
        action,
        Action::Write
            | Action::Create
            | Action::Delete
            | Action::Connect
            | Action::Listen
            | Action::Exec
            | Action::Fork
            | Action::Signal
            | Action::Call
            | Action::Load
            | Action::Sleep
    )
}

fn is_external_effect_capability(capability: &Capability) -> bool {
    matches!(
        capability.category,
        Category::Net | Category::Proc | Category::Ffi | Category::Stdout
    ) || is_write_like_action(capability.action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs_read_cap(target: &str) -> Capability {
        Capability::new(Category::Fs, Action::Read, target, "test").unwrap()
    }

    #[test]
    fn empty_manifest_has_nothing() {
        let m = Manifest::empty();
        assert!(!m.has(Category::Fs, Action::Read, "/anything"));
        let err = m.check(Category::Fs, Action::Read, "/x").unwrap_err();
        assert_eq!(err.category, Category::Fs);
        assert_eq!(err.action, Action::Read);
        assert_eq!(err.target, "/x");
        assert!(err.message.contains("fs:read:/x"));
    }

    #[test]
    fn exact_match() {
        let m = Manifest::new(vec![fs_read_cap("./grammars/json.tokens")]);
        assert!(m.has(Category::Fs, Action::Read, "./grammars/json.tokens"));
        assert!(!m.has(Category::Fs, Action::Read, "./grammars/other.tokens"));
    }

    #[test]
    fn glob_match() {
        let m = Manifest::new(vec![fs_read_cap("./grammars/*.tokens")]);
        assert!(m.has(Category::Fs, Action::Read, "./grammars/json.tokens"));
        assert!(m.has(Category::Fs, Action::Read, "./grammars/yaml.tokens"));
        assert!(!m.has(Category::Fs, Action::Read, "./grammars/sub/json.tokens"));
    }

    #[test]
    fn category_action_must_match() {
        let m = Manifest::new(vec![fs_read_cap("./x")]);
        assert!(!m.has(Category::Fs, Action::Write, "./x"));
        assert!(!m.has(Category::Net, Action::Connect, "./x"));
    }

    #[test]
    fn check_returns_ok_on_match() {
        let m = Manifest::new(vec![fs_read_cap("./x")]);
        assert!(m.check(Category::Fs, Action::Read, "./x").is_ok());
    }

    #[test]
    fn load_minimal_pure_manifest() {
        let json = r#"{
            "version": 1,
            "package": "rust/capability-cage-test",
            "capabilities": [],
            "justification": "pure computation"
        }"#;
        let m = Manifest::load_from_str(json).unwrap();
        assert_eq!(m.capabilities().len(), 0);
    }

    #[test]
    fn load_manifest_with_one_cap() {
        let json = r#"{
            "version": 1,
            "package": "rust/json-lexer",
            "capabilities": [
                {
                    "category": "fs",
                    "action": "read",
                    "target": "./grammars/*.tokens",
                    "justification": "load lexer DFA"
                }
            ]
        }"#;
        let m = Manifest::load_from_str(json).unwrap();
        assert_eq!(m.capabilities().len(), 1);
        assert!(m.has(Category::Fs, Action::Read, "./grammars/json.tokens"));
    }

    #[test]
    fn capability_surface_summary_counts_manifest_shape_without_targets() {
        let manifest = Manifest::new(vec![
            Capability::new(Category::Fs, Action::Read, "./input/*.json", "read inputs")
                .unwrap()
                .with_flavor(CapabilityFlavor::Ingestion)
                .with_trust(CapabilityTrust::Untrusted),
            Capability::new(
                Category::Fs,
                Action::Write,
                "./out/report.json",
                "write report",
            )
            .unwrap()
            .with_flavor(CapabilityFlavor::Actuation)
            .with_trust(CapabilityTrust::Trusted),
            Capability::new(Category::Env, Action::Read, "APP_MODE", "read mode").unwrap(),
            Capability::new(Category::Stdout, Action::Write, "stdout", "emit progress").unwrap(),
        ]);

        let summary = manifest.capability_surface_summary();

        assert_eq!(
            summary,
            CapabilitySurfaceSummary {
                total_capabilities: 4,
                unique_categories: 3,
                unique_actions: 2,
                unique_targets: 4,
                filesystem_capabilities: 2,
                network_capabilities: 0,
                process_capabilities: 0,
                environment_capabilities: 1,
                ffi_capabilities: 0,
                time_capabilities: 0,
                stdio_capabilities: 1,
                read_like_capabilities: 2,
                write_like_capabilities: 2,
                external_effect_capabilities: 2,
                ingestion_capabilities: 1,
                actuation_capabilities: 1,
                internal_capabilities: 0,
                trusted_capabilities: 1,
                untrusted_capabilities: 1,
                unannotated_capabilities: 2,
            }
        );
        assert!(summary.has_write_or_actuation());
        assert!(summary.crosses_untrusted_boundary());
        assert!(!summary.has_network_access());
        assert!(!summary.has_process_access());
        assert!(!summary.fully_annotated());
    }

    #[test]
    fn empty_capability_surface_summary_is_safe_default() {
        let summary = Manifest::empty().capability_surface_summary();

        assert_eq!(summary, CapabilitySurfaceSummary::empty());
        assert!(summary.is_empty());
        assert!(summary.fully_annotated());
        assert!(!summary.has_write_or_actuation());
        assert!(!summary.crosses_untrusted_boundary());
    }

    #[test]
    fn load_manifest_preserves_read_write_annotations() {
        let json = r#"{
            "version": 1,
            "package": "rust/mail-ingest",
            "capabilities": [
                {
                    "category": "net",
                    "action": "connect",
                    "target": "imap.example.test:993",
                    "flavor": "ingestion",
                    "trust": "untrusted",
                    "justification": "read mail"
                }
            ]
        }"#;

        let m = Manifest::load_from_str(json).unwrap();
        let cap = &m.capabilities()[0];

        assert_eq!(cap.flavor, Some(CapabilityFlavor::Ingestion));
        assert_eq!(cap.trust, Some(CapabilityTrust::Untrusted));
        assert!(m.validate_read_write_separation().is_ok());
    }

    #[test]
    fn load_manifest_rejects_unknown_read_write_annotations() {
        let json = r#"{
            "version": 1,
            "capabilities": [
                {
                    "category": "net",
                    "action": "connect",
                    "target": "imap.example.test:993",
                    "flavor": "read"
                }
            ]
        }"#;

        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::Schema { reason } => assert!(reason.contains("flavor")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_mixed_untrusted_input_and_actuation() {
        let json = r#"{
            "version": 1,
            "package": "rust/mail-agent",
            "capabilities": [
                {
                    "category": "net",
                    "action": "connect",
                    "target": "imap.example.test:993",
                    "flavor": "ingestion",
                    "trust": "untrusted",
                    "justification": "read mail"
                },
                {
                    "category": "fs",
                    "action": "write",
                    "target": "package:/state/outbox.json",
                    "justification": "write outbox"
                }
            ]
        }"#;

        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::RwsViolation {
                untrusted_inputs,
                actuations,
                message,
            } => {
                assert_eq!(untrusted_inputs.len(), 1);
                assert_eq!(actuations.len(), 1);
                assert!(message.contains("untrusted inputs"));
            }
            other => panic!("expected RwsViolation error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_same_path_read_write_overlap() {
        let json = r#"{
            "version": 1,
            "package": "rust/cache-agent",
            "capabilities": [
                {
                    "category": "fs",
                    "action": "read",
                    "target": "package:/state/cache.json",
                    "trust": "trusted"
                },
                {
                    "category": "fs",
                    "action": "write",
                    "target": "package:/state/cache.json"
                }
            ]
        }"#;

        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::RwsViolation { message, .. } => assert!(message.contains("overlap")),
            other => panic!("expected RwsViolation error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_accepts_pure_actuation_default_net_connect() {
        let json = r#"{
            "version": 1,
            "package": "rust/mail-send",
            "capabilities": [
                {
                    "category": "net",
                    "action": "connect",
                    "target": "smtp.example.test:465"
                }
            ]
        }"#;

        let m = Manifest::load_from_str(json).unwrap();
        assert_eq!(m.capabilities().len(), 1);
    }

    #[test]
    fn load_manifest_rejects_missing_version() {
        let json = r#"{ "capabilities": [] }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::Schema { reason } => assert!(reason.contains("version")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_unknown_version() {
        let json = r#"{ "version": 2, "capabilities": [] }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::Schema { reason } => assert!(reason.contains("version")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_missing_capabilities() {
        let json = r#"{ "version": 1 }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::Schema { reason } => assert!(reason.contains("capabilities")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_capabilities_not_array() {
        let json = r#"{ "version": 1, "capabilities": {} }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::Schema { reason } => assert!(reason.contains("array")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_unknown_category() {
        let json = r#"{
            "version": 1,
            "capabilities": [
                { "category": "network", "action": "connect", "target": "x:80" }
            ]
        }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::Schema { reason } => assert!(reason.contains("category")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_invalid_pair() {
        // fs + connect is not a valid combination.
        let json = r#"{
            "version": 1,
            "capabilities": [
                { "category": "fs", "action": "connect", "target": "x" }
            ]
        }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::InvalidCombination(InvalidCombination::UnsupportedPair { .. }) => {}
            other => panic!("expected InvalidCombination error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_empty_target() {
        let json = r#"{
            "version": 1,
            "capabilities": [
                { "category": "fs", "action": "read", "target": "" }
            ]
        }"#;
        let err = Manifest::load_from_str(json).unwrap_err();
        match err {
            ManifestError::InvalidCombination(InvalidCombination::EmptyTarget) => {}
            other => panic!("expected EmptyTarget error, got {other:?}"),
        }
    }

    #[test]
    fn load_manifest_rejects_unparseable_json() {
        let err = Manifest::load_from_str("not json").unwrap_err();
        assert!(matches!(err, ManifestError::Parse(_)));
    }

    #[test]
    fn try_new_rejects_programmatic_rws_violations() {
        let input = Capability::new(
            Category::Net,
            Action::Connect,
            "imap.example.test:993",
            "read mail",
        )
        .unwrap()
        .with_flavor(CapabilityFlavor::Ingestion)
        .with_trust(CapabilityTrust::Untrusted);
        let actuation = Capability::new(
            Category::Fs,
            Action::Write,
            "package:/state/outbox.json",
            "write",
        )
        .unwrap();

        let err = Manifest::try_new(vec![input, actuation]).unwrap_err();
        assert!(matches!(err, ManifestError::RwsViolation { .. }));
    }
}
