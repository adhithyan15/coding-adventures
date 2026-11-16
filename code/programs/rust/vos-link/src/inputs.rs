//! # Loading and judging the inputs of a document
//!
//! A document is assembled from five kinds of file:
//!
//! ```text
//!   <impl>/vos.toml                    kind, contract, obligations           (VOS02 D-GD2)
//!   <impl>/warrant/impl.part           the hand-written implementation block
//!   <contract>/vos.toml                must say kind = "contract"
//!   <contract>/contract.vos            the tiers and the instance            (parsed, not copied)
//!   <contract>/warrant/contract.part   the contract block
//!   <contract>/warrant/obligations-<tier>.part   one per tier `contract.vos` declares
//! ```
//!
//! [`load`] finds them (paths contained as in VOS02 6.8), reads them within their caps,
//! and judges them.  It returns either everything the pure library needs ([`Loaded`]) or
//! the [`Finding`]s that stop a document from being made.  A path or size problem is a
//! [`NoVerdict`] instead (exit 2): the rules could not even be applied.
//!
//! ## Directory layout is fixed (D-GD3)
//!
//! The implementation directory is exactly `<root>/code/packages/rust/<impl>`, the
//! contract directory exactly `<root>/code/packages/rust/<contract>`, and the document
//! `<root>/code/specs/<impl>.warrant`.  `<root>` is four levels above the implementation
//! directory, or `--root`, which must agree.

use std::path::{Path, PathBuf};

use vos_manifest::caps::{CONTRACT_MAX_BYTES, PART_MAX_BYTES};
use vos_manifest::contract::{is_ident, parse_contract};
use vos_manifest::part::admit_part;
use vos_manifest::warrant::{claims_of_block, ClaimFact};
use vos_manifest::{Contract, Part, Tier};
use vos_shape_check::manifest::{parse_vos_toml, Kind, VosToml, VOS_TOML_MAX_BYTES};

use crate::error::{Finding, NoVerdict};
use crate::fsio::{absolute_plain, ensure_plain_dirs, list_names, read_regular_opt};

/// The three kinds of implementation (VOS02 R6); the other `vos.toml` kinds name crates
/// that are not implementations and are refused before this type is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImplKind {
    /// A real implementation.
    Production,
    /// A deterministic executable model with no `bound` rows.
    Reference,
    /// The most-general implementation with an oracle register.
    Any,
}

impl ImplKind {
    /// The `vos.toml` spelling.
    pub fn name(self) -> &'static str {
        match self {
            ImplKind::Production => "production",
            ImplKind::Reference => "reference",
            ImplKind::Any => "any",
        }
    }
}

/// Where everything is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The repository root.
    pub root: PathBuf,
    /// `<root>/code/packages/rust/<impl>`.
    pub impl_dir: PathBuf,
    /// The implementation package name.
    pub impl_pkg: String,
}

impl Layout {
    /// `<root>/code/packages/rust/<package>`.
    pub fn package_dir(&self, package: &str) -> PathBuf {
        self.root
            .join("code")
            .join("packages")
            .join("rust")
            .join(package)
    }

    /// `<root>/code/specs`.
    pub fn specs_dir(&self) -> PathBuf {
        self.root.join("code").join("specs")
    }

    /// The document's file name: `<impl>.warrant`.
    pub fn doc_name(&self) -> String {
        format!("{}.warrant", self.impl_pkg)
    }

    /// The path as the report prints it.
    pub fn doc_display(&self) -> String {
        format!("code/specs/{}", self.doc_name())
    }
}

/// Find the layout from the command line.
pub fn resolve_layout(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Layout, NoVerdict> {
    let impl_dir = absolute_plain(crate_arg)?;
    let name = impl_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| NoVerdict::new(format!("{}: not a package directory", impl_dir.display())))?
        .to_string();
    if !is_ident(&name) {
        return Err(NoVerdict::new(format!(
            "{}: `{name}` is not a package name ([a-z][a-z0-9-]{{0,31}})",
            impl_dir.display()
        )));
    }
    // .../code/packages/rust/<name>: the four ancestors.
    let mut up = impl_dir.ancestors();
    let _ = up.next();
    let rust = up.next();
    let packages = up.next();
    let code = up.next();
    let root = up.next();
    let ok = matches!(
        (rust, packages, code, root),
        (Some(r), Some(p), Some(c), Some(_))
            if r.file_name().is_some_and(|n| n == "rust")
                && p.file_name().is_some_and(|n| n == "packages")
                && c.file_name().is_some_and(|n| n == "code")
    );
    if !ok {
        return Err(NoVerdict::new(format!(
            "{}: an implementation lives at <root>/code/packages/rust/<package>",
            impl_dir.display()
        )));
    }
    let derived = root.map(Path::to_path_buf).unwrap_or_default();
    if let Some(given) = root_arg {
        let given = absolute_plain(given)?;
        if given != derived {
            return Err(NoVerdict::new(format!(
                "--root {} is not the repository root of {} ({})",
                given.display(),
                impl_dir.display(),
                derived.display()
            )));
        }
    }
    Ok(Layout {
        root: derived,
        impl_dir,
        impl_pkg: name,
    })
}

/// Everything the pure library needs to assemble and judge a document.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// Where everything is.
    pub layout: Layout,
    /// The declared kind: production, reference or any.
    pub kind: ImplKind,
    /// The contract package.
    pub contract_pkg: String,
    /// The tiers `gen-doc` embeds, sorted.
    pub obligations: Vec<Tier>,
    /// `contract.vos`.
    pub contract: Contract,
    /// `warrant/contract.part`.
    pub contract_part: Part,
    /// The obligations part of every tier the contract defines.
    pub tier_parts: Vec<(Tier, Part)>,
    /// The claims (name, `claim_sha256`) of every tier's part.
    pub tier_claims: Vec<(Tier, Vec<ClaimFact>)>,
    /// `<impl>/warrant/impl.part`.
    pub impl_part: Part,
}

/// What loading produced.
#[derive(Debug)]
pub enum Load {
    /// All inputs are admitted and consistent.
    Ready(Box<Loaded>),
    /// The inputs are readable but wrong.
    Findings(Vec<Finding>),
}

/// The file name of a tier's obligations part.
pub fn obligations_file(tier: Tier) -> String {
    format!("obligations-{}.part", tier.name())
}

fn exists_plain_dir(path: &Path) -> Result<bool, NoVerdict> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err(NoVerdict::new(format!("{}: is a symlink", path.display())))
        }
        Ok(m) if m.is_dir() => Ok(true),
        Ok(_) => Err(NoVerdict::new(format!(
            "{}: is not a directory",
            path.display()
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(NoVerdict::new(format!("{}: {e}", path.display()))),
    }
}

/// Read and parse a `vos.toml`.
fn read_vos_toml(
    dir: &Path,
    label: &str,
    f: &mut Vec<Finding>,
) -> Result<Option<VosToml>, NoVerdict> {
    let path = dir.join("vos.toml");
    let Some(bytes) = read_regular_opt(&path, VOS_TOML_MAX_BYTES)? else {
        f.push(Finding::new("GD05", format!("{label}vos.toml: not found")));
        return Ok(None);
    };
    let Ok(text) = String::from_utf8(bytes) else {
        f.push(Finding::new("GD05", format!("{label}vos.toml: not UTF-8")));
        return Ok(None);
    };
    match parse_vos_toml(&text) {
        Ok(v) => Ok(Some(v)),
        Err((line, why)) => {
            f.push(Finding::new(
                "GD05",
                format!("{label}vos.toml:{line}: {why}"),
            ));
            Ok(None)
        }
    }
}

/// Admit one part; a cap is a no-verdict, anything else a finding.
fn admit(bytes: &[u8], file: &str, f: &mut Vec<Finding>) -> Result<Option<Part>, NoVerdict> {
    match admit_part(bytes) {
        Ok(p) => Ok(Some(p)),
        Err(e) if e.is_cap() => Err(NoVerdict::new(format!("{file}: {e}"))),
        Err(e) => {
            f.push(Finding::new("GD01", format!("{file}: {e}")));
            Ok(None)
        }
    }
}

/// Load and judge the inputs for the implementation at `crate_arg`.
pub fn load(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Load, NoVerdict> {
    let layout = resolve_layout(crate_arg, root_arg)?;
    ensure_plain_dirs(&layout.root, &["code", "packages", "rust"])?;
    let mut f: Vec<Finding> = Vec::new();

    // --- the implementation's vos.toml (D-GD2) ---------------------------------------
    let Some(vt) = read_vos_toml(&layout.impl_dir, "", &mut f)? else {
        return Ok(Load::Findings(f));
    };
    let kind = match vt.kind {
        Some(Kind::Production) => Some(ImplKind::Production),
        Some(Kind::Reference) => Some(ImplKind::Reference),
        Some(Kind::Any) => Some(ImplKind::Any),
        Some(other) => {
            f.push(Finding::new(
                "GD05",
                format!(
                    "vos.toml: kind `{}` is not an implementation kind (production, reference or any)",
                    other.name()
                ),
            ));
            None
        }
        None => {
            f.push(Finding::new("GD05", "vos.toml: no `kind`"));
            None
        }
    };
    let contract_pkg = match &vt.contract {
        Some(c) if is_ident(c) => Some(c.clone()),
        Some(c) => {
            f.push(Finding::new(
                "GD05",
                format!("vos.toml: contract `{c}` is not a package name ([a-z][a-z0-9-]{{0,31}})"),
            ));
            None
        }
        None => {
            f.push(Finding::new("GD05", "vos.toml: no `contract`"));
            None
        }
    };
    if vt.obligations.is_empty() {
        f.push(Finding::new("GD05", "vos.toml: no `obligations`"));
    }
    let (Some(kind), Some(contract_pkg)) = (kind, contract_pkg) else {
        return Ok(Load::Findings(f));
    };

    // --- the contract package --------------------------------------------------------
    let contract_dir = layout.package_dir(&contract_pkg);
    if !exists_plain_dir(&contract_dir)? {
        f.push(Finding::new(
            "GD05",
            format!("vos.toml: contract package `{contract_pkg}` does not exist under code/packages/rust"),
        ));
        return Ok(Load::Findings(f));
    }
    if let Some(cvt) = read_vos_toml(&contract_dir, "contract package: ", &mut f)? {
        if cvt.kind != Some(Kind::Contract) {
            f.push(Finding::new(
                "GD05",
                format!(
                    "contract package `{contract_pkg}`: vos.toml does not say kind = \"contract\""
                ),
            ));
        }
    }

    let contract_bytes = read_regular_opt(&contract_dir.join("contract.vos"), CONTRACT_MAX_BYTES)?;
    let contract = match contract_bytes {
        None => {
            f.push(Finding::new("GD02", "contract.vos: not found"));
            None
        }
        Some(bytes) => match parse_contract(&bytes) {
            Ok(c) => Some(c),
            Err(e) if e.is_cap() => return Err(NoVerdict::new(e.to_string())),
            Err(e) => {
                f.push(Finding::new("GD02", e.to_string()));
                None
            }
        },
    };
    let Some(contract) = contract else {
        return Ok(Load::Findings(f));
    };
    if contract.package != contract_pkg {
        f.push(Finding::new(
            "GD02",
            format!(
                "contract.vos names package `{}` but lives in `{contract_pkg}`",
                contract.package
            ),
        ));
    }

    // --- the tier files agree with contract.vos (VOS-D049) ----------------------------
    let warrant_dir = contract_dir.join("warrant");
    let listing = list_names(&warrant_dir)?;
    let mut tier_parts: Vec<(Tier, Part)> = Vec::new();
    let mut tier_claims: Vec<(Tier, Vec<ClaimFact>)> = Vec::new();
    for name in &listing {
        let is_obligation = name.starts_with("obligations-") && name.ends_with(".part");
        let declared = contract.tiers.iter().any(|t| *name == obligations_file(*t));
        if is_obligation && !declared {
            f.push(Finding::new(
                "GD03",
                format!("{name} exists but contract.vos has no matching `tier` row"),
            ));
        }
    }
    let contract_part = match read_regular_opt(&warrant_dir.join("contract.part"), PART_MAX_BYTES)?
    {
        None => {
            f.push(Finding::new("GD03", "warrant/contract.part: not found"));
            None
        }
        Some(b) => admit(&b, "warrant/contract.part", &mut f)?,
    };
    for tier in &contract.tiers {
        let file = obligations_file(*tier);
        let Some(b) = read_regular_opt(&warrant_dir.join(&file), PART_MAX_BYTES)? else {
            f.push(Finding::new(
                "GD03",
                format!(
                    "contract.vos has `tier {}` but warrant/{file} does not exist",
                    tier.name()
                ),
            ));
            continue;
        };
        let Some(part) = admit(&b, &format!("warrant/{file}"), &mut f)? else {
            continue;
        };
        match claims_of_block(part.text()) {
            Ok(claims) => tier_claims.push((*tier, claims)),
            Err(e) => f.push(Finding::new("GD04", format!("warrant/{file}: {e}"))),
        }
        tier_parts.push((*tier, part));
    }
    // An obligation name is one claim: the same name in two tiers would make "carries"
    // ambiguous (and is a duplicate declaration in any document that embeds both).
    let mut seen = std::collections::BTreeMap::new();
    for (tier, claims) in &tier_claims {
        for c in claims {
            if let Some(first) = seen.insert(c.name.clone(), *tier) {
                f.push(Finding::new(
                    "GD04",
                    format!(
                        "obligation `{}` is declared in tier {} and in tier {}",
                        c.name,
                        first.name(),
                        tier.name()
                    ),
                ));
            }
        }
    }

    // --- obligations: sorted, unique, defined (D-GD2) -----------------------------------
    let mut obligations: Vec<Tier> = Vec::new();
    for name in &vt.obligations {
        match Tier::parse(name) {
            None => f.push(Finding::new(
                "GD05",
                format!(
                    "vos.toml: obligations `{name}` is not a tier (any, canonical, policy, safety)"
                ),
            )),
            Some(t) if !contract.has_tier(t) => f.push(Finding::new(
                "GD05",
                format!("vos.toml: obligations `{name}` is not a tier of `{contract_pkg}`"),
            )),
            Some(t) => {
                if let Some(&last) = obligations.last() {
                    if t <= last {
                        f.push(Finding::new(
                            "GD05",
                            "vos.toml: obligations must be sorted and unique (any, canonical, policy, safety)",
                        ));
                    }
                }
                obligations.push(t);
            }
        }
    }

    // --- the implementation block ----------------------------------------------------
    let impl_listing = list_names(&layout.impl_dir.join("warrant"))?;
    let impl_part = if impl_listing.iter().any(|n| n == "impl.part") {
        match read_regular_opt(
            &layout.impl_dir.join("warrant").join("impl.part"),
            PART_MAX_BYTES,
        )? {
            Some(b) => admit(&b, "warrant/impl.part", &mut f)?,
            None => None,
        }
    } else {
        f.push(Finding::new("GD01", "warrant/impl.part: not found"));
        None
    };

    match (f.is_empty(), contract_part, impl_part) {
        (true, Some(contract_part), Some(impl_part)) => Ok(Load::Ready(Box::new(Loaded {
            layout,
            kind,
            contract_pkg,
            obligations,
            contract,
            contract_part,
            tier_parts,
            tier_claims,
            impl_part,
        }))),
        _ => Ok(Load::Findings(f)),
    }
}
