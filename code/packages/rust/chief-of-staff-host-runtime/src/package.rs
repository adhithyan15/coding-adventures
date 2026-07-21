use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_sha256::Sha256Hasher;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

const SIGNATURE_FILE: &str = "SIGNATURE";
const KEY_ID_FILE: &str = "PUBKEY_ID";
const HASH_DOMAIN: &[u8] = b"chief-agent-package-v1\0";

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

    fn get(&self, key_id: &str) -> Option<&TrustedPackageKey> {
        self.keys.get(key_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAgentPackage {
    pub(crate) path: PathBuf,
    pub(crate) digest: [u8; 32],
    pub(crate) key_id: String,
    pub(crate) key_type: PackageKeyType,
    pub(crate) maximum_tier: PrivilegeTier,
}

impl VerifiedAgentPackage {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
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
        .get(&key_id)
        .ok_or_else(|| PackageVerificationError::UntrustedKey(key_id.clone()))?;
    let signature_bytes =
        fs::read(package_path.join(SIGNATURE_FILE)).map_err(PackageVerificationError::Io)?;
    let signature: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| PackageVerificationError::InvalidSignatureLength)?;
    let (digest, files) = hash_package_contents(package_path)?;
    for required in ["manifest.json", "launch.sh"] {
        if !files.contains(required) {
            return Err(PackageVerificationError::MissingRequiredFile(required));
        }
    }
    if !files.iter().any(|path| path.starts_with("code/")) {
        return Err(PackageVerificationError::MissingCode);
    }
    if !coding_adventures_ed25519::verify(&digest, &signature, &key.public_key) {
        return Err(PackageVerificationError::SignatureMismatch);
    }
    Ok(VerifiedAgentPackage {
        path: package_path.to_path_buf(),
        digest,
        key_id,
        key_type: key.key_type,
        maximum_tier: key.maximum_tier,
    })
}

pub fn hash_package_contents(
    package_path: &Path,
) -> Result<([u8; 32], BTreeSet<String>), PackageVerificationError> {
    let mut entries = Vec::new();
    collect_files(package_path, package_path, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256Hasher::new();
    hasher.update(HASH_DOMAIN);
    let mut paths = BTreeSet::new();
    for (path, bytes) in entries {
        paths.insert(path.clone());
        hasher.update(&(path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    Ok((hasher.digest(), paths))
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
    MissingRequiredFile(&'static str),
    MissingCode,
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
            Self::MissingRequiredFile(path) => write!(f, "agent package is missing '{path}'"),
            Self::MissingCode => f.write_str("agent package must contain at least one code file"),
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
        fs::write(
            path.join("launch.sh"),
            b"#!/bin/sh\nexec deno run --no-prompt code/agent.ts\n",
        )
        .unwrap();
        fs::write(path.join("code/agent.ts"), b"console.log('ready');\n").unwrap();
        fs::write(path.join(KEY_ID_FILE), key_id).unwrap();
        let (digest, _) = hash_package_contents(path).unwrap();
        fs::write(path.join(SIGNATURE_FILE), sign(&digest, secret_key)).unwrap();
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
        fs::write(path.join("code/agent.ts"), b"console.log('tampered');\n").unwrap();
        assert!(matches!(
            verify_agent_package(&path, &keyring),
            Err(PackageVerificationError::SignatureMismatch)
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
