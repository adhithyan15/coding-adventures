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
/// The largest emitted trigger relation. This matches the pure selector's
/// independent trigger-relation ceiling.
pub const MAX_CARGO_TRIGGER_RELATIONS: usize = 16_384;
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
    RelationLimitExceeded,
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
            Self::RelationLimitExceeded => "CARGO_TRIGGER_RELATION_LIMIT_EXCEEDED",
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

    // Validation uses one fixed category precedence. Within each category all
    // elements yield the same code, so permuting independently invalid records
    // cannot change the diagnostic.
    if input
        .packages
        .iter()
        .any(|package| !valid_package_name(&package.name))
    {
        return Err(CargoTriggerSnapshotError::InvalidPackageName);
    }
    if input
        .packages
        .iter()
        .any(|package| !is_canonical_identity(&package.root))
    {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    let package_names: BTreeSet<_> = input
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    if package_names.len() != input.packages.len() {
        return Err(CargoTriggerSnapshotError::DuplicatePackageName);
    }
    let package_roots: BTreeSet<_> = input
        .packages
        .iter()
        .map(|package| package.root.as_str())
        .collect();
    if package_roots.len() != input.packages.len() {
        return Err(CargoTriggerSnapshotError::DuplicatePackageRoot);
    }
    for package in &input.packages {
        for path in &package.trigger_paths {
            lexical_join(&package.root, path)?;
        }
    }

    if input
        .crates
        .iter()
        .any(|crate_snapshot| !is_canonical_identity(&crate_snapshot.path))
    {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    let crate_paths: BTreeSet<_> = input
        .crates
        .iter()
        .map(|crate_snapshot| crate_snapshot.path.as_str())
        .collect();
    if crate_paths.len() != input.crates.len() {
        return Err(CargoTriggerSnapshotError::DuplicateCratePath);
    }
    for crate_snapshot in &input.crates {
        for path in &crate_snapshot.link_paths {
            lexical_join(&crate_snapshot.path, path)?;
        }
    }

    if input
        .exemptions
        .iter()
        .any(|exemption| !is_canonical_identity(exemption))
    {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    let exemptions: BTreeSet<_> = input.exemptions.iter().cloned().collect();
    if exemptions.len() != input.exemptions.len() {
        return Err(CargoTriggerSnapshotError::DuplicateExemption);
    }

    let mut packages_by_name: Vec<_> = input.packages.iter().collect();
    packages_by_name.sort_by(|left, right| left.name.cmp(&right.name));
    let mut owner_index = PackageRootIndex::default();
    for (package_id, package) in packages_by_name.iter().enumerate() {
        owner_index.insert(&package.root, package_id);
    }
    let crates: BTreeMap<_, _> = input
        .crates
        .iter()
        .map(|crate_snapshot| (crate_snapshot.path.clone(), crate_snapshot))
        .collect();

    let mut walker = ClosureWalker {
        crates,
        owner_index,
        exemptions,
        package_count: packages_by_name.len(),
        memo: BTreeMap::new(),
        expanding: BTreeSet::new(),
    };
    let mut relations = BTreeSet::<(usize, usize)>::new();

    for (dependent_id, package) in packages_by_name.iter().enumerate() {
        let mut triggers = PackageSet::new(packages_by_name.len());
        let mut paths: Vec<_> = package.trigger_paths.iter().collect();
        paths.sort();
        for path in paths {
            triggers.union(&walker.follow(&package.root, path, 1)?.packages);
        }
        for trigger_id in triggers.ids() {
            if trigger_id != dependent_id
                && relations.insert((trigger_id, dependent_id))
                && relations.len() > MAX_CARGO_TRIGGER_RELATIONS
            {
                return Err(CargoTriggerSnapshotError::RelationLimitExceeded);
            }
        }
    }

    Ok(relations
        .into_iter()
        .map(|(trigger_id, dependent_id)| CargoTriggerRelation {
            trigger: packages_by_name[trigger_id].name.clone(),
            dependent: packages_by_name[dependent_id].name.clone(),
        })
        .collect())
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
    if !is_canonical_identity(&joined) {
        return Err(CargoTriggerSnapshotError::UnsafePath);
    }
    Ok(joined)
}

/// A segment trie resolves the deepest containing package in one pass over a
/// crate identity. It avoids comparing every path with every package root.
#[derive(Default)]
struct PackageRootIndex {
    root: PackageRootNode,
}

#[derive(Default)]
struct PackageRootNode {
    owner: Option<usize>,
    children: BTreeMap<String, PackageRootNode>,
}

impl PackageRootIndex {
    fn insert(&mut self, root: &str, owner: usize) {
        let mut node = &mut self.root;
        for segment in root.split('/') {
            node = node.children.entry(segment.to_string()).or_default();
        }
        node.owner = Some(owner);
    }

    fn owner(&self, path: &str) -> Option<usize> {
        let mut node = &self.root;
        let mut owner = node.owner;
        for segment in path.split('/') {
            let Some(child) = node.children.get(segment) else {
                break;
            };
            node = child;
            if node.owner.is_some() {
                owner = node.owner;
            }
        }
        owner
    }
}

/// Compact interned package membership. At the 4,096-package ceiling one
/// memoized closure is exactly 512 bytes rather than thousands of cloned
/// package names and tree nodes.
#[derive(Clone)]
struct PackageSet {
    words: Box<[u64]>,
}

impl PackageSet {
    fn new(package_count: usize) -> Self {
        Self {
            words: vec![0; package_count.div_ceil(64)].into_boxed_slice(),
        }
    }

    fn insert(&mut self, package_id: usize) {
        self.words[package_id / 64] |= 1u64 << (package_id % 64);
    }

    fn union(&mut self, other: &Self) {
        debug_assert_eq!(self.words.len(), other.words.len());
        for (word, other_word) in self.words.iter_mut().zip(other.words.iter()) {
            *word |= *other_word;
        }
    }

    fn ids(&self) -> PackageSetIds<'_> {
        PackageSetIds {
            words: &self.words,
            word_index: 0,
            remaining: self.words.first().copied().unwrap_or(0),
        }
    }
}

struct PackageSetIds<'a> {
    words: &'a [u64],
    word_index: usize,
    remaining: u64,
}

impl Iterator for PackageSetIds<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remaining != 0 {
                let bit = self.remaining.trailing_zeros() as usize;
                self.remaining &= self.remaining - 1;
                return Some(self.word_index * 64 + bit);
            }
            self.word_index += 1;
            self.remaining = *self.words.get(self.word_index)?;
        }
    }
}

struct ClosureWalker<'a> {
    crates: BTreeMap<String, &'a CargoCrateSnapshot>,
    owner_index: PackageRootIndex,
    exemptions: BTreeSet<String>,
    package_count: usize,
    memo: BTreeMap<String, ClosureResult>,
    expanding: BTreeSet<String>,
}

#[derive(Clone)]
struct ClosureResult {
    packages: PackageSet,
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

        let mut result = PackageSet::new(self.package_count);
        if let Some(owner) = self.owner_index.owner(&target) {
            result.insert(owner);
        } else if !self.exemptions.contains(&target) {
            return Err(CargoTriggerSnapshotError::UnownedCrate);
        }
        let closure = self.closure(&target, depth)?;
        result.union(&closure.packages);
        Ok(ClosureResult {
            packages: result,
            height: closure.height,
        })
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

        let mut links = self
            .crates
            .get(crate_path)
            .ok_or(CargoTriggerSnapshotError::MissingCrate)?
            .link_paths
            .clone();
        links.sort();
        let mut result = PackageSet::new(self.package_count);
        let mut height = 1;
        for link in links {
            match self.follow(crate_path, &link, depth + 1) {
                Ok(found) => {
                    height = height.max(1 + found.height);
                    result.union(&found.packages);
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
