//! # The VER13 assumption catalogue, as the report tool reads it
//!
//! Every proof label is an assume-guarantee statement: *if* the listed
//! assumptions hold, *then* the claim holds (VER00 section 7.5.1).  VER13 gives
//! every assumption a stable id in `code/assumptions/catalogue-v1.json`, and a
//! report may cite nothing else.  This module is the report tool's side of
//! that contract.  It loads three data files that live side by side:
//!
//! ```text
//! code/assumptions/catalogue-v1.json    what may be assumed (84 entries today)
//! code/assumptions/evidence-kinds.json  which evidence id proves which kind
//! code/assumptions/non-claims.json      what a package says it does NOT claim
//! ```
//!
//! and refuses anything malformed before a single report rule runs.
//!
//! ## Why the tool re-validates a file CI already checks
//!
//! `code/scripts/assumption_catalogue.py` checks the catalogue in CI (AC-14:
//! unique ids, resolving and acyclic edges, spec sync).  The report tool still
//! re-checks the *shape* here, because a report is only as trustworthy as the
//! data it was checked against, and the tool may run on a checkout where CI
//! never ran.  An analogy: a bank teller counts the notes even though the
//! armoured car's driver already did.  The spec-sync and backlog rules need
//! the VER13 markdown and are left to the Python checker.
//!
//! ## The pin
//!
//! The report records the SHA-256 of all three files in its `catalogue`
//! object (VER13 section 5.3):
//!
//! ```json
//! "catalogue": {"schema": "ver13-assumption-catalogue-v1",
//!               "path": "code/assumptions/catalogue-v1.json",
//!               "sha256": "<64 hex>",
//!               "evidence_kinds_sha256": "<64 hex>",
//!               "non_claims_sha256": "<64 hex>"}
//! ```
//!
//! `--check` recomputes the digests of the files on disk and fails closed when
//! any differs (rule AC-14, `CatalogueDrift`): a report checked against a
//! catalogue it was not generated from proves nothing.
//!
//! ## Fail closed
//!
//! | Input problem                                   | Result            |
//! |-------------------------------------------------|-------------------|
//! | file missing, unreadable, not UTF-8             | `CatalogueError`  |
//! | larger than its byte cap (checked while reading)| `CatalogueError`  |
//! | duplicate JSON key, unknown or missing field    | `CatalogueError`  |
//! | out-of-vocabulary layer/kind/status/rung/...    | `CatalogueError`  |
//! | dangling or cyclic `depends_on` / `refines`     | `CatalogueError`  |
//! | an evidence kind used but not defined (or v.v.) | `CatalogueError`  |
//! | two evidence-id patterns that can match one id  | `CatalogueError`  |

#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;

use crate::json::JsonValue;

// ---------------------------------------------------------------------------
// Locations, schemas and caps
// ---------------------------------------------------------------------------

/// Repository-relative path of the catalogue.  The report records it verbatim.
pub const CATALOGUE_PATH: &str = "code/assumptions/catalogue-v1.json";
/// Repository-relative path of the evidence-kind table (VER13 section 5.1).
pub const EVIDENCE_KINDS_PATH: &str = "code/assumptions/evidence-kinds.json";
/// Repository-relative path of the declared non-claims (rule AC-12).
pub const NON_CLAIMS_PATH: &str = "code/assumptions/non-claims.json";

pub const CATALOGUE_SCHEMA: &str = "ver13-assumption-catalogue-v1";
pub const EVIDENCE_KINDS_SCHEMA: &str = "ver13-evidence-kinds-v1";
pub const NON_CLAIMS_SCHEMA: &str = "ver13-non-claims-v1";

/// Byte caps, enforced on the bytes actually read (never on a metadata size
/// that could change between the check and the read).  The catalogue is about
/// 110 KiB today; a 1 MiB cap leaves room for growth and refuses a file that
/// would make the character-vector JSON parser allocate unreasonably.
pub const MAX_CATALOGUE_BYTES: u64 = 1024 * 1024;
pub const MAX_EVIDENCE_KINDS_BYTES: u64 = 256 * 1024;
pub const MAX_NON_CLAIMS_BYTES: u64 = 64 * 1024;
/// At most this many catalogue entries, evidence kinds or non-claims.
pub const MAX_ENTRIES: usize = 4096;
/// At most this many items in any list inside one entry.
pub const MAX_LIST_ITEMS: usize = 256;
/// At most this many bytes in any one string field.
pub const MAX_TEXT_BYTES: usize = 4096;
/// At most this many `/`-separated segments in an evidence-id pattern.
pub const MAX_PATTERN_SEGMENTS: usize = 16;
/// At most this many patterns across all evidence kinds (bounds the
/// pairwise overlap check, which also only compares within a bucket).
pub const MAX_TOTAL_PATTERNS: usize = 4096;

// ---------------------------------------------------------------------------
// Vocabularies (VER13 sections 2 and 5.2).  The Python checker and the JSON
// Schema under code/specs/schemas/ carry the same lists.
// ---------------------------------------------------------------------------

/// The eight stack layers, nearest the author first.  Index order matters:
/// a label `proved-to-L` speaks about every layer *above* (before) `L`.
pub const STACK_LAYERS: [&str; 8] = ["spec", "src", "wir", "cc", "mc", "uarch", "gate", "phys"];
/// `os` and `gov` have no stack position (VER13 section 5.2).
pub const OFF_STACK_LAYERS: [&str; 2] = ["os", "gov"];
pub const FAMILIES: [&str; 12] = [
    "spec", "src", "wir", "cc", "mc", "uarch", "gate", "phys", "tcb", "os", "gov", "att",
];
pub const KINDS: [&str; 8] = [
    "model",
    "trust",
    "tool",
    "environment",
    "attestation",
    "physical",
    "governance",
    "declared",
];
pub const STATUSES: [&str; 5] = ["assumed", "attested", "evidenced", "discharged", "floor"];
/// Discharge rungs of VER13 section 4, weakest first.
pub const RUNGS: [&str; 5] = ["R0+", "R1", "R2", "R3", "R4"];
pub const DIFFICULTIES: [&str; 5] = ["S", "M", "L", "XL", "N/A"];
pub const TIERS: [&str; 12] = [
    "O1", "O2", "O3", "O3/O4", "O4", "O5", "O6", "P", "floor", "gov", "via-ladder", "",
];
/// Label kinds an `applies_to` may name (`bounded(k)` stands for every `bounded(N)`).
pub const PROOF_LABELS: [&str; 5] = [
    "proved",
    "bounded(k)",
    "proved-per-run",
    "proved-to-isa",
    "proved-to-gate-level",
];
pub const CLAIM_KINDS: [&str; 6] = [
    "constant-time",
    "functional",
    "bound",
    "capability",
    "weakness-row",
    "report",
];
pub const INPUT_CLASSES: [&str; 4] = ["public", "secret", "trusted", "hostile"];
/// Where evidence of a kind is found.  `planned` means no manifest produces
/// it yet, so no evidence id of that kind can discharge anything today.
pub const MANIFESTS: [&str; 4] = ["test", "proof", "bench", "planned"];
/// The fields a per-run evidence entry pins (rule AC-9).
pub const PINNED_FIELDS: [&str; 4] = ["toolchain", "target", "flags", "artefact_sha256"];

const ENTRY_REQUIRED: [&str; 18] = [
    "id",
    "aliases",
    "refines",
    "refined_by",
    "layer",
    "family",
    "title",
    "statement",
    "kind",
    "status",
    "applies_to",
    "depends_on",
    "discharge",
    "since",
    "legacy",
    "used_by",
    "tier",
    "constrains_input_classes",
];
const ENTRY_OPTIONAL: [&str; 2] = ["discharged_by", "notes"];
const APPLIES_REQUIRED: [&str; 3] = ["labels", "claims", "packages"];
const DISCHARGE_REQUIRED: [&str; 6] = ["evidence_kinds", "rung", "tools", "difficulty", "backlog", "plan"];
const DISCHARGED_BY_REQUIRED: [&str; 4] = ["evidence", "digest", "stack", "pinned"];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// The catalogue files cannot be used.  Always fatal: there is no "partly
/// valid" catalogue a report could be checked against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueError(pub String);

impl fmt::Display for CatalogueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AC-14 MalformedCatalogue: {}", self.0)
    }
}

impl std::error::Error for CatalogueError {}

fn bad<T>(msg: impl Into<String>) -> Result<T, CatalogueError> {
    Err(CatalogueError(msg.into()))
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// Which proofs an assumption is about: proof label kinds, claim kinds and
/// packages (`*` = every package).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliesTo {
    pub labels: Vec<String>,
    pub claims: Vec<String>,
    pub packages: Vec<String>,
}

/// What would discharge an assumption (only the fields the report rules use
/// are kept; the rest are validated and dropped).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discharge {
    pub evidence_kinds: Vec<String>,
    /// The lowest rung (`R0+`..`R4`) at which a discharge counts.
    pub rung: String,
}

/// One piece of evidence behind a catalogue entry whose status is `discharged`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DischargedBy {
    pub evidence: String,
    /// `sha256:<64 hex>` of the manifest that holds `evidence`.
    pub digest: String,
    pub stack: String,
    pub pinned: BTreeMap<String, String>,
}

/// One catalogue entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assumption {
    pub id: String,
    pub aliases: Vec<String>,
    pub refines: Option<String>,
    pub refined_by: Vec<String>,
    pub layer: String,
    pub family: String,
    pub kind: String,
    pub status: String,
    pub applies_to: AppliesTo,
    pub depends_on: Vec<String>,
    pub discharge: Option<Discharge>,
    pub discharged_by: Vec<DischargedBy>,
}

impl Assumption {
    /// `status: floor` and `kind: declared` (part of the claim) or `physical`
    /// entries are the bottom of the ladder: software cannot discharge them.
    pub fn is_floor(&self) -> bool {
        self.status == "floor" || self.kind == "declared" || self.kind == "physical"
    }

    /// Position of the layer in [`STACK_LAYERS`]; `None` for `os` / `gov`.
    pub fn stack_index(&self) -> Option<usize> {
        STACK_LAYERS.iter().position(|l| *l == self.layer)
    }

    /// Does the entry name `package` (or every package)?
    pub fn names_package(&self, package: &str) -> bool {
        self.applies_to.packages.iter().any(|p| p == "*" || p == package)
    }
}

/// The parsed catalogue with id and alias indexes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogue {
    pub entries: Vec<Assumption>,
    index: BTreeMap<String, usize>,
    aliases: BTreeMap<String, usize>,
}

impl Catalogue {
    /// The entry with canonical id `id`.
    pub fn get(&self, id: &str) -> Option<&Assumption> {
        self.index.get(id).map(|&i| &self.entries[i])
    }

    /// The entry that owns alias `alias`.
    pub fn by_alias(&self, alias: &str) -> Option<&Assumption> {
        self.aliases.get(alias).map(|&i| &self.entries[i])
    }

    /// An id or an alias, resolved to its entry.
    pub fn resolve(&self, id_or_alias: &str) -> Option<&Assumption> {
        self.get(id_or_alias).or_else(|| self.by_alias(id_or_alias))
    }
}

/// One segment of an evidence-id pattern: a literal, or `*` for exactly one
/// non-empty segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Any,
}

/// One row of `evidence-kinds.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceKind {
    pub kind: String,
    /// `test`, `proof`, `bench` or `planned` ([`MANIFESTS`]).
    pub manifest: String,
    pub patterns: Vec<Vec<Segment>>,
    /// The verdict the manifest entry must carry for the evidence to count.
    pub verdict: String,
    /// Per-run evidence is about one pinned build (rule AC-9).
    pub per_run: bool,
}

/// `evidence-kinds.json`, parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceKinds {
    pub kinds: Vec<EvidenceKind>,
}

impl EvidenceKinds {
    /// The kind whose pattern matches `evidence_id`.  `Ok(None)` when no
    /// pattern matches; an error when more than one kind does (patterns are
    /// checked disjoint at load, so this is defence in depth).
    pub fn classify(&self, evidence_id: &str) -> Result<Option<&EvidenceKind>, String> {
        let segments: Vec<&str> = evidence_id.split('/').collect();
        let mut found: Option<&EvidenceKind> = None;
        for k in &self.kinds {
            if k.patterns.iter().any(|p| pattern_matches(p, &segments)) {
                if let Some(prev) = found {
                    return Err(format!(
                        "evidence id {evidence_id:?} matches both kind {} and kind {}",
                        prev.kind, k.kind
                    ));
                }
                found = Some(k);
            }
        }
        Ok(found)
    }

    pub fn get(&self, kind: &str) -> Option<&EvidenceKind> {
        self.kinds.iter().find(|k| k.kind == kind)
    }
}

/// One row of `non-claims.json`: a package states it does *not* provide a
/// property (VER13 section 2, "Non-claim"), so the report may never show that
/// property as proved (rule AC-12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonClaim {
    pub package: String,
    pub cwe: u32,
    /// A claim kind from [`CLAIM_KINDS`], e.g. `constant-time`.
    pub claim: String,
    /// Where the package's documentation says so.
    pub source: String,
}

/// Everything a report is checked against, with the digests it pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueSet {
    pub catalogue: Catalogue,
    pub kinds: EvidenceKinds,
    pub non_claims: Vec<NonClaim>,
    pub catalogue_sha256: String,
    pub evidence_kinds_sha256: String,
    pub non_claims_sha256: String,
}

impl CatalogueSet {
    /// Find `code/assumptions/` by walking up from `package_dir` (the same way
    /// the Warrant document is found) and load all three files from it.
    ///
    /// The search starts at the package directory's *parent*: a package that
    /// ships its own `code/assumptions/*.json` must not be able to shadow the
    /// repository catalogue it is checked against (security review, LOW).
    pub fn locate(package_dir: &Path) -> Result<CatalogueSet, CatalogueError> {
        let start = canonical_dir(package_dir)?;
        if start.join(CATALOGUE_PATH).exists() {
            return bad(format!(
                "{} ships its own {CATALOGUE_PATH}; the catalogue must live above the package, not in it",
                package_dir.display()
            ));
        }
        for dir in start.ancestors().skip(1) {
            if dir.join(CATALOGUE_PATH).is_file() {
                return Self::load_from_root(dir);
            }
        }
        bad(format!("{CATALOGUE_PATH} not found above {}", package_dir.display()))
    }

    /// Load the catalogue of repository root `root`, which must be a strict
    /// ancestor of `package_dir`.  [`crate::evidence::Evidence::load`] passes
    /// the root at which it found the package's Warrant document, so the spec
    /// and the catalogue always come from the same checkout.
    pub fn load_for_package(root: &Path, package_dir: &Path) -> Result<CatalogueSet, CatalogueError> {
        let pkg = canonical_dir(package_dir)?;
        let root = canonical_dir(root)?;
        if root == pkg || !pkg.starts_with(&root) {
            return bad(format!(
                "catalogue root {} is not a strict ancestor of the package directory {}",
                root.display(),
                pkg.display()
            ));
        }
        if !root.join(CATALOGUE_PATH).is_file() {
            return bad(format!("{CATALOGUE_PATH} not found at {}", root.display()));
        }
        Self::load_from_root(&root)
    }

    /// Load the three files below repository root `root`.
    pub fn load_from_root(root: &Path) -> Result<CatalogueSet, CatalogueError> {
        let read = |rel: &str, cap: u64| -> Result<Vec<u8>, CatalogueError> {
            let path: PathBuf = root.join(rel);
            crate::evidence::read_capped(&path, cap).map_err(|e| CatalogueError(e.0))
        };
        Self::from_bytes(
            &read(CATALOGUE_PATH, MAX_CATALOGUE_BYTES)?,
            &read(EVIDENCE_KINDS_PATH, MAX_EVIDENCE_KINDS_BYTES)?,
            &read(NON_CLAIMS_PATH, MAX_NON_CLAIMS_BYTES)?,
        )
    }

    /// Parse and cross-validate the three files from their bytes.
    pub fn from_bytes(
        catalogue: &[u8],
        kinds: &[u8],
        non_claims: &[u8],
    ) -> Result<CatalogueSet, CatalogueError> {
        let cat = parse_catalogue(catalogue)?;
        let ek = parse_evidence_kinds(kinds)?;
        let nc = parse_non_claims(non_claims)?;
        // Closure in both directions: every kind the catalogue names is
        // defined, and every defined kind is named by some entry, so the two
        // files cannot drift apart silently.
        let mut used: BTreeSet<&str> = BTreeSet::new();
        for e in &cat.entries {
            for k in e.discharge.iter().flat_map(|d| d.evidence_kinds.iter()) {
                if ek.get(k).is_none() {
                    return bad(format!("{}: evidence kind `{k}` is not defined in {EVIDENCE_KINDS_PATH}", e.id));
                }
                used.insert(k);
            }
        }
        for k in &ek.kinds {
            if !used.contains(k.kind.as_str()) {
                return bad(format!("{EVIDENCE_KINDS_PATH}: kind `{}` is named by no catalogue entry", k.kind));
            }
        }
        Ok(CatalogueSet {
            catalogue: cat,
            kinds: ek,
            non_claims: nc,
            catalogue_sha256: sha256_hex(catalogue),
            evidence_kinds_sha256: sha256_hex(kinds),
            non_claims_sha256: sha256_hex(non_claims),
        })
    }
}

fn canonical_dir(dir: &Path) -> Result<PathBuf, CatalogueError> {
    dir.canonicalize().map_err(|e| CatalogueError(format!("directory {}: {e}", dir.display())))
}

// ---------------------------------------------------------------------------
// Strict field readers.  Each takes the object, the key and a description of
// where we are, and refuses a wrong type instead of defaulting.
// ---------------------------------------------------------------------------

type Obj = BTreeMap<String, JsonValue>;

fn parse_root(bytes: &[u8], what: &str, schema: &str) -> Result<JsonValue, CatalogueError> {
    let text = std::str::from_utf8(bytes).map_err(|_| CatalogueError(format!("{what} is not UTF-8")))?;
    let v = JsonValue::parse(text).map_err(|e| CatalogueError(format!("{what} is not valid JSON: {e}")))?;
    let obj = v.as_object().ok_or_else(|| CatalogueError(format!("{what}: root must be an object")))?;
    match obj.get("schema").and_then(|s| s.as_str()) {
        Some(s) if s == schema => Ok(v),
        _ => bad(format!("{what}: `schema` must be {schema:?}")),
    }
}

fn as_obj<'a>(v: &'a JsonValue, what: &str) -> Result<&'a Obj, CatalogueError> {
    v.as_object().ok_or_else(|| CatalogueError(format!("{what} must be an object")))
}

/// Every key of `o` is required or optional, and every required key is there.
fn exact_fields(o: &Obj, required: &[&str], optional: &[&str], what: &str) -> Result<(), CatalogueError> {
    for key in o.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return bad(format!("{what}: unknown field `{key}`"));
        }
    }
    for key in required {
        if !o.contains_key(*key) {
            return bad(format!("{what}: missing field `{key}`"));
        }
    }
    Ok(())
}

fn text(o: &Obj, key: &str, what: &str) -> Result<String, CatalogueError> {
    match o.get(key) {
        Some(JsonValue::String(s)) if s.len() <= MAX_TEXT_BYTES => Ok(s.clone()),
        Some(JsonValue::String(_)) => bad(format!("{what}: `{key}` is longer than {MAX_TEXT_BYTES} bytes")),
        _ => bad(format!("{what}: `{key}` must be a string")),
    }
}

fn nonempty_text(o: &Obj, key: &str, what: &str) -> Result<String, CatalogueError> {
    let s = text(o, key, what)?;
    if s.trim().is_empty() {
        return bad(format!("{what}: `{key}` must not be empty"));
    }
    Ok(s)
}

fn text_list(o: &Obj, key: &str, what: &str) -> Result<Vec<String>, CatalogueError> {
    let arr = o
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| CatalogueError(format!("{what}: `{key}` must be a list")))?;
    if arr.len() > MAX_LIST_ITEMS {
        return bad(format!("{what}: `{key}` has more than {MAX_LIST_ITEMS} items"));
    }
    arr.iter()
        .map(|x| match x {
            JsonValue::String(s) if s.len() <= MAX_TEXT_BYTES => Ok(s.clone()),
            _ => bad(format!("{what}: `{key}` items must be strings of at most {MAX_TEXT_BYTES} bytes")),
        })
        .collect()
}

fn in_vocab(value: &str, vocab: &[&str], field: &str, what: &str) -> Result<(), CatalogueError> {
    if vocab.contains(&value) {
        Ok(())
    } else {
        bad(format!("{what}: {field} {value:?} is not one of {vocab:?}"))
    }
}

fn vocab_list(o: &Obj, key: &str, vocab: &[&str], what: &str) -> Result<Vec<String>, CatalogueError> {
    let list = text_list(o, key, what)?;
    for item in &list {
        in_vocab(item, vocab, key, what)?;
    }
    Ok(list)
}

// Hand-written character classes (no regex dependency), one per VER13 pattern.

/// `[A-Za-z0-9._-]+` (VER13 section 5.2 `id`).
pub fn is_id(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// `[a-z0-9._:-]+` (aliases may carry legacy `:` spellings).
fn is_alias(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

/// `[a-z0-9_-]+/[a-z0-9_-]+`, or `*` when `star` is allowed.
fn is_package(s: &str, star: bool) -> bool {
    if s == "*" {
        return star;
    }
    let part = |p: &str| {
        !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
    };
    matches!(s.split_once('/'), Some((a, b)) if part(a) && part(b))
}

/// `[A-Z][A-Z0-9]*-[0-9A-Za-z]+` (a VER-BACKLOG id such as `AS-002`).
fn is_backlog_id(s: &str) -> bool {
    let Some((head, tail)) = s.split_once('-') else { return false };
    let mut h = head.bytes();
    matches!(h.next(), Some(b) if b.is_ascii_uppercase())
        && h.all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && !tail.is_empty()
        && tail.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `sha256:` followed by 64 lowercase hex digits.
pub fn is_prefixed_digest(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(is_hex64)
}

/// 64 lowercase hex digits.
pub fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

// ---------------------------------------------------------------------------
// catalogue-v1.json
// ---------------------------------------------------------------------------

/// Parse and validate the catalogue: structure (the twin of
/// `ver13-assumption-catalogue-v1.schema.json`), identity, graph and shape
/// rules.  The rules mirror `code/scripts/assumption_catalogue.py`.
pub fn parse_catalogue(bytes: &[u8]) -> Result<Catalogue, CatalogueError> {
    let root = parse_root(bytes, "catalogue", CATALOGUE_SCHEMA)?;
    let root_obj = as_obj(&root, "catalogue")?;
    exact_fields(root_obj, &["schema", "assumptions"], &[], "catalogue")?;
    let raw = root_obj
        .get("assumptions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CatalogueError("catalogue: `assumptions` must be a list".into()))?;
    if raw.is_empty() || raw.len() > MAX_ENTRIES {
        return bad(format!("catalogue: `assumptions` must hold 1..={MAX_ENTRIES} entries"));
    }
    let mut entries = Vec::with_capacity(raw.len());
    for (i, v) in raw.iter().enumerate() {
        entries.push(parse_entry(v, i)?);
    }

    // Identity: ids unique; aliases unique and never equal to an id.
    let mut index = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        if index.insert(e.id.clone(), i).is_some() {
            return bad(format!("duplicate id `{}`", e.id));
        }
    }
    let mut aliases = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        for a in &e.aliases {
            if index.contains_key(a) {
                return bad(format!("{}: alias `{a}` collides with an id", e.id));
            }
            if aliases.insert(a.clone(), i).is_some() {
                return bad(format!("{}: alias `{a}` is owned twice", e.id));
            }
        }
    }

    // Graph: every edge resolves, the two views of `refines` agree, no cycle.
    for e in &entries {
        for r in e.depends_on.iter().chain(e.refined_by.iter()).chain(e.refines.iter()) {
            if !index.contains_key(r) {
                return bad(format!("{}: names unknown id `{r}`", e.id));
            }
            if *r == e.id {
                return bad(format!("{}: an entry cannot depend on, refine or be refined by itself", e.id));
            }
        }
        if let Some(parent) = &e.refines {
            if !entries[index[parent]].refined_by.contains(&e.id) {
                return bad(format!("{}: refines {parent} but is not in its refined_by", e.id));
            }
        }
        for child in &e.refined_by {
            if entries[index[child]].refines.as_deref() != Some(e.id.as_str()) {
                return bad(format!("{}: refined_by {child} whose refines is not {}", e.id, e.id));
            }
        }
    }
    if let Some(cycle) = find_cycle(&entries, &index) {
        return bad(format!("depends_on/refines cycle through `{cycle}`"));
    }

    // Shape rules: the catalogue-side halves of AC-3, AC-8, AC-11 and AC-15.
    for e in &entries {
        if e.kind == "declared" && e.status != "floor" {
            return bad(format!("AC-8 {}: kind declared must have status floor", e.id));
        }
        if (e.status == "floor") != e.discharge.is_none() {
            return bad(format!("AC-8 {}: discharge must be null exactly when status is floor", e.id));
        }
        if e.status == "discharged" && e.is_floor() {
            return bad(format!("AC-8 {}: kind {} can never be discharged", e.id, e.kind));
        }
        if (e.status == "discharged") != !e.discharged_by.is_empty() {
            return bad(format!("AC-3 {}: discharged_by is required exactly when status is discharged", e.id));
        }
        if (e.kind == "attestation") != (e.status == "attested") {
            return bad(format!("AC-11 {}: kind attestation and status attested go together", e.id));
        }
        let unverified = e.aliases.iter().filter(|a| a.starts_with("unverified::")).count();
        if (e.kind == "attestation" && unverified != 1) || (e.kind != "attestation" && unverified != 0) {
            return bad(format!("AC-11 {}: exactly the attestations own one unverified:: alias each", e.id));
        }
    }
    Ok(Catalogue { entries, index, aliases })
}

fn parse_entry(v: &JsonValue, i: usize) -> Result<Assumption, CatalogueError> {
    let pre = format!("catalogue entry {i}");
    let o = as_obj(v, &pre)?;
    let id = text(o, "id", &pre)?;
    if !is_id(&id) {
        return bad(format!("{pre}: id {id:?} violates [A-Za-z0-9._-]+"));
    }
    let what = format!("catalogue entry `{id}`");
    let what = what.as_str();
    exact_fields(o, &ENTRY_REQUIRED, &ENTRY_OPTIONAL, what)?;

    for key in ["title", "statement", "since"] {
        nonempty_text(o, key, what)?;
    }
    text(o, "used_by", what)?;
    if let Some(n) = o.get("notes") {
        if n.as_str().is_none() {
            return bad(format!("{what}: `notes` must be a string"));
        }
        text(o, "notes", what)?;
    }
    if o.get("legacy").and_then(|b| b.as_bool()).is_none() {
        return bad(format!("{what}: `legacy` must be a boolean"));
    }
    let layer = text(o, "layer", what)?;
    let layers: Vec<&str> = STACK_LAYERS.iter().chain(OFF_STACK_LAYERS.iter()).copied().collect();
    in_vocab(&layer, &layers, "layer", what)?;
    let family = text(o, "family", what)?;
    in_vocab(&family, &FAMILIES, "family", what)?;
    let kind = text(o, "kind", what)?;
    in_vocab(&kind, &KINDS, "kind", what)?;
    let status = text(o, "status", what)?;
    in_vocab(&status, &STATUSES, "status", what)?;
    in_vocab(&text(o, "tier", what)?, &TIERS, "tier", what)?;

    let aliases = text_list(o, "aliases", what)?;
    if let Some(a) = aliases.iter().find(|a| !is_alias(a)) {
        return bad(format!("{what}: alias {a:?} violates [a-z0-9._:-]+"));
    }
    let refines = match o.get("refines") {
        Some(JsonValue::Null) => None,
        Some(JsonValue::String(s)) => Some(s.clone()),
        _ => return bad(format!("{what}: `refines` must be an id or null")),
    };
    let refined_by = text_list(o, "refined_by", what)?;
    let depends_on = text_list(o, "depends_on", what)?;
    let classes = vocab_list(o, "constrains_input_classes", &INPUT_CLASSES, what)?;
    // AC-15: an environment or spec assumption may not constrain a hostile input.
    if classes.iter().any(|c| c == "hostile") && (kind == "environment" || family == "spec") {
        return bad(format!("AC-15 {what}: an environment/spec assumption may not constrain a hostile input"));
    }

    let ap = as_obj(o.get("applies_to").unwrap_or(&JsonValue::Null), &format!("{what}: applies_to"))?;
    exact_fields(ap, &APPLIES_REQUIRED, &[], &format!("{what}: applies_to"))?;
    let applies_to = AppliesTo {
        labels: vocab_list(ap, "labels", &PROOF_LABELS, what)?,
        claims: vocab_list(ap, "claims", &CLAIM_KINDS, what)?,
        packages: text_list(ap, "packages", what)?,
    };
    if let Some(p) = applies_to.packages.iter().find(|p| !is_package(p, true)) {
        return bad(format!("{what}: applies_to.packages entry {p:?} is not `*` or `lang/name`"));
    }

    let discharge = match o.get("discharge") {
        Some(JsonValue::Null) => None,
        Some(d) => {
            let dw = format!("{what}: discharge");
            let d = as_obj(d, &dw)?;
            exact_fields(d, &DISCHARGE_REQUIRED, &[], &dw)?;
            let evidence_kinds = text_list(d, "evidence_kinds", &dw)?;
            if evidence_kinds.is_empty() {
                return bad(format!("{dw}: `evidence_kinds` must not be empty"));
            }
            let rung = text(d, "rung", &dw)?;
            in_vocab(&rung, &RUNGS, "rung", &dw)?;
            text_list(d, "tools", &dw)?;
            in_vocab(&text(d, "difficulty", &dw)?, &DIFFICULTIES, "difficulty", &dw)?;
            let backlog = text(d, "backlog", &dw)?;
            if !is_backlog_id(&backlog) {
                return bad(format!("{dw}: backlog {backlog:?} is not a backlog id"));
            }
            nonempty_text(d, "plan", &dw)?;
            Some(Discharge { evidence_kinds, rung })
        }
        None => return bad(format!("{what}: missing `discharge`")),
    };

    let mut discharged_by = Vec::new();
    if let Some(list) = o.get("discharged_by") {
        let list = list
            .as_array()
            .ok_or_else(|| CatalogueError(format!("{what}: `discharged_by` must be a list")))?;
        if list.len() > MAX_LIST_ITEMS {
            return bad(format!("{what}: `discharged_by` has more than {MAX_LIST_ITEMS} items"));
        }
        for item in list {
            let dw = format!("{what}: discharged_by");
            let d = as_obj(item, &dw)?;
            exact_fields(d, &DISCHARGED_BY_REQUIRED, &[], &dw)?;
            let digest = text(d, "digest", &dw)?;
            if !is_prefixed_digest(&digest) {
                return bad(format!("{dw}: digest must be sha256:<64 hex>"));
            }
            discharged_by.push(DischargedBy {
                evidence: nonempty_text(d, "evidence", &dw)?,
                digest,
                stack: nonempty_text(d, "stack", &dw)?,
                pinned: parse_pinned_object(d.get("pinned").unwrap_or(&JsonValue::Null), &dw, true)?,
            });
        }
    }

    Ok(Assumption {
        id,
        aliases,
        refines,
        refined_by,
        layer,
        family,
        kind,
        status,
        applies_to,
        depends_on,
        discharge,
        discharged_by,
    })
}

/// A `pinned` object: string values only, keys from [`PINNED_FIELDS`].  When
/// `may_be_empty` is false every pinned field must be present and non-empty.
pub fn parse_pinned_object(
    v: &JsonValue,
    what: &str,
    may_be_empty: bool,
) -> Result<BTreeMap<String, String>, CatalogueError> {
    let o = as_obj(v, &format!("{what}: pinned"))?;
    let mut out = BTreeMap::new();
    for (k, val) in o {
        if !PINNED_FIELDS.contains(&k.as_str()) {
            return bad(format!("{what}: pinned has unknown field `{k}`"));
        }
        match val {
            JsonValue::String(s) if !s.is_empty() && s.len() <= MAX_TEXT_BYTES => {
                out.insert(k.clone(), s.clone());
            }
            _ => return bad(format!("{what}: pinned `{k}` must be a non-empty string")),
        }
    }
    if !may_be_empty || !out.is_empty() {
        for f in PINNED_FIELDS {
            if !out.contains_key(f) {
                return bad(format!("{what}: pinned is missing `{f}`"));
            }
        }
    }
    Ok(out)
}

/// A cycle in the union of `depends_on` and `refines`, if there is one.
/// Iterative three-colour DFS: white = unvisited, grey = on the current path,
/// black = finished.  Reaching a grey node again closes a cycle.
fn find_cycle(entries: &[Assumption], index: &BTreeMap<String, usize>) -> Option<String> {
    let succ = |i: usize| -> Vec<usize> {
        let e = &entries[i];
        e.depends_on.iter().chain(e.refines.iter()).map(|r| index[r]).collect()
    };
    let mut colour = vec![0u8; entries.len()]; // 0 white, 1 grey, 2 black
    for start in 0..entries.len() {
        if colour[start] != 0 {
            continue;
        }
        let mut stack: Vec<(usize, Vec<usize>)> = vec![(start, succ(start))];
        colour[start] = 1;
        while let Some((node, next)) = stack.last_mut() {
            if let Some(n) = next.pop() {
                match colour[n] {
                    0 => {
                        colour[n] = 1;
                        let s = succ(n);
                        stack.push((n, s));
                    }
                    1 => return Some(entries[n].id.clone()),
                    _ => {}
                }
            } else {
                colour[*node] = 2;
                stack.pop();
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// evidence-kinds.json
// ---------------------------------------------------------------------------

/// Parse one evidence-id pattern: 1..=16 `/`-separated segments, each `*` or a
/// literal of `[A-Za-z0-9_.:-]+`; the first segment is always a literal (the
/// namespace), so a pattern can never match every id.
pub fn parse_pattern(p: &str) -> Result<Vec<Segment>, String> {
    let parts: Vec<&str> = p.split('/').collect();
    if parts.is_empty() || parts.len() > MAX_PATTERN_SEGMENTS {
        return Err(format!("pattern {p:?} must have 1..={MAX_PATTERN_SEGMENTS} segments"));
    }
    let mut out = Vec::with_capacity(parts.len());
    for (i, s) in parts.iter().enumerate() {
        if *s == "*" {
            if i == 0 {
                return Err(format!("pattern {p:?} must start with a literal namespace"));
            }
            out.push(Segment::Any);
        } else if !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
        {
            out.push(Segment::Literal(s.to_string()));
        } else {
            return Err(format!("pattern {p:?} has a malformed segment {s:?}"));
        }
    }
    Ok(out)
}

/// Does a parsed pattern match the segments of an id?  `*` matches exactly one
/// non-empty segment; there is no multi-segment wildcard.
pub fn pattern_matches(pattern: &[Segment], id: &[&str]) -> bool {
    pattern.len() == id.len()
        && pattern.iter().zip(id).all(|(p, s)| match p {
            Segment::Any => !s.is_empty(),
            Segment::Literal(l) => l == s,
        })
}

/// Can one id match both patterns?  Exact for this grammar: equal length and,
/// position by position, either side is `*` or the literals are equal.
fn patterns_overlap(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| match (x, y) {
            (Segment::Literal(l), Segment::Literal(r)) => l == r,
            _ => true,
        })
}

/// Parse and validate `evidence-kinds.json`.
pub fn parse_evidence_kinds(bytes: &[u8]) -> Result<EvidenceKinds, CatalogueError> {
    let what = "evidence-kinds";
    let root = parse_root(bytes, what, EVIDENCE_KINDS_SCHEMA)?;
    let o = as_obj(&root, what)?;
    exact_fields(o, &["schema", "kinds"], &[], what)?;
    let raw = o
        .get("kinds")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CatalogueError(format!("{what}: `kinds` must be a list")))?;
    if raw.is_empty() || raw.len() > MAX_ENTRIES {
        return bad(format!("{what}: `kinds` must hold 1..={MAX_ENTRIES} rows"));
    }
    let mut kinds: Vec<EvidenceKind> = Vec::with_capacity(raw.len());
    for (i, v) in raw.iter().enumerate() {
        let w = format!("{what} row {i}");
        let r = as_obj(v, &w)?;
        exact_fields(r, &["kind", "manifest", "patterns", "verdict", "per_run", "description"], &[], &w)?;
        let kind = text(r, "kind", &w)?;
        if kind.is_empty() || !kind.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
            return bad(format!("{w}: kind {kind:?} violates [a-z0-9-]+"));
        }
        let manifest = text(r, "manifest", &w)?;
        in_vocab(&manifest, &MANIFESTS, "manifest", &w)?;
        let verdict = text(r, "verdict", &w)?;
        in_vocab(&verdict, &["pass"], "verdict", &w)?;
        let per_run = r
            .get("per_run")
            .and_then(|b| b.as_bool())
            .ok_or_else(|| CatalogueError(format!("{w}: `per_run` must be a boolean")))?;
        nonempty_text(r, "description", &w)?;
        let raw_patterns = text_list(r, "patterns", &w)?;
        if raw_patterns.is_empty() {
            return bad(format!("{w}: `patterns` must not be empty"));
        }
        let mut patterns = Vec::new();
        for p in &raw_patterns {
            let seg = parse_pattern(p).map_err(|e| CatalogueError(format!("{w}: {e}")))?;
            // A kind's namespace says which manifest could ever hold it, so a
            // planned kind cannot be satisfied by today's manifests by accident.
            let first = match &seg[0] {
                Segment::Literal(l) => l.as_str(),
                Segment::Any => unreachable!("parse_pattern refuses a leading *"),
            };
            let ok = match manifest.as_str() {
                "test" | "proof" => first == "warrant",
                "bench" => first == "bench",
                _ => first != "warrant" && first != "bench",
            };
            if !ok {
                return bad(format!("{w}: pattern {p:?} is outside the namespace of manifest `{manifest}`"));
            }
            patterns.push(seg);
        }
        if kinds.iter().any(|k| k.kind == kind) {
            return bad(format!("{what}: kind `{kind}` is defined twice"));
        }
        kinds.push(EvidenceKind { kind, manifest, patterns, verdict, per_run });
    }
    // No evidence id may be claimable by two kinds (or twice by one).  Two
    // patterns can only overlap with equal length and the same first segment
    // (always a literal), so the pairwise check runs within those buckets.
    let total: usize = kinds.iter().map(|k| k.patterns.len()).sum();
    if total > MAX_TOTAL_PATTERNS {
        return bad(format!("{what}: more than {MAX_TOTAL_PATTERNS} patterns in total"));
    }
    // (pattern length, first segment) -> (kind, pattern) pairs in that bucket.
    type Bucket<'a> = Vec<(&'a str, &'a Vec<Segment>)>;
    let mut buckets: BTreeMap<(usize, String), Bucket> = BTreeMap::new();
    for k in &kinds {
        for p in &k.patterns {
            let head = match &p[0] {
                Segment::Literal(l) => l.clone(),
                Segment::Any => unreachable!("parse_pattern refuses a leading *"),
            };
            buckets.entry((p.len(), head)).or_default().push((k.kind.as_str(), p));
        }
    }
    for bucket in buckets.values() {
        for (i, (ka, pa)) in bucket.iter().enumerate() {
            for (kb, pb) in &bucket[i + 1..] {
                if patterns_overlap(pa, pb) {
                    return bad(format!("{what}: a pattern of `{ka}` overlaps a pattern of `{kb}`"));
                }
            }
        }
    }
    Ok(EvidenceKinds { kinds })
}

// ---------------------------------------------------------------------------
// non-claims.json
// ---------------------------------------------------------------------------

/// Parse and validate `non-claims.json`.
pub fn parse_non_claims(bytes: &[u8]) -> Result<Vec<NonClaim>, CatalogueError> {
    let what = "non-claims";
    let root = parse_root(bytes, what, NON_CLAIMS_SCHEMA)?;
    let o = as_obj(&root, what)?;
    exact_fields(o, &["schema", "non_claims"], &[], what)?;
    let raw = o
        .get("non_claims")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CatalogueError(format!("{what}: `non_claims` must be a list")))?;
    if raw.len() > MAX_ENTRIES {
        return bad(format!("{what}: more than {MAX_ENTRIES} rows"));
    }
    let mut out: Vec<NonClaim> = Vec::with_capacity(raw.len());
    for (i, v) in raw.iter().enumerate() {
        let w = format!("{what} row {i}");
        let r = as_obj(v, &w)?;
        exact_fields(r, &["package", "cwe", "claim", "source", "statement"], &[], &w)?;
        let package = text(r, "package", &w)?;
        if !is_package(&package, false) {
            return bad(format!("{w}: package {package:?} is not `lang/name`"));
        }
        let cwe = r
            .get("cwe")
            .and_then(|c| c.as_u32())
            .filter(|c| (1..=1_000_000).contains(c))
            .ok_or_else(|| CatalogueError(format!("{w}: `cwe` must be an integer in 1..=1000000")))?;
        let claim = text(r, "claim", &w)?;
        in_vocab(&claim, &CLAIM_KINDS, "claim", &w)?;
        let source = nonempty_text(r, "source", &w)?;
        if !source.starts_with("code/") || source.contains("..") || source.contains('\\') {
            return bad(format!("{w}: source {source:?} must be a repository path under code/"));
        }
        nonempty_text(r, "statement", &w)?;
        if out.iter().any(|n| n.package == package && n.cwe == cwe && n.claim == claim) {
            return bad(format!("{w}: duplicate non-claim"));
        }
        out.push(NonClaim { package, cwe, claim, source });
    }
    Ok(out)
}
