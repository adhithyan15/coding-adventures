//! Verified package discovery and registration candidates for D18 agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_agent_manifest::{parse_manifest, AgentManifest, ManifestError};
use chief_of_staff_host_runtime::{
    verify_agent_package, PackageKeyring, PackageVerificationError, VerifiedAgentPackage,
};
use chief_of_staff_service_registry::{
    HostName, HostRegistration, PackagePath, RegistryError, RestartPolicy,
};
use chief_of_staff_tool_api::PrivilegeTier;
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One authenticated package plus the inert registration it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredAgent {
    package: VerifiedAgentPackage,
    manifest: AgentManifest,
    registration: HostRegistration,
}

impl DiscoveredAgent {
    /// Borrow the cryptographically verified sealed package.
    pub fn package(&self) -> &VerifiedAgentPackage {
        &self.package
    }
    /// Borrow the parsed, authenticated schema-v1 manifest.
    pub fn manifest(&self) -> &AgentManifest {
        &self.manifest
    }
    /// Borrow the immutable candidate for explicit service registration.
    pub fn registration(&self) -> &HostRegistration {
        &self.registration
    }
    /// Consume this result and return its registration candidate.
    pub fn into_registration(self) -> HostRegistration {
        self.registration
    }
}

/// Fail-closed package discovery error.
#[derive(Debug)]
pub enum DiscoveryError {
    /// The scan root is not a real, non-symlink directory.
    InvalidPackagesDirectory(PathBuf),
    /// Filesystem access failed.
    Io {
        /// Path being accessed.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// Package signature or sealed layout verification failed.
    Package {
        /// Candidate package path.
        path: PathBuf,
        /// Verification failure.
        source: PackageVerificationError,
    },
    /// Authenticated manifest bytes are not UTF-8.
    NonUtf8Manifest(PathBuf),
    /// The authenticated manifest is invalid or incompatible.
    Manifest {
        /// Candidate package path.
        path: PathBuf,
        /// Strict manifest error.
        source: ManifestError,
    },
    /// The manifest tier exceeds the signing key ceiling.
    TierExceeded {
        /// Stable manifest agent identity.
        agent: String,
        /// Manifest-requested tier number.
        requested: u8,
        /// Maximum tier authorized by the signing key.
        maximum: PrivilegeTier,
    },
    /// The canonical package path is not portable UTF-8.
    NonUtf8Path(PathBuf),
    /// A registry identity or path rejected the manifest data.
    Registry {
        /// Canonical candidate package path.
        path: PathBuf,
        /// Registry value validation failure.
        source: RegistryError,
    },
    /// Two candidates declare the same agent identity.
    DuplicateAgent(String),
}

impl Display for DiscoveryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPackagesDirectory(p) => {
                write!(f, "invalid agent packages directory: {}", p.display())
            }
            Self::Io { path, source } => write!(
                f,
                "agent discovery I/O failed at '{}': {source}",
                path.display()
            ),
            Self::Package { path, source } => write!(
                f,
                "agent package verification failed at '{}': {source}",
                path.display()
            ),
            Self::NonUtf8Manifest(p) => {
                write!(f, "authenticated manifest is not UTF-8: {}", p.display())
            }
            Self::Manifest { path, source } => write!(
                f,
                "authenticated manifest is invalid at '{}': {source}",
                path.display()
            ),
            Self::TierExceeded {
                agent,
                requested,
                maximum,
            } => write!(
                f,
                "agent '{agent}' requests tier {requested} above signing-key ceiling {maximum}"
            ),
            Self::NonUtf8Path(p) => write!(f, "agent package path is not UTF-8: {}", p.display()),
            Self::Registry { path, source } => write!(
                f,
                "agent registration is invalid at '{}': {source}",
                path.display()
            ),
            Self::DuplicateAgent(agent) => {
                write!(f, "duplicate discovered agent identity: {agent}")
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Verify and inspect one operator-selected package without registering it.
pub fn inspect_agent_package(
    path: &Path,
    keyring: &PackageKeyring,
) -> Result<DiscoveredAgent, DiscoveryError> {
    let package =
        verify_agent_package(path, keyring).map_err(|source| DiscoveryError::Package {
            path: path.to_path_buf(),
            source,
        })?;
    let source = std::str::from_utf8(package.manifest_bytes())
        .map_err(|_| DiscoveryError::NonUtf8Manifest(path.to_path_buf()))?;
    let manifest = parse_manifest(source).map_err(|source| DiscoveryError::Manifest {
        path: path.to_path_buf(),
        source,
    })?;
    if manifest.privilege_tier > tier_number(package.maximum_tier()) {
        return Err(DiscoveryError::TierExceeded {
            agent: manifest.agent.clone(),
            requested: manifest.privilege_tier,
            maximum: package.maximum_tier(),
        });
    }
    let canonical = fs::canonicalize(path).map_err(|source| DiscoveryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let portable = canonical
        .to_str()
        .ok_or_else(|| DiscoveryError::NonUtf8Path(canonical.clone()))?;
    let host_name =
        HostName::new(manifest.agent.clone()).map_err(|source| DiscoveryError::Registry {
            path: canonical.clone(),
            source,
        })?;
    let package_path = PackagePath::new(portable).map_err(|source| DiscoveryError::Registry {
        path: canonical.clone(),
        source,
    })?;
    let registration = HostRegistration::new(
        host_name,
        package_path,
        package.digest(),
        restart_policy(&manifest.restart_policy),
    );
    Ok(DiscoveredAgent {
        package,
        manifest,
        registration,
    })
}

/// Scan immediate `.agent` children and return one complete stable snapshot.
///
/// Non-package siblings are ignored. One invalid or duplicate candidate fails
/// the entire scan so callers cannot accidentally act on a partial catalog.
pub fn discover_agent_packages(
    directory: &Path,
    keyring: &PackageKeyring,
) -> Result<Vec<DiscoveredAgent>, DiscoveryError> {
    let metadata = fs::symlink_metadata(directory).map_err(|source| DiscoveryError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(DiscoveryError::InvalidPackagesDirectory(
            directory.to_path_buf(),
        ));
    }
    let entries = fs::read_dir(directory).map_err(|source| DiscoveryError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| DiscoveryError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(".agent"))
        {
            candidates.push(entry.path());
        }
    }
    candidates.sort();
    let mut identities = BTreeSet::new();
    let mut agents = Vec::with_capacity(candidates.len());
    for path in candidates {
        let agent = inspect_agent_package(&path, keyring)?;
        if !identities.insert(agent.manifest.agent.clone()) {
            return Err(DiscoveryError::DuplicateAgent(agent.manifest.agent));
        }
        agents.push(agent);
    }
    agents.sort_by(|a, b| a.manifest.agent.cmp(&b.manifest.agent));
    Ok(agents)
}

fn tier_number(tier: PrivilegeTier) -> u8 {
    match tier {
        PrivilegeTier::Tier0 => 0,
        PrivilegeTier::Tier1 => 1,
        PrivilegeTier::Tier2 => 2,
        PrivilegeTier::Tier3 => 3,
    }
}

fn restart_policy(value: &str) -> RestartPolicy {
    match value {
        "always" => RestartPolicy::Always,
        "never" => RestartPolicy::Never,
        _ => RestartPolicy::OnFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_host_runtime::{
        hash_package_contents, DenoLaunchPlan, PackageKeyType, TrustedPackageKey,
    };
    use coding_adventures_ed25519::{generate_keypair, sign};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        keyring: PackageKeyring,
        secret: [u8; 64],
    }
    impl Fixture {
        fn new(max: PrivilegeTier) -> Self {
            let root = std::env::temp_dir().join(format!(
                "chief-discovery-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            let (public, secret) = generate_keypair(&[51; 32]);
            let mut keyring = PackageKeyring::new();
            keyring
                .trust(
                    TrustedPackageKey::new("test", PackageKeyType::Production, public, max)
                        .unwrap(),
                )
                .unwrap();
            Self {
                root,
                keyring,
                secret,
            }
        }
        fn package(&self, dir: &str, agent: &str, tier: u8) -> PathBuf {
            let path = self.root.join(dir);
            fs::create_dir_all(path.join("code")).unwrap();
            fs::write(path.join("code/agent_runtime.ts"), "console.log('ok');\n").unwrap();
            fs::write(path.join("launch.sh"), DenoLaunchPlan::launch_script()).unwrap();
            fs::write(path.join("PUBKEY_ID"), "test\n").unwrap();
            fs::write(path.join("manifest.json"), manifest(agent, tier)).unwrap();
            let digest = hash_package_contents(&path).unwrap().0;
            fs::write(path.join("SIGNATURE"), sign(&digest, &self.secret)).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn manifest(agent: &str, tier: u8) -> String {
        format!(
            r#"{{"version":1,"agent":"{agent}","description":"Verified package discovery test agent.","privilege_tier":{tier},"channels":{{"reads":["agent-input"],"writes":["agent-output"]}},"capabilities":[],"restart_policy":"always","justification":"Uses only declared encrypted channels."}}"#
        )
    }

    #[test]
    fn explicit_candidate_uses_authenticated_identity() {
        let f = Fixture::new(PrivilegeTier::Tier2);
        let path = f.package("weather.agent", "weather-agent", 2);
        let found = inspect_agent_package(&path, &f.keyring).unwrap();
        assert_eq!(found.registration().host_name().as_str(), "weather-agent");
        assert_eq!(
            found.registration().package_hash(),
            &found.package().digest()
        );
        assert!(Path::new(found.registration().package_path().as_str()).is_absolute());
    }
    #[test]
    fn scan_is_sorted_and_ignores_siblings() {
        let f = Fixture::new(PrivilegeTier::Tier3);
        f.package("z.agent", "zulu-agent", 0);
        f.package("a.agent", "alpha-agent", 0);
        fs::write(f.root.join("README"), "ignore").unwrap();
        let names = discover_agent_packages(&f.root, &f.keyring)
            .unwrap()
            .into_iter()
            .map(|a| a.manifest.agent)
            .collect::<Vec<_>>();
        assert_eq!(names, ["alpha-agent", "zulu-agent"]);
    }
    #[test]
    fn scan_fails_on_one_invalid_candidate() {
        let f = Fixture::new(PrivilegeTier::Tier3);
        f.package("ok.agent", "valid-agent", 0);
        fs::create_dir(f.root.join("bad.agent")).unwrap();
        assert!(matches!(
            discover_agent_packages(&f.root, &f.keyring),
            Err(DiscoveryError::Package { .. })
        ));
    }
    #[test]
    fn signing_key_tier_is_enforced() {
        let f = Fixture::new(PrivilegeTier::Tier1);
        let path = f.package("high.agent", "high-agent", 2);
        assert!(matches!(
            inspect_agent_package(&path, &f.keyring),
            Err(DiscoveryError::TierExceeded { requested: 2, .. })
        ));
    }
    #[test]
    fn duplicate_identities_fail_the_snapshot() {
        let f = Fixture::new(PrivilegeTier::Tier3);
        f.package("one.agent", "same-agent", 0);
        f.package("two.agent", "same-agent", 0);
        assert!(matches!(
            discover_agent_packages(&f.root, &f.keyring),
            Err(DiscoveryError::DuplicateAgent(_))
        ));
    }
}
