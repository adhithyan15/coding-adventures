#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityFlavor {
    Ingestion,
    Actuation,
    Internal,
}

impl CapabilityFlavor {
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityFlavor::Ingestion => "ingestion",
            CapabilityFlavor::Actuation => "actuation",
            CapabilityFlavor::Internal => "internal",
        }
    }
}

impl fmt::Display for CapabilityFlavor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityTrust {
    Trusted,
    Untrusted,
}

impl CapabilityTrust {
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityTrust::Trusted => "trusted",
            CapabilityTrust::Untrusted => "untrusted",
        }
    }
}

impl fmt::Display for CapabilityTrust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    pub category: String,
    pub action: String,
    pub target: String,
    pub flavor: Option<CapabilityFlavor>,
    pub trust: Option<CapabilityTrust>,
    pub justification: Option<String>,
}

impl Capability {
    pub fn new(
        category: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            action: action.into(),
            target: target.into(),
            flavor: None,
            trust: None,
            justification: None,
        }
    }

    pub fn with_flavor(mut self, flavor: CapabilityFlavor) -> Self {
        self.flavor = Some(flavor);
        self
    }

    pub fn with_trust(mut self, trust: CapabilityTrust) -> Self {
        self.trust = Some(trust);
        self
    }

    pub fn with_justification(mut self, justification: impl Into<String>) -> Self {
        self.justification = Some(justification.into());
        self
    }

    pub fn identifier(&self) -> String {
        format!("{}:{}:{}", self.category, self.action, self.target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityClassification {
    pub flavor: CapabilityFlavor,
    pub trust: CapabilityTrust,
    pub is_input: bool,
    pub is_untrusted_input: bool,
    pub is_external_actuation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityManifestSummary {
    pub total_capabilities: usize,
    pub ingestion_capabilities: usize,
    pub actuation_capabilities: usize,
    pub internal_capabilities: usize,
    pub trusted_capabilities: usize,
    pub untrusted_capabilities: usize,
    pub input_capabilities: usize,
    pub untrusted_inputs: usize,
    pub external_actuations: usize,
    pub read_side_capabilities: usize,
    pub write_side_capabilities: usize,
    pub overlapping_read_write_pairs: usize,
    pub justified_capabilities: usize,
}

impl CapabilityManifestSummary {
    pub fn from_capabilities(capabilities: &[Capability]) -> Self {
        let mut summary = Self {
            overlapping_read_write_pairs: count_overlap_pairs(capabilities),
            ..Self::default()
        };

        for capability in capabilities {
            let classification = classify_capability(capability);

            summary.total_capabilities += 1;
            match classification.flavor {
                CapabilityFlavor::Ingestion => summary.ingestion_capabilities += 1,
                CapabilityFlavor::Actuation => summary.actuation_capabilities += 1,
                CapabilityFlavor::Internal => summary.internal_capabilities += 1,
            }
            match classification.trust {
                CapabilityTrust::Trusted => summary.trusted_capabilities += 1,
                CapabilityTrust::Untrusted => summary.untrusted_capabilities += 1,
            }
            if classification.is_input {
                summary.input_capabilities += 1;
            }
            if classification.is_untrusted_input {
                summary.untrusted_inputs += 1;
            }
            if classification.is_external_actuation {
                summary.external_actuations += 1;
            }
            if is_read_side(capability) {
                summary.read_side_capabilities += 1;
            }
            if is_write_side(capability) {
                summary.write_side_capabilities += 1;
            }
            if capability.justification.is_some() {
                summary.justified_capabilities += 1;
            }
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_capabilities == 0
    }

    pub fn has_rws_risk(&self) -> bool {
        self.untrusted_inputs > 0 && self.external_actuations > 0
    }

    pub fn has_same_resource_overlap(&self) -> bool {
        self.overlapping_read_write_pairs > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwsViolation {
    pub untrusted_inputs: Vec<Capability>,
    pub actuations: Vec<Capability>,
    pub message: String,
}

impl fmt::Display for RwsViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for RwsViolation {}

pub fn classify_capability(capability: &Capability) -> CapabilityClassification {
    let flavor = capability
        .flavor
        .unwrap_or_else(|| default_flavor(capability));
    let trust = capability
        .trust
        .unwrap_or_else(|| default_trust(capability));
    let is_input = is_input_capability(capability, flavor);
    let is_untrusted_input = is_input && trust == CapabilityTrust::Untrusted;
    let is_external_actuation = flavor == CapabilityFlavor::Actuation;

    CapabilityClassification {
        flavor,
        trust,
        is_input,
        is_untrusted_input,
        is_external_actuation,
    }
}

pub fn summarize_manifest(capabilities: &[Capability]) -> CapabilityManifestSummary {
    CapabilityManifestSummary::from_capabilities(capabilities)
}

pub fn validate_manifest(capabilities: &[Capability]) -> Result<(), RwsViolation> {
    let mut untrusted_inputs = Vec::new();
    let mut actuations = Vec::new();

    for capability in capabilities {
        let classification = classify_capability(capability);

        if classification.is_untrusted_input {
            push_unique(&mut untrusted_inputs, capability);
        }

        if classification.is_external_actuation {
            push_unique(&mut actuations, capability);
        }
    }

    let has_untrusted_and_actuation = !untrusted_inputs.is_empty() && !actuations.is_empty();
    let has_overlap =
        collect_overlap_violations(capabilities, &mut untrusted_inputs, &mut actuations);

    if has_untrusted_and_actuation || has_overlap {
        let message = if has_overlap {
            "read/write separation violation: manifest contains overlapping read/write capabilities"
                .to_string()
        } else {
            "read/write separation violation: manifest contains untrusted inputs and external actuations; split the agent or insert a trusted channel boundary".to_string()
        };

        Err(RwsViolation {
            untrusted_inputs,
            actuations,
            message,
        })
    } else {
        Ok(())
    }
}

fn default_flavor(capability: &Capability) -> CapabilityFlavor {
    match (capability.category.as_str(), capability.action.as_str()) {
        ("net", "connect")
        | ("fs", "write" | "create" | "delete")
        | ("vault", "write" | "request_lease") => CapabilityFlavor::Actuation,
        ("proc", _) => CapabilityFlavor::Actuation,
        _ => CapabilityFlavor::Internal,
    }
}

fn default_trust(capability: &Capability) -> CapabilityTrust {
    match (capability.category.as_str(), capability.action.as_str()) {
        ("net", "connect") => CapabilityTrust::Untrusted,
        ("net", "listen") => {
            if is_loopback_target(&capability.target) {
                CapabilityTrust::Trusted
            } else {
                CapabilityTrust::Untrusted
            }
        }
        ("fs", "read") => {
            if is_package_internal_target(&capability.target) {
                CapabilityTrust::Trusted
            } else {
                CapabilityTrust::Untrusted
            }
        }
        _ => CapabilityTrust::Trusted,
    }
}

fn is_input_capability(capability: &Capability, flavor: CapabilityFlavor) -> bool {
    match (capability.category.as_str(), capability.action.as_str()) {
        ("net", "connect") => flavor == CapabilityFlavor::Ingestion,
        ("net", "listen") | ("fs", "read") | ("channel", "read") => true,
        _ => flavor == CapabilityFlavor::Ingestion,
    }
}

fn is_loopback_target(target: &str) -> bool {
    target == "localhost"
        || target.starts_with("localhost:")
        || target == "127.0.0.1"
        || target.starts_with("127.0.0.1:")
        || target == "::1"
        || target.starts_with("[::1]:")
}

fn is_package_internal_target(target: &str) -> bool {
    target.starts_with("package:")
        || target.starts_with("pkg:")
        || target.starts_with("./package/")
        || target.starts_with("package/")
}

fn collect_overlap_violations(
    capabilities: &[Capability],
    reads: &mut Vec<Capability>,
    writes: &mut Vec<Capability>,
) -> bool {
    let mut found = false;

    for read in capabilities {
        if !is_read_side(read) {
            continue;
        }

        for write in capabilities {
            if !is_write_side(write) || read.category != write.category {
                continue;
            }

            if resources_overlap(&read.target, &write.target) {
                push_unique(reads, read);
                push_unique(writes, write);
                found = true;
            }
        }
    }

    found
}

fn count_overlap_pairs(capabilities: &[Capability]) -> usize {
    let mut count = 0;

    for read in capabilities {
        if !is_read_side(read) {
            continue;
        }

        for write in capabilities {
            if !is_write_side(write) || read.category != write.category {
                continue;
            }

            if resources_overlap(&read.target, &write.target) {
                count += 1;
            }
        }
    }

    count
}

fn is_read_side(capability: &Capability) -> bool {
    matches!(
        (capability.category.as_str(), capability.action.as_str()),
        ("fs", "read") | ("vault", "read") | ("channel", "read")
    )
}

fn is_write_side(capability: &Capability) -> bool {
    matches!(
        (capability.category.as_str(), capability.action.as_str()),
        ("fs", "write" | "create" | "delete")
            | ("vault", "write" | "request_lease")
            | ("channel", "write")
    )
}

fn resources_overlap(left: &str, right: &str) -> bool {
    left == right || glob_prefix_matches(left, right) || glob_prefix_matches(right, left)
}

fn glob_prefix_matches(pattern: &str, value: &str) -> bool {
    pattern
        .strip_suffix('*')
        .is_some_and(|prefix| value.starts_with(prefix))
}

fn push_unique(collection: &mut Vec<Capability>, capability: &Capability) {
    if !collection.contains(capability) {
        collection.push(capability.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(category: &str, action: &str, target: &str) -> Capability {
        Capability::new(category, action, target)
    }

    #[test]
    fn pure_ingestion_manifest_is_accepted() {
        let capabilities = vec![
            cap("net", "connect", "api.weather.gov:443").with_flavor(CapabilityFlavor::Ingestion),
            cap("channel", "write", "weather-snapshots"),
        ];

        assert!(validate_manifest(&capabilities).is_ok());
    }

    #[test]
    fn pure_actuation_manifest_is_accepted() {
        let capabilities = vec![
            cap("channel", "read", "email-drafts").with_trust(CapabilityTrust::Trusted),
            cap("net", "connect", "smtp.gmail.com:465"),
        ];

        assert!(validate_manifest(&capabilities).is_ok());
    }

    #[test]
    fn mixed_manifest_is_rejected_with_both_lists() {
        let capabilities = vec![
            cap("net", "connect", "imap.gmail.com:993").with_flavor(CapabilityFlavor::Ingestion),
            cap("fs", "write", "/tmp/outbox/message.txt"),
        ];

        let violation = validate_manifest(&capabilities).expect_err("manifest should be rejected");
        assert_eq!(violation.untrusted_inputs.len(), 1);
        assert_eq!(violation.actuations.len(), 1);
        assert_eq!(
            violation.untrusted_inputs[0].identifier(),
            "net:connect:imap.gmail.com:993"
        );
        assert_eq!(
            violation.actuations[0].identifier(),
            "fs:write:/tmp/outbox/message.txt"
        );
    }

    #[test]
    fn fs_read_write_overlap_on_same_path_is_rejected() {
        let capabilities = vec![
            cap("fs", "read", "package:/state/cache.json"),
            cap("fs", "write", "package:/state/cache.json"),
        ];

        let violation = validate_manifest(&capabilities).expect_err("overlap should be rejected");
        assert_eq!(
            violation.untrusted_inputs[0].identifier(),
            "fs:read:package:/state/cache.json"
        );
        assert_eq!(
            violation.actuations[0].identifier(),
            "fs:write:package:/state/cache.json"
        );
    }

    #[test]
    fn fs_read_write_overlap_on_glob_is_rejected() {
        let capabilities = vec![
            cap("fs", "read", "package:/state/*"),
            cap("fs", "write", "package:/state/cache.json"),
        ];

        assert!(validate_manifest(&capabilities).is_err());
    }

    #[test]
    fn fs_read_write_on_disjoint_paths_is_accepted() {
        let capabilities = vec![
            cap("fs", "read", "package:/templates/weather.txt"),
            cap("fs", "write", "/tmp/weather-email.txt"),
        ];

        assert!(validate_manifest(&capabilities).is_ok());
    }

    #[test]
    fn vault_read_write_same_secret_is_rejected() {
        let capabilities = vec![
            cap("vault", "read", "gmail-app-password"),
            cap("vault", "write", "gmail-app-password"),
        ];

        let violation =
            validate_manifest(&capabilities).expect_err("same secret should be rejected");
        assert_eq!(
            violation.untrusted_inputs[0].identifier(),
            "vault:read:gmail-app-password"
        );
        assert_eq!(
            violation.actuations[0].identifier(),
            "vault:write:gmail-app-password"
        );
    }

    #[test]
    fn vault_read_and_write_disjoint_secrets_are_accepted() {
        let capabilities = vec![
            cap("vault", "read", "imap-credentials"),
            cap("vault", "write", "smtp-credentials"),
        ];

        assert!(validate_manifest(&capabilities).is_ok());
    }

    #[test]
    fn channel_read_write_same_channel_is_rejected() {
        let capabilities = vec![
            cap("channel", "read", "weather-snapshots"),
            cap("channel", "write", "weather-snapshots"),
        ];

        let violation =
            validate_manifest(&capabilities).expect_err("same channel should be rejected");
        assert_eq!(
            violation.untrusted_inputs[0].identifier(),
            "channel:read:weather-snapshots"
        );
        assert_eq!(
            violation.actuations[0].identifier(),
            "channel:write:weather-snapshots"
        );
    }

    #[test]
    fn manifest_summary_counts_capability_shape_and_risk() {
        let capabilities = vec![
            cap("net", "connect", "api.weather.gov:443")
                .with_flavor(CapabilityFlavor::Ingestion)
                .with_justification("fetch weather alerts"),
            cap("channel", "write", "weather-snapshots"),
            cap("fs", "read", "package:/state/*"),
            cap("fs", "write", "package:/state/cache.json"),
        ];

        let summary = summarize_manifest(&capabilities);

        assert_eq!(summary.total_capabilities, 4);
        assert_eq!(summary.ingestion_capabilities, 1);
        assert_eq!(summary.actuation_capabilities, 1);
        assert_eq!(summary.internal_capabilities, 2);
        assert_eq!(summary.trusted_capabilities, 3);
        assert_eq!(summary.untrusted_capabilities, 1);
        assert_eq!(summary.input_capabilities, 2);
        assert_eq!(summary.untrusted_inputs, 1);
        assert_eq!(summary.external_actuations, 1);
        assert_eq!(summary.read_side_capabilities, 1);
        assert_eq!(summary.write_side_capabilities, 2);
        assert_eq!(summary.overlapping_read_write_pairs, 1);
        assert_eq!(summary.justified_capabilities, 1);
        assert!(summary.has_rws_risk());
        assert!(summary.has_same_resource_overlap());
        assert!(!summary.is_empty());
    }

    #[test]
    fn empty_manifest_summary_is_empty() {
        let summary = summarize_manifest(&[]);

        assert!(summary.is_empty());
        assert!(!summary.has_rws_risk());
        assert!(!summary.has_same_resource_overlap());
    }

    #[test]
    fn explicit_flavor_overrides_allow_ingestion_only_networks() {
        let capabilities = vec![
            cap("net", "connect", "api.weather.gov:443").with_flavor(CapabilityFlavor::Ingestion),
            cap("net", "connect", "forecast.weather.gov:443")
                .with_flavor(CapabilityFlavor::Ingestion),
        ];

        assert!(validate_manifest(&capabilities).is_ok());
    }

    #[test]
    fn default_net_connect_is_actuation_and_conflicts_with_untrusted_input() {
        let default_connect = cap("net", "connect", "smtp.gmail.com:465");
        let classification = classify_capability(&default_connect);

        assert_eq!(classification.flavor, CapabilityFlavor::Actuation);
        assert_eq!(classification.trust, CapabilityTrust::Untrusted);
        assert!(!classification.is_untrusted_input);
        assert!(classification.is_external_actuation);

        let capabilities = vec![default_connect, cap("net", "listen", "0.0.0.0:8080")];
        let violation =
            validate_manifest(&capabilities).expect_err("untrusted input plus actuation rejected");
        assert_eq!(
            violation.untrusted_inputs[0].identifier(),
            "net:listen:0.0.0.0:8080"
        );
        assert_eq!(
            violation.actuations[0].identifier(),
            "net:connect:smtp.gmail.com:465"
        );
    }
}
