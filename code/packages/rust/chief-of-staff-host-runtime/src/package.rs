use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_sha256::Sha256Hasher;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const SIGNATURE_FILE: &str = "SIGNATURE";
const KEY_ID_FILE: &str = "PUBKEY_ID";
const HASH_DOMAIN: &[u8] = b"chief-agent-package-v1\0";
const DENO_ENTRYPOINT: &str = "code/agent_runtime.ts";
const SKILL_ENTRYPOINT: &str = "SKILL.md";
const DENO_FLAGS: &[&str] = &[
    "run",
    "--quiet",
    "--no-prompt",
    "--deny-net",
    "--deny-read",
    "--deny-write",
    "--deny-env",
    "--deny-sys",
    "--deny-run",
    "--deny-ffi",
];

/// Canonical build-time and runtime launch plan for deny-all Deno agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenoLaunchPlan;

impl DenoLaunchPlan {
    pub fn entrypoint_relative() -> &'static str {
        DENO_ENTRYPOINT
    }

    pub fn arguments(entrypoint: &Path) -> Result<Vec<String>, PackageVerificationError> {
        let entrypoint = entrypoint
            .to_str()
            .ok_or_else(|| PackageVerificationError::NonUtf8Path(entrypoint.to_path_buf()))?;
        Ok(DENO_FLAGS
            .iter()
            .map(|flag| (*flag).to_string())
            .chain(std::iter::once(entrypoint.to_string()))
            .collect())
    }

    pub fn launch_script() -> String {
        let arguments = DENO_FLAGS
            .iter()
            .copied()
            .chain(std::iter::once(DENO_ENTRYPOINT))
            .collect::<Vec<_>>()
            .join(" ");
        format!("#!/bin/sh\nexec deno {arguments}\n")
    }

    pub fn write_launch_script(package_path: &Path) -> Result<(), PackageVerificationError> {
        fs::write(package_path.join("launch.sh"), Self::launch_script())
            .map_err(PackageVerificationError::Io)
    }
}

/// Authenticated runtime kind selected by a sealed agent package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPackageRuntime {
    /// A deny-all Deno subprocess using the canonical trusted launch plan.
    Deno,
    /// A zero-code Level 1 skill executed by the trusted in-process runtime.
    Skill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKeyType {
    Production,
    Developer,
    ThirdParty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPackageKey {
    pub key_id: String,
    pub key_type: PackageKeyType,
    pub public_key: [u8; 32],
    pub maximum_tier: PrivilegeTier,
}

impl TrustedPackageKey {
    pub fn new(
        key_id: impl Into<String>,
        key_type: PackageKeyType,
        public_key: [u8; 32],
        maximum_tier: PrivilegeTier,
    ) -> Result<Self, PackageVerificationError> {
        let key_id = key_id.into();
        validate_key_id(&key_id)?;
        if key_type == PackageKeyType::Developer && maximum_tier > PrivilegeTier::Tier1 {
            return Err(PackageVerificationError::DeveloperTierTooHigh(maximum_tier));
        }
        Ok(Self {
            key_id,
            key_type,
            public_key,
            maximum_tier,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct PackageKeyring {
    keys: BTreeMap<String, TrustedPackageKey>,
}

impl PackageKeyring {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trust(&mut self, key: TrustedPackageKey) -> Result<(), PackageVerificationError> {
        if self.keys.insert(key.key_id.clone(), key).is_some() {
            return Err(PackageVerificationError::DuplicateKeyId);
        }
        Ok(())
    }

    /// Return one trusted public key by its stable package key identifier.
    pub fn trusted_key(&self, key_id: &str) -> Option<&TrustedPackageKey> {
        self.keys.get(key_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAgentPackage {
    pub(crate) path: PathBuf,
    pub(crate) digest: [u8; 32],
    pub(crate) manifest_bytes: Vec<u8>,
    pub(crate) key_id: String,
    pub(crate) key_type: PackageKeyType,
    pub(crate) maximum_tier: PrivilegeTier,
    pub(crate) runtime: AgentPackageRuntime,
    pub(crate) skill_source_bytes: Option<Vec<u8>>,
}

impl VerifiedAgentPackage {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Borrow the exact `manifest.json` bytes covered by the verified digest.
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn key_type(&self) -> PackageKeyType {
        self.key_type
    }

    pub fn maximum_tier(&self) -> PrivilegeTier {
        self.maximum_tier
    }

    /// Return the authenticated runtime kind selected by the package layout.
    pub fn runtime(&self) -> AgentPackageRuntime {
        self.runtime
    }

    /// Borrow the exact authenticated `SKILL.md` bytes for a Level 1 package.
    pub fn skill_source_bytes(&self) -> Option<&[u8]> {
        self.skill_source_bytes.as_deref()
    }
}

pub fn verify_agent_package(
    package_path: &Path,
    keyring: &PackageKeyring,
) -> Result<VerifiedAgentPackage, PackageVerificationError> {
    let metadata = fs::symlink_metadata(package_path).map_err(PackageVerificationError::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PackageVerificationError::InvalidPackageRoot);
    }
    let key_id = fs::read_to_string(package_path.join(KEY_ID_FILE))
        .map_err(PackageVerificationError::Io)?
        .trim()
        .to_string();
    validate_key_id(&key_id)?;
    let key = keyring
        .trusted_key(&key_id)
        .ok_or_else(|| PackageVerificationError::UntrustedKey(key_id.clone()))?;
    let signature_bytes =
        fs::read(package_path.join(SIGNATURE_FILE)).map_err(PackageVerificationError::Io)?;
    let signature: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| PackageVerificationError::InvalidSignatureLength)?;
    let entries = package_contents(package_path)?;
    let (digest, _) = hash_contents(&entries);
    let (runtime, skill_source_bytes) = validate_package_layout(&entries)?;
    if !coding_adventures_ed25519::verify(&digest, &signature, &key.public_key) {
        return Err(PackageVerificationError::SignatureMismatch);
    }
    Ok(VerifiedAgentPackage {
        path: package_path.to_path_buf(),
        digest,
        manifest_bytes: file_bytes(&entries, "manifest.json")
            .expect("required manifest must be present in collected package files")
            .to_vec(),
        key_id,
        key_type: key.key_type,
        maximum_tier: key.maximum_tier,
        runtime,
        skill_source_bytes,
    })
}

/// Validate and sign one already-populated package directory.
///
/// Authentication metadata is written only after the unsigned contents match
/// one supported runtime layout. Existing authentication files are never
/// overwritten.
pub fn sign_agent_package(
    package_path: &Path,
    key_id: &str,
    secret_key: &[u8; 64],
) -> Result<[u8; 32], PackageVerificationError> {
    validate_key_id(key_id)?;
    let metadata = fs::symlink_metadata(package_path).map_err(PackageVerificationError::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PackageVerificationError::InvalidPackageRoot);
    }
    for authentication_file in [KEY_ID_FILE, SIGNATURE_FILE] {
        match fs::symlink_metadata(package_path.join(authentication_file)) {
            Ok(_) => return Err(PackageVerificationError::AlreadySigned),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(PackageVerificationError::Io(error)),
        }
    }
    let entries = package_contents(package_path)?;
    validate_package_layout(&entries)?;
    let (digest, _) = hash_contents(&entries);
    let signature = coding_adventures_ed25519::sign(&digest, secret_key);
    write_new_authentication_file(package_path.join(SIGNATURE_FILE), &signature)?;
    if let Err(error) =
        write_new_authentication_file(package_path.join(KEY_ID_FILE), key_id.as_bytes())
    {
        let _ = fs::remove_file(package_path.join(SIGNATURE_FILE));
        return Err(error);
    }
    Ok(digest)
}

fn write_new_authentication_file(
    path: PathBuf,
    bytes: &[u8],
) -> Result<(), PackageVerificationError> {
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(PackageVerificationError::AlreadySigned);
        }
        Err(error) => return Err(PackageVerificationError::Io(error)),
    };
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(PackageVerificationError::Io(error));
    }
    Ok(())
}

pub fn hash_package_contents(
    package_path: &Path,
) -> Result<([u8; 32], BTreeSet<String>), PackageVerificationError> {
    let entries = package_contents(package_path)?;
    Ok(hash_contents(&entries))
}

fn package_contents(
    package_path: &Path,
) -> Result<Vec<(String, Vec<u8>)>, PackageVerificationError> {
    let mut entries = Vec::new();
    collect_files(package_path, package_path, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

fn hash_contents(entries: &[(String, Vec<u8>)]) -> ([u8; 32], BTreeSet<String>) {
    let mut hasher = Sha256Hasher::new();
    hasher.update(HASH_DOMAIN);
    let mut paths = BTreeSet::new();
    for (path, bytes) in entries {
        paths.insert(path.clone());
        hasher.update(&(path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    (hasher.digest(), paths)
}

fn validate_package_layout(
    entries: &[(String, Vec<u8>)],
) -> Result<(AgentPackageRuntime, Option<Vec<u8>>), PackageVerificationError> {
    let files = entries
        .iter()
        .map(|(path, _)| path.as_str())
        .collect::<BTreeSet<_>>();
    if !files.contains("manifest.json") {
        return Err(PackageVerificationError::MissingRequiredFile(
            "manifest.json",
        ));
    }
    if let Some(skill_source) = file_bytes(entries, SKILL_ENTRYPOINT) {
        if let Some(unexpected) = files
            .iter()
            .find(|path| !matches!(**path, "manifest.json" | SKILL_ENTRYPOINT))
        {
            return Err(PackageVerificationError::UnexpectedSkillPackageFile(
                (*unexpected).to_string(),
            ));
        }
        return Ok((AgentPackageRuntime::Skill, Some(skill_source.to_vec())));
    }
    if !files.contains("launch.sh") {
        return Err(PackageVerificationError::MissingRequiredFile("launch.sh"));
    }
    if !files.iter().any(|path| path.starts_with("code/")) {
        return Err(PackageVerificationError::MissingCode);
    }
    if !files.contains(DENO_ENTRYPOINT) {
        return Err(PackageVerificationError::MissingDenoEntrypoint);
    }
    let launch_script = file_bytes(entries, "launch.sh")
        .expect("required launch script must be present in collected package files");
    if launch_script != DenoLaunchPlan::launch_script().as_bytes() {
        return Err(PackageVerificationError::UntrustedLaunchScript);
    }
    Ok((AgentPackageRuntime::Deno, None))
}

fn file_bytes<'a>(entries: &'a [(String, Vec<u8>)], path: &str) -> Option<&'a [u8]> {
    entries
        .iter()
        .find(|(candidate, _)| candidate == path)
        .map(|(_, bytes)| bytes.as_slice())
}

fn collect_files(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), PackageVerificationError> {
    for entry in fs::read_dir(current).map_err(PackageVerificationError::Io)? {
        let entry = entry.map_err(PackageVerificationError::Io)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(PackageVerificationError::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(PackageVerificationError::Symlink(path));
        }
        if metadata.is_dir() {
            collect_files(root, &path, entries)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("walked package file must remain under root");
            let relative = relative
                .to_str()
                .ok_or_else(|| PackageVerificationError::NonUtf8Path(path.clone()))?
                .replace('\\', "/");
            if relative != SIGNATURE_FILE && relative != KEY_ID_FILE {
                entries.push((
                    relative,
                    fs::read(path).map_err(PackageVerificationError::Io)?,
                ));
            }
        }
    }
    Ok(())
}

fn validate_key_id(key_id: &str) -> Result<(), PackageVerificationError> {
    if key_id.is_empty()
        || key_id.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '-')
        })
    {
        return Err(PackageVerificationError::InvalidKeyId);
    }
    Ok(())
}

#[derive(Debug)]
pub enum PackageVerificationError {
    Io(std::io::Error),
    InvalidPackageRoot,
    InvalidKeyId,
    DuplicateKeyId,
    DeveloperTierTooHigh(PrivilegeTier),
    UntrustedKey(String),
    InvalidSignatureLength,
    AlreadySigned,
    MissingRequiredFile(&'static str),
    MissingCode,
    MissingDenoEntrypoint,
    UntrustedLaunchScript,
    UnexpectedSkillPackageFile(String),
    Symlink(PathBuf),
    NonUtf8Path(PathBuf),
    SignatureMismatch,
}

impl Display for PackageVerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "package I/O failed: {error}"),
            Self::InvalidPackageRoot => f.write_str("agent package root must be a real directory"),
            Self::InvalidKeyId => f.write_str("package key id must be a non-empty ASCII label"),
            Self::DuplicateKeyId => f.write_str("package key id is already trusted"),
            Self::DeveloperTierTooHigh(tier) => {
                write!(f, "developer package keys cannot authorize {tier}")
            }
            Self::UntrustedKey(key_id) => write!(f, "package key '{key_id}' is not trusted"),
            Self::InvalidSignatureLength => f.write_str("package signature must be 64 raw bytes"),
            Self::AlreadySigned => f.write_str("agent package authentication files already exist"),
            Self::MissingRequiredFile(path) => write!(f, "agent package is missing '{path}'"),
            Self::MissingCode => f.write_str("agent package must contain at least one code file"),
            Self::MissingDenoEntrypoint => {
                f.write_str("agent package is missing 'code/agent_runtime.ts'")
            }
            Self::UntrustedLaunchScript => {
                f.write_str("agent package launch.sh does not match the trusted deny-all plan")
            }
            Self::UnexpectedSkillPackageFile(path) => {
                write!(
                    f,
                    "SKILL.md package contains unexpected signed file '{path}'"
                )
            }
            Self::Symlink(path) => write!(
                f,
                "agent package cannot contain symlink '{}'",
                path.display()
            ),
            Self::NonUtf8Path(path) => {
                write!(f, "agent package path is not UTF-8: '{}'", path.display())
            }
            Self::SignatureMismatch => f.write_str("agent package signature did not verify"),
        }
    }
}

impl Error for PackageVerificationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_ed25519::{generate_keypair, sign};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn package_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "chief-agent-package-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_signed_package(path: &Path, key_id: &str, secret_key: &[u8; 64]) {
        fs::create_dir_all(path.join("code")).unwrap();
        fs::write(path.join("manifest.json"), b"{\"runtime\":\"typescript\"}").unwrap();
        DenoLaunchPlan::write_launch_script(path).unwrap();
        fs::write(
            path.join("code/agent_runtime.ts"),
            b"console.log('ready');\n",
        )
        .unwrap();
        fs::write(path.join(KEY_ID_FILE), key_id).unwrap();
        let (digest, _) = hash_package_contents(path).unwrap();
        fs::write(path.join(SIGNATURE_FILE), sign(&digest, secret_key)).unwrap();
    }

    fn write_signed_skill_package(path: &Path, key_id: &str, secret_key: &[u8; 64]) {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("manifest.json"), b"{\"runtime\":\"skill\"}").unwrap();
        fs::write(
            path.join(SKILL_ENTRYPOINT),
            b"# Weather\n\nReport friendly forecasts.\n\n## Capabilities needed\n- none\n",
        )
        .unwrap();
        sign_agent_package(path, key_id, secret_key).unwrap();
    }

    #[test]
    fn verifies_signed_package_and_rejects_byte_tampering() {
        let path = package_dir("tamper");
        let (public_key, secret_key) = generate_keypair(&[7; 32]);
        write_signed_package(&path, "prod-1", &secret_key);
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "prod-1",
                    PackageKeyType::Production,
                    public_key,
                    PrivilegeTier::Tier3,
                )
                .unwrap(),
            )
            .unwrap();

        let verified = verify_agent_package(&path, &keyring).unwrap();
        assert_eq!(verified.key_id(), "prod-1");
        assert_eq!(verified.key_type(), PackageKeyType::Production);
        assert_eq!(verified.runtime(), AgentPackageRuntime::Deno);
        assert_eq!(verified.skill_source_bytes(), None);
        assert_eq!(verified.manifest_bytes(), b"{\"runtime\":\"typescript\"}");
        fs::write(path.join("code/agent.ts"), b"console.log('tampered');\n").unwrap();
        assert!(matches!(
            verify_agent_package(&path, &keyring),
            Err(PackageVerificationError::SignatureMismatch)
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn verifies_signed_skill_package_without_a_deno_entrypoint() {
        let path = package_dir("skill");
        let (public_key, secret_key) = generate_keypair(&[41; 32]);
        write_signed_skill_package(&path, "dev-skill", &secret_key);
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "dev-skill",
                    PackageKeyType::Developer,
                    public_key,
                    PrivilegeTier::Tier1,
                )
                .unwrap(),
            )
            .unwrap();

        let verified = verify_agent_package(&path, &keyring).unwrap();
        assert_eq!(verified.runtime(), AgentPackageRuntime::Skill);
        assert!(verified
            .skill_source_bytes()
            .unwrap()
            .starts_with(b"# Weather\n"));
        assert!(matches!(
            sign_agent_package(&path, "dev-skill", &secret_key),
            Err(PackageVerificationError::AlreadySigned)
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn skill_package_rejects_any_executable_file() {
        let path = package_dir("skill-extra-code");
        fs::create_dir_all(path.join("code")).unwrap();
        fs::write(path.join("manifest.json"), b"{}").unwrap();
        fs::write(path.join(SKILL_ENTRYPOINT), b"# Skill\n").unwrap();
        fs::write(path.join(DENO_ENTRYPOINT), b"console.log('no');\n").unwrap();
        let (_, secret_key) = generate_keypair(&[43; 32]);
        assert!(matches!(
            sign_agent_package(&path, "dev-skill", &secret_key),
            Err(PackageVerificationError::UnexpectedSkillPackageFile(path))
                if path == DENO_ENTRYPOINT
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn package_hash_is_stable_across_directory_locations() {
        let first = package_dir("stable-a");
        let second = package_dir("stable-b");
        let (_, secret_key) = generate_keypair(&[9; 32]);
        write_signed_package(&first, "dev-1", &secret_key);
        write_signed_package(&second, "dev-1", &secret_key);
        assert_eq!(
            hash_package_contents(&first).unwrap().0,
            hash_package_contents(&second).unwrap().0
        );
        fs::remove_dir_all(first).unwrap();
        fs::remove_dir_all(second).unwrap();
    }

    #[test]
    fn trusted_launch_plan_is_literal_and_rejects_script_tampering() {
        let path = package_dir("launch-plan");
        let (public_key, secret_key) = generate_keypair(&[31; 32]);
        write_signed_package(&path, "prod-launch", &secret_key);
        let mut keyring = PackageKeyring::new();
        keyring
            .trust(
                TrustedPackageKey::new(
                    "prod-launch",
                    PackageKeyType::Production,
                    public_key,
                    PrivilegeTier::Tier3,
                )
                .unwrap(),
            )
            .unwrap();

        let script = DenoLaunchPlan::launch_script();
        for flag in DENO_FLAGS {
            assert!(script.contains(flag));
        }
        assert!(!script.contains('$'));
        verify_agent_package(&path, &keyring).unwrap();

        fs::write(
            path.join("launch.sh"),
            "#!/bin/sh\nexec deno run --allow-net code/agent_runtime.ts\n",
        )
        .unwrap();
        assert!(matches!(
            verify_agent_package(&path, &keyring),
            Err(PackageVerificationError::UntrustedLaunchScript)
        ));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn developer_key_cannot_be_trusted_above_tier_one() {
        let (public_key, _) = generate_keypair(&[11; 32]);
        assert!(matches!(
            TrustedPackageKey::new(
                "dev-1",
                PackageKeyType::Developer,
                public_key,
                PrivilegeTier::Tier2,
            ),
            Err(PackageVerificationError::DeveloperTierTooHigh(
                PrivilegeTier::Tier2
            ))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn package_hash_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let path = package_dir("symlink");
        fs::create_dir_all(path.join("code")).unwrap();
        fs::write(path.join("outside.ts"), b"secret").unwrap();
        symlink(path.join("outside.ts"), path.join("code/agent.ts")).unwrap();
        assert!(matches!(
            hash_package_contents(&path),
            Err(PackageVerificationError::Symlink(_))
        ));
        fs::remove_dir_all(path).unwrap();
    }
}
