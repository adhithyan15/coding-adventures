//! Build and load sealed D18 Level 1 `SKILL.md` agent packages.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_host_runtime::{
    sign_agent_package, AgentPackageRuntime, PackageVerificationError, VerifiedAgentPackage,
};
use chief_of_staff_skill_parser::{parse_skill, ParsedSkill, SkillParseError};
use core::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

/// Build one signed Level 1 package in a new caller-selected directory.
///
/// The target must not exist and is never overwritten. If a write or signing
/// step fails, only the directory created by this call is removed.
pub fn build_signed_skill_package(
    target: &Path,
    source: &str,
    key_id: &str,
    secret_key: &[u8; 64],
) -> Result<ParsedSkill, SkillPackageError> {
    let skill = parse_skill(source).map_err(SkillPackageError::Parse)?;
    let manifest = skill
        .manifest
        .to_json()
        .map_err(|error| SkillPackageError::Manifest(error.to_string()))?;
    match fs::create_dir(target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(SkillPackageError::TargetExists(target.to_path_buf()));
        }
        Err(error) => return Err(SkillPackageError::Io(error)),
    }
    let result = (|| {
        fs::write(target.join("SKILL.md"), source).map_err(SkillPackageError::Io)?;
        fs::write(target.join("manifest.json"), manifest).map_err(SkillPackageError::Io)?;
        sign_agent_package(target, key_id, secret_key).map_err(SkillPackageError::Package)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(target);
        return Err(error);
    }
    Ok(skill)
}

/// Parse the exact authenticated instructions from a verified Level 1 package.
///
/// The signed manifest must be the canonical manifest generated from those
/// instructions. No package path is re-read after verification.
pub fn load_verified_skill(
    package: &VerifiedAgentPackage,
) -> Result<ParsedSkill, SkillPackageError> {
    if package.runtime() != AgentPackageRuntime::Skill {
        return Err(SkillPackageError::WrongRuntime(package.runtime()));
    }
    let source = std::str::from_utf8(
        package
            .skill_source_bytes()
            .expect("verified Skill packages retain authenticated source bytes"),
    )
    .map_err(|_| SkillPackageError::NonUtf8Skill)?;
    let skill = parse_skill(source).map_err(SkillPackageError::Parse)?;
    let canonical_manifest = skill
        .manifest
        .to_json()
        .map_err(|error| SkillPackageError::Manifest(error.to_string()))?;
    if canonical_manifest.as_bytes() != package.manifest_bytes() {
        return Err(SkillPackageError::ManifestMismatch);
    }
    Ok(skill)
}

/// Stable failures from Level 1 package construction and loading.
#[derive(Debug)]
pub enum SkillPackageError {
    /// The caller-selected output path already exists.
    TargetExists(PathBuf),
    /// Filesystem access failed while creating a new package.
    Io(std::io::Error),
    /// The skill document is malformed.
    Parse(SkillParseError),
    /// Canonical manifest serialization failed.
    Manifest(String),
    /// Shared package layout validation or signing failed.
    Package(PackageVerificationError),
    /// A non-Level 1 package was passed to the Level 1 loader.
    WrongRuntime(AgentPackageRuntime),
    /// The authenticated `SKILL.md` is not UTF-8.
    NonUtf8Skill,
    /// Signed instructions and signed generated policy disagree.
    ManifestMismatch,
}

impl Display for SkillPackageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetExists(path) => {
                write!(formatter, "skill package target already exists: {}", path.display())
            }
            Self::Io(error) => write!(formatter, "skill package I/O failed: {error}"),
            Self::Parse(error) => write!(formatter, "invalid packaged skill: {error}"),
            Self::Manifest(error) => write!(formatter, "manifest serialization failed: {error}"),
            Self::Package(error) => write!(formatter, "package signing failed: {error}"),
            Self::WrongRuntime(runtime) => {
                write!(formatter, "expected Skill package runtime, found {runtime:?}")
            }
            Self::NonUtf8Skill => formatter.write_str("authenticated SKILL.md is not UTF-8"),
            Self::ManifestMismatch => formatter.write_str(
                "authenticated manifest does not match the manifest derived from SKILL.md",
            ),
        }
    }
}

impl std::error::Error for SkillPackageError {}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_host_runtime::{
        verify_agent_package, DenoLaunchPlan, PackageKeyType, PackageKeyring, TrustedPackageKey,
    };
    use chief_of_staff_tool_api::PrivilegeTier;
    use coding_adventures_ed25519::generate_keypair;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SKILL: &str = "---\nagent: weather-reporter\ndescription: Reports friendly forecasts for requested cities.\nprivilege_tier: 0\nreads: [weather-requests]\nwrites: [weather-reports]\nmessage_schema_versions: [weather-requests=1, weather-reports=1]\n---\n# Weather Reporter\n\nReport a brief forecast for the requested city.\n\n## Capabilities needed\n- none\n";

    fn package_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "chief-skill-package-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn builds_verifies_and_loads_one_skill_only_package() {
        let path = package_dir("round-trip");
        let (public_key, secret_key) = generate_keypair(&[51; 32]);
        let built = build_signed_skill_package(&path, SKILL, "dev-weather", &secret_key).unwrap();
        assert_eq!(built.manifest.agent, "weather-reporter");
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-weather",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        let package = verify_agent_package(&path, &keyring).unwrap();
        assert_eq!(package.runtime(), AgentPackageRuntime::Skill);
        assert_eq!(load_verified_skill(&package).unwrap(), built);
        let signed_files = fs::read_dir(&path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            signed_files,
            ["PUBKEY_ID", "SIGNATURE", "SKILL.md", "manifest.json"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn never_overwrites_an_existing_target() {
        let path = package_dir("existing");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep.txt"), "preserve").unwrap();
        let (_, secret_key) = generate_keypair(&[53; 32]);
        assert!(matches!(
            build_signed_skill_package(&path, SKILL, "dev-weather", &secret_key),
            Err(SkillPackageError::TargetExists(found)) if found == path
        ));
        assert_eq!(fs::read_to_string(path.join("keep.txt")).unwrap(), "preserve");
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn rejects_a_deno_package_at_the_skill_loader_boundary() {
        let path = package_dir("deno");
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("code")).unwrap();
        fs::write(path.join("manifest.json"), "{}").unwrap();
        DenoLaunchPlan::write_launch_script(&path).unwrap();
        fs::write(path.join("code/agent_runtime.ts"), "console.log('ready');\n").unwrap();
        let (public_key, secret_key) = generate_keypair(&[55; 32]);
        sign_agent_package(&path, "dev-deno", &secret_key).unwrap();
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-deno",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        let package = verify_agent_package(&path, &keyring).unwrap();
        assert!(matches!(
            load_verified_skill(&package),
            Err(SkillPackageError::WrongRuntime(AgentPackageRuntime::Deno))
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn rejects_signed_policy_that_does_not_match_signed_instructions() {
        let path = package_dir("manifest-mismatch");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("SKILL.md"), SKILL).unwrap();
        fs::write(path.join("manifest.json"), "{}\n").unwrap();
        let (public_key, secret_key) = generate_keypair(&[57; 32]);
        sign_agent_package(&path, "dev-mismatch", &secret_key).unwrap();
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-mismatch",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();
        let package = verify_agent_package(&path, &keyring).unwrap();
        assert!(matches!(
            load_verified_skill(&package),
            Err(SkillPackageError::ManifestMismatch)
        ));
        fs::remove_dir_all(path).unwrap();
    }
}
