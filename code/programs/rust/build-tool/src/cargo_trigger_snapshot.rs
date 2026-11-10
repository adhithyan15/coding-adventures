//! Pure Cargo test-trigger closure over immutable snapshots.
//!
//! The native Go build tool discovers Cargo manifests, checks exact filesystem
//! identity, reads the orphan ledger, and inspects `.cargo/config*`.  Those are
//! host-authority operations.  This module deliberately starts one boundary
//! later: a caller supplies closed package, crate, configuration, and exemption
//! snapshots, and this code performs only lexical path reduction and graph
//! closure.
//!
//! Keeping that line sharp matters.  A test-trigger edge is not a build-graph
//! edge: Cargo permits development cycles, and selecting a dependent package's
//! tests must not make that package's library dependents look changed.  The
//! result here is therefore a separate sorted relation which the already-pure
//! selector can follow exactly one hop.

use std::collections::{BTreeMap, BTreeSet};

/// The largest closed package universe accepted by one evaluation.
pub const MAX_CARGO_TRIGGER_PACKAGES: usize = 4_096;
/// The largest captured crate universe accepted by one evaluation.
pub const MAX_CARGO_TRIGGER_CRATES: usize = 16_384;
/// The combined ceiling for root trigger paths and transitive link paths.
pub const MAX_CARGO_TRIGGER_REFERENCES: usize = 16_384;
/// The largest immutable orphan-exemption snapshot.
pub const MAX_CARGO_TRIGGER_EXEMPTIONS: usize = 4_096;
/// Cargo link chains deeper than this fail closed instead of being truncated.
pub const MAX_CARGO_TRIGGER_DEPTH: usize = 16;

const MAX_IDENTITY_BYTES: usize = 4_096;
const MAX_PACKAGE_NAME_BYTES: usize = 256;

/// One discovered package and the path triggers retained from its root
/// manifest's development and patch tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTriggerPackageSnapshot {
    pub name: String,
    pub root: String,
    pub trigger_paths: Vec<String>,
}

/// One captured Cargo crate and the normal/build path links retained by the
/// neutral `crate_closure` manifest profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoCrateSnapshot {
    pub path: String,
    pub link_paths: Vec<String>,
}

/// Closed input for authority-free trigger resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTriggerSnapshotInput {
    pub packages: Vec<CargoTriggerPackageSnapshot>,
    pub crates: Vec<CargoCrateSnapshot>,
    pub exemptions: Vec<String>,
    /// Whether the captured ancestor Cargo-configuration snapshot contains a
    /// `paths`, `patch`, or `source` redirect.
    pub config_redirected: bool,
}

/// A changed `trigger` package requires the `dependent` package's tests.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CargoTriggerRelation {
    pub trigger: String,
    pub dependent: String,
}

/// Stable, bounded failure categories.  Raw caller-controlled paths are never
/// retained in the error, so machine output cannot leak rejected source data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoTriggerSnapshotError {
    PackageLimitExceeded,
    CrateLimitExceeded,
    ReferenceLimitExceeded,
    ExemptionLimitExceeded,
    InvalidPackageName,
    UnsafePath,
    DuplicatePackageName,
    DuplicatePackageRoot,
    DuplicateCratePath,
    DuplicateExemption,
    ConfigRedirect,
    MissingCrate,
    UnownedCrate,
    DependencyCycle,
    DepthLimitExceeded,
}

impl CargoTriggerSnapshotError {
    /// Stable diagnostic code used by adapters without embedding source text.
    pub const fn code(self) -> &'static str {
        match self {
            Self::PackageLimitExceeded => "CARGO_TRIGGER_PACKAGE_LIMIT_EXCEEDED",
            Self::CrateLimitExceeded => "CARGO_TRIGGER_CRATE_LIMIT_EXCEEDED",
            Self::ReferenceLimitExceeded => "CARGO_TRIGGER_REFERENCE_LIMIT_EXCEEDED",
            Self::ExemptionLimitExceeded => "CARGO_TRIGGER_EXEMPTION_LIMIT_EXCEEDED",
            Self::InvalidPackageName => "CARGO_TRIGGER_PACKAGE_NAME_INVALID",
            Self::UnsafePath => "CARGO_TRIGGER_PATH_UNSAFE",
            Self::DuplicatePackageName => "CARGO_TRIGGER_PACKAGE_DUPLICATE",
            Self::DuplicatePackageRoot => "CARGO_TRIGGER_PACKAGE_ROOT_DUPLICATE",
            Self::DuplicateCratePath => "CARGO_TRIGGER_CRATE_DUPLICATE",
            Self::DuplicateExemption => "CARGO_TRIGGER_EXEMPTION_DUPLICATE",
            Self::ConfigRedirect => "CARGO_TRIGGER_CONFIG_REDIRECT",
            Self::MissingCrate => "CARGO_TRIGGER_CRATE_MISSING",
            Self::UnownedCrate => "CARGO_TRIGGER_CRATE_UNOWNED",
            Self::DependencyCycle => "CARGO_TRIGGER_CYCLE",
            Self::DepthLimitExceeded => "CARGO_TRIGGER_DEPTH_LIMIT_EXCEEDED",
        }
    }
}

/// Resolve a complete, deterministic trigger relation or one atomic error.
pub fn resolve_cargo_trigger_snapshot(
    input: &CargoTriggerSnapshotInput,
) -> Result<Vec<CargoTriggerRelation>, CargoTriggerSnapshotError> {
    validate_top_level_limits(input)?;
    if input.config_redirected {
        return Err(CargoTriggerSnapshotError::ConfigRedirect);
    }

    let mut package_names = BTreeSet::new();
    let mut package_roots = BTreeMap::new();
    for package in &input.packages {
        if !valid_package_name(&package.name) {
            return Err(CargoTriggerSnapshotError::InvalidPackageName);
        }
        if !is_canonical_identity(&package.root) {
            return Err(CargoTriggerSnapshotError::UnsafePath);
        }
        if !package_names.insert(package.name.clone()) {
            return Err(CargoTriggerSnapshotError::DuplicatePackageName);
        }
        if package_roots
            .insert(package.root.clone(), package.name.clone())
            .is_some()
        {
            return Err(CargoTriggerSnapshotError::DuplicatePackageRoot);
        }
        for path in &package.trigger_paths {
            lexical_join(&package.root, path)?;
        }
    }

    let mut crates = BTreeMap::new();
    for crate_snapshot in &input.crates {
        if !is_canonical_identity(&crate_snapshot.path) {
            return Err(CargoTriggerSnapshotError::UnsafePath);
        }
        if crates
            .insert(crate_snapshot.path.clone(), crate_snapshot)
            .is_some()
        {
            return Err(CargoTriggerSnapshotError::DuplicateCratePath);
        }
        for path in &crate_snapshot.link_paths {
            lexical_join(&crate_snapshot.path, path)?;
        }
    }

    let mut exemptions = BTreeSet::new();
    for exemption in &input.exemptions {
        if !is_canonical_identity(exemption) {
            return Err(CargoTriggerSnapshotError::UnsafePath);
        }
        if !exemptions.insert(exemption.clone()) {
            return Err(CargoTriggerSnapshotError::DuplicateExemption);
        }
    }

    let mut walker = ClosureWalker {
        crates,
        package_roots,
        exemptions,
        memo: BTreeMap::new(),
        expanding: BTreeSet::new(),
    };
    let mut relations = BTreeSet::new();

    // Input order carries no meaning.  Iterating a sorted projection makes
    // even which independent failure is reported deterministic.
    let packages_by_name: BTreeMap<_, _> = input
        .packages
        .iter()
        .map(|package| (package.name.as_str(), package))
        .collect();
    for package in packages_by_name.values() {
        let mut triggers = BTreeSet::new();
        for path in &package.trigger_paths {
            triggers.extend(walker.follow(&package.root, path, 1)?.packages);
        }
        for trigger in triggers {
            if trigger != package.name {
                relations.insert(CargoTriggerRelation {
                    trigger,
                    dependent: package.name.clone(),
                });
            }
        }
    }

    Ok(relations.into_iter().collect())
}

fn validate_top_level_limits(
    input: &CargoTriggerSnapshotInput,
) -> Result<(), CargoTriggerSnapshotError> {
    if input.packages.len() > MAX_CARGO_TRIGGER_PACKAGES {
        return Err(CargoTriggerSnapshotError::PackageLimitExceeded);
    }
    if input.crates.len() > MAX_CARGO_TRIGGER_CRATES {
        return Err(CargoTriggerSnapshotError::CrateLimitExceeded);
    }
    if input.exemptions.len() > MAX_CARGO_TRIGGER_EXEMPTIONS {
        return Err(CargoTriggerSnapshotError::ExemptionLimitExceeded);
    }
    let package_references = input
        .packages
        .iter()
        .try_fold(0usize, |count, package| {
            count.checked_add(package.trigger_paths.len())
        })
        .ok_or(CargoTriggerSnapshotError::ReferenceLimitExceeded)?;
    let crate_references = input
        .crates
        .iter()
        .try_fold(0usize, |count, crate_snapshot| {
            count.checked_add(crate_snapshot.link_paths.len())
        })
        .ok_or(CargoTriggerSnapshotError::ReferenceLimitExceeded)?;
    let references = package_references
        .checked_add(crate_references)
        .ok_or(CargoTriggerSnapshotError::ReferenceLimitExceeded)?;
    if references > MAX_CARGO_TRIGGER_REFERENCES {
        Err(CargoTriggerSnapshotError::ReferenceLimitExceeded)
    } else {
        Ok(())
    }
}

fn valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_PACKAGE_NAME_BYTES
        && !name.chars().any(|character| character.is_control())
}

fn has_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_canonical_identity(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_IDENTITY_BYTES
        && !path.starts_with('/')
        && !has_drive_prefix(path)
        && !path.contains(['\0', '\\'])
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// Join one manifest-relative dependency path without consulting the host.
/// `.` and `..` are accepted only in references; canonical snapshot identities
/// have already forbidden them.
fn lexical_join(base: &str, path: &str) -> Result<String, CargoTriggerSnapshotError> {
    if path.is_empty()
        || path.len() > MAX_IDENTITY_BYTES
        || path.starts_with('/')
        || has_drive_prefix(path)
        || path.contains(['\0', '\\'])
    {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    let mut segments: Vec<&str> = base.split('/').collect();
    for segment in path.split('/') {
        match segment {
            "" => return Err(CargoTriggerSnapshotError::UnsafePath),
            "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(CargoTriggerSnapshotError::UnsafePath);
                }
            }
            value => segments.push(value),
        }
    }
    if segments.is_empty() {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    let joined = segments.join("/");
    if joined.len() > MAX_IDENTITY_BYTES {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    Ok(joined)
}

struct ClosureWalker<'a> {
    crates: BTreeMap<String, &'a CargoCrateSnapshot>,
    package_roots: BTreeMap<String, String>,
    exemptions: BTreeSet<String>,
    memo: BTreeMap<String, ClosureResult>,
    expanding: BTreeSet<String>,
}

#[derive(Clone)]
struct ClosureResult {
    packages: BTreeSet<String>,
    /// Maximum number of crate records from this crate through one leaf,
    /// including this crate itself.
    height: usize,
}

impl ClosureWalker<'_> {
    fn follow(
        &mut self,
        from: &str,
        relative: &str,
        depth: usize,
    ) -> Result<ClosureResult, CargoTriggerSnapshotError> {
        if depth > MAX_CARGO_TRIGGER_DEPTH {
            return Err(CargoTriggerSnapshotError::DepthLimitExceeded);
        }
        let target = lexical_join(from, relative)?;
        if !self.crates.contains_key(&target) {
            return Err(CargoTriggerSnapshotError::MissingCrate);
        }

        let mut result = BTreeSet::new();
        if let Some(owner) = self.package_owner(&target) {
            result.insert(owner);
        } else if !self.exemptions.contains(&target) {
            return Err(CargoTriggerSnapshotError::UnownedCrate);
        }
        let closure = self.closure(&target, depth)?;
        result.extend(closure.packages);
        Ok(ClosureResult {
            packages: result,
            height: closure.height,
        })
    }

    fn package_owner(&self, crate_path: &str) -> Option<String> {
        self.package_roots
            .iter()
            .filter(|(root, _)| {
                crate_path == root.as_str()
                    || crate_path
                        .strip_prefix(root.as_str())
                        .is_some_and(|suffix| suffix.starts_with('/'))
            })
            .max_by_key(|(root, _)| root.len())
            .map(|(_, name)| name.clone())
    }

    fn closure(
        &mut self,
        crate_path: &str,
        depth: usize,
    ) -> Result<ClosureResult, CargoTriggerSnapshotError> {
        // Check the path-specific depth before using a shallow cached result.
        // Otherwise a diamond could smuggle a 17-level route through a closure
        // first computed at level two.
        if depth > MAX_CARGO_TRIGGER_DEPTH {
            return Err(CargoTriggerSnapshotError::DepthLimitExceeded);
        }
        if let Some(cached) = self.memo.get(crate_path) {
            if depth
                .checked_add(cached.height - 1)
                .is_none_or(|deepest| deepest > MAX_CARGO_TRIGGER_DEPTH)
            {
                return Err(CargoTriggerSnapshotError::DepthLimitExceeded);
            }
            return Ok(cached.clone());
        }
        if !self.expanding.insert(crate_path.to_string()) {
            return Err(CargoTriggerSnapshotError::DependencyCycle);
        }

        let links = self
            .crates
            .get(crate_path)
            .ok_or(CargoTriggerSnapshotError::MissingCrate)?
            .link_paths
            .clone();
        let mut result = BTreeSet::new();
        let mut height = 1;
        for link in links {
            match self.follow(crate_path, &link, depth + 1) {
                Ok(found) => {
                    height = height.max(1 + found.height);
                    result.extend(found.packages);
                }
                Err(error) => {
                    self.expanding.remove(crate_path);
                    return Err(error);
                }
            }
        }
        self.expanding.remove(crate_path);
        let closure = ClosureResult {
            packages: result,
            height,
        };
        self.memo.insert(crate_path.to_string(), closure.clone());
        Ok(closure)
    }
}
