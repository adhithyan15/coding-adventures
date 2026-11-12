//! # Evidence manifests and report derivation (VER04h section 9)
//!
//! An assurance report may only quote numbers that a run wrote down.  This
//! module reads the manifests those runs produced
//!
//! * `evidence/test-manifest.json`  (generated Warrant test target),
//! * `evidence/proof-manifest.json` (Warrant-driven proof obligations),
//! * `evidence/bench-manifest.json` (microbenchmark target),
//!
//! plus the effect certificate and the Warrant document itself, and does two
//! things with them:
//!
//! 1. **derive** every claim number, label and certificate digest
//!    ([`derive_claims`]) and every `PackageAuditContext` flag
//!    ([`Evidence::audit_context`]) -- the generator contains no literal
//!    evidence;
//! 2. **cross-check** an existing report against them ([`verify_evidence`]),
//!    failing closed on unknown evidence ids, count/label drift, certificate
//!    files that do not match their hashes, Warrant claims without evidence,
//!    stale manifests, and a "no unsafe" claim next to a declared unsafe
//!    exception.
//!
//! Since VER13 (AS-001a part 2) [`Evidence::load`] also loads the assumption
//! catalogue found above the package directory ([`crate::catalogue`]), claims
//! cite catalogue ids, and [`verify_evidence`] runs the assumption rules
//! AC-1..AC-14 of [`crate::assumptions`] before comparing claims field by field.
//!
//! ## Hardening (security review B1-B9)
//!
//! Everything read from disk is untrusted until proven otherwise: manifests and
//! certificates are size-capped ([`MAX_MANIFEST_BYTES`], [`MAX_CERTIFICATE_BYTES`]),
//! JSON objects with duplicate keys are rejected by the parser, ids may not
//! repeat (within or across manifests) or collide with attestation ids, the
//! Warrant path must be the package's own spec and may not escape the
//! repository, the effect certificate must be well typed and is cross-checked
//! against a scan of `src/**/*.rs` for `unsafe`, and every claim is matched to
//! the derived one by exact id with no duplicates or extras.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::codegen::{extract_claim_cases, find_operation_in_expr};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, ParserLimits, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use crate::assumptions::{
    check_assumptions, derive_claim_assumptions, op_claim_kinds, remaining_by_layer, AssumptionCheckError,
    BASELINE_RUNG, HOST_STACK,
};
use crate::catalogue::CatalogueSet;
use crate::json::JsonValue;
use crate::matrix::{AuditEvidence, AuditProfile, PackageAuditContext, WeaknessStatus};
use crate::report::{
    AssuranceReport, ClaimEvidence, ExhaustiveEvidence, FuzzEvidence, ProofEvidence,
    PropertyEvidence, RungsSummary,
};

/// Why evidence could not be loaded or derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceError(pub String);

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "evidence error: {}", self.0)
    }
}

impl std::error::Error for EvidenceError {}

fn err<T>(msg: impl Into<String>) -> Result<T, EvidenceError> {
    Err(EvidenceError(msg.into()))
}

// ---------------------------------------------------------------------------
// Manifest models
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct TestEntry {
    pub id: String,
    pub kind: String,
    pub op: String,
    pub claim: String,
    pub verdict: String,
    pub cases: u64,
    pub iterations: u64,
    pub admitted: u64,
    pub equal_cases: u64,
    pub caught_by: String,
}

#[derive(Clone, Debug)]
pub struct CertRef {
    pub id: String,
    pub kind: String,
}

/// `CertRef::kind` of a certificate whose miter collapsed to a constant before
/// any SAT work: valid, but vacuous evidence the report must label as such.
pub const TRIVIAL_CERT_KIND: &str = "trivial-miter-collapsed";
/// `certificate_kind` value of a claim backed by at least one trivial certificate.
pub const TRIVIAL_MARK: &str = "trivial-miter";
/// `certificate_kind` value of a claim backed only by non-trivial miters.
pub const MITER_MARK: &str = "miter";

/// Largest manifest / effect certificate / spec / source file we will read.
pub const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
/// Largest DIMACS or DRAT certificate file we will read.
pub const MAX_CERTIFICATE_BYTES: u64 = 64 * 1024 * 1024;

/// Attestation that the Warrant document labels every operand of every
/// operation `public` (derived from the spec on disk by [`Evidence::load`];
/// see [`OpFacts::labelled_public`]).  It backs the "not applicable" CWE-208 row.
pub const NO_SECRET_OPERANDS_ATTESTATION: &str = "warrant-spec::all-operands-public";

/// The computed exemption that records CWE-208 as not applicable (no secret
/// operand, no constant-time obligation).  `--check` re-derives both facts.
pub const NOT_APPLICABLE_208_EXEMPTION: &str = "EXEMPT-CWE-208-NO-SECRET-OPERANDS";

/// Attestation ids derived from the effect certificate or the Warrant
/// document.  A manifest entry may not use one of these ids, or it could vouch
/// for them by itself.
pub const ATTESTATION_IDS: [&str; 13] = [
    "effect-certificate::proved",
    "analyzer::no-unsafe-or-ffi",
    "effect-certificate::unsafe-exception",
    "capability-audit::proc-empty",
    "capability-audit::fs-empty",
    "capability-audit::ui-empty",
    "capability-audit::stdout-empty",
    "capability-audit::rand-empty",
    "capability-audit::thread-empty",
    "capability-audit::no-external-resources",
    "capability-audit::no-database-sinks",
    "capability-audit::banned-constructs-clean",
    NO_SECRET_OPERANDS_ATTESTATION,
];

#[derive(Clone, Debug, Default)]
pub struct ProofEntry {
    pub id: String,
    pub op: String,
    pub function: String,
    pub kind: String,
    pub label: String,
    /// `pass`, `fail`, `unchecked` (resource not tracked) or `not-live` (the
    /// obligation was attempted and the tooling refused the source).
    pub verdict: String,
    pub max_len: u64,
    pub certificates: Vec<CertRef>,
    /// SHA-256 binding the entry to the lowered function and obligation
    /// (`warrant_proofs.rs`); empty when the manifest carries none.
    pub obligation_sha256: String,
    /// Free text the driver recorded (for `not-live`, the precise blocker).
    pub note: String,
}

impl ProofEntry {
    /// An obligation whose outcome is a verdict about the code, as opposed to
    /// a recorded inability to run it.
    fn is_live(&self) -> bool {
        self.verdict != "not-live" && self.verdict != "unchecked"
    }
}

/// Facts read from the Warrant document that the manifests must cover.
#[derive(Clone, Debug, Default)]
pub struct SpecFacts {
    pub module: String,
    pub sha256: String,
    /// (claim name, operation) in document order.
    pub claims: Vec<(String, String)>,
    pub examples: Vec<(String, String)>,
    /// Operations in document order with their evidence obligations.
    pub ops: Vec<OpFacts>,
}

#[derive(Clone, Debug, Default)]
pub struct OpFacts {
    pub name: String,
    pub has_secret: bool,
    pub has_steps_bound: bool,
    /// Every parameter of the operation has a `labels` row naming it (or one
    /// of its projections), and no row of the operation is `secret`.  An
    /// unlabelled parameter is *unclassified*, not public, so it leaves this
    /// `false`.
    pub labelled_public: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CertFacts {
    pub label: String,
    pub declared_effects: Vec<String>,
    /// Justifications of `banned_construct_exceptions` for `unsafe_code`.
    pub unsafe_exceptions: Vec<String>,
    pub other_exceptions: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Evidence {
    pub tests: Vec<TestEntry>,
    pub test_spec_sha256: String,
    pub test_source_sha256: String,
    pub bench_source_sha256: String,
    /// Files under `src/` whose code (not comments or strings) contains `unsafe`.
    pub unsafe_source_files: Vec<String>,
    pub proofs: Vec<ProofEntry>,
    pub proof_spec_sha256: String,
    pub proof_source_sha256: String,
    pub bench: Vec<(String, String)>,
    pub hw_instruction_counters: String,
    pub spec: SpecFacts,
    pub cert: CertFacts,
    pub source_sha256: String,
    pub proofs_dir: PathBuf,
    /// `rust/<package directory name>`: the package the evidence is about.
    pub package_id: String,
    /// The VER13 catalogue, evidence kinds and non-claims, with their digests.
    pub catalogue: CatalogueSet,
    /// SHA-256 (hex) of each manifest's bytes, by manifest name
    /// (`test`, `proof`, `bench`); AC-3 compares catalogue digests with it.
    pub manifest_sha256: BTreeMap<String, String>,
    /// Build tuples recorded by manifest entries that carry a `pinned`
    /// object, by evidence id (rule AC-9).  No manifest records one today.
    pub pinned: BTreeMap<String, BTreeMap<String, String>>,
}

/// Read `path` as bytes, refusing anything larger than `cap`.  The limit is
/// enforced on the bytes actually read (`take(cap + 1)`), not on a metadata
/// length that could change between the check and the read.
pub fn read_capped(path: &Path, cap: u64) -> Result<Vec<u8>, EvidenceError> {
    use std::io::Read as _;
    let file = std::fs::File::open(path)
        .map_err(|e| EvidenceError(format!("cannot read {}: {e}", path.display())))?;
    let mut bytes = Vec::new();
    file.take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| EvidenceError(format!("cannot read {}: {e}", path.display())))?;
    if bytes.len() as u64 > cap {
        return err(format!("{} is larger than the {cap}-byte limit", path.display()));
    }
    Ok(bytes)
}

fn parse(path: &Path) -> Result<JsonValue, EvidenceError> {
    Ok(parse_with_digest(path)?.0)
}

/// Parse a manifest and return the SHA-256 of the exact bytes parsed, so a
/// digest always describes what was read (no second read can differ).
fn parse_with_digest(path: &Path) -> Result<(JsonValue, String), EvidenceError> {
    let bytes = read_capped(path, MAX_MANIFEST_BYTES)?;
    let digest = sha256_hex(&bytes);
    let text = String::from_utf8(bytes).map_err(|_| EvidenceError(format!("{} is not UTF-8", path.display())))?;
    let v = JsonValue::parse(&text)
        .map_err(|e| EvidenceError(format!("{} is not valid JSON: {e}", path.display())))?;
    Ok((v, digest))
}

/// The optional `pinned` build tuple of a manifest entry (rule AC-9): all four
/// fields of [`crate::catalogue::PINNED_FIELDS`], non-empty strings.
fn record_pinned(
    entry: &JsonValue,
    id: &str,
    out: &mut BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), EvidenceError> {
    if let Some(p) = entry.get("pinned") {
        let tuple = crate::catalogue::parse_pinned_object(p, id, false).map_err(|e| EvidenceError(e.0))?;
        out.insert(id.to_string(), tuple);
    }
    Ok(())
}

/// A required string field; wrong type or absence is an error, not `""`.
fn req_text(v: &JsonValue, key: &str, what: &str) -> Result<String, EvidenceError> {
    match v.get(key) {
        Some(JsonValue::String(s)) => Ok(s.clone()),
        Some(_) => err(format!("{what}: `{key}` must be a string")),
        None => err(format!("{what}: missing `{key}`")),
    }
}

fn text(v: &JsonValue, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// Largest count a manifest may state: 2^53, the biggest integer a JSON
/// number (an `f64`) carries exactly.
pub const MAX_COUNT: u64 = 1 << 53;

/// A count field: absent is 0, but a present value must be a non-negative
/// integer no larger than [`MAX_COUNT`].  Negative, fractional, non-numeric
/// or oversized values fail closed (audit N3) instead of being clamped.
fn num(v: &JsonValue, key: &str) -> Result<u64, EvidenceError> {
    match v.get(key) {
        None => Ok(0),
        Some(x) => match x.as_i64().and_then(|n| u64::try_from(n).ok()) {
            Some(n) if n <= MAX_COUNT => Ok(n),
            _ => err(format!("`{key}` must be an integer between 0 and {MAX_COUNT}")),
        },
    }
}

fn schema(v: &JsonValue, want: &str, path: &Path) -> Result<(), EvidenceError> {
    if text(v, "schema") == want {
        Ok(())
    } else {
        err(format!("{} must have schema {want:?}", path.display()))
    }
}

fn load_spec(path: &Path) -> Result<SpecFacts, EvidenceError> {
    let source = read_capped(path, MAX_MANIFEST_BYTES)?;
    let doc = SourceDocument::admit(&source, SourceLimits::default())
        .map_err(|e| EvidenceError(format!("spec admission: {e:?}")))?;
    let layout = capture_layout(&doc, LayoutLimits::default())
        .map_err(|e| EvidenceError(format!("spec layout: {e:?}")))?;
    let file: WarrantFile<'_> = parse_document(&layout, &doc, ParserLimits::default())
        .map_err(|e| EvidenceError(format!("spec parse: {e:?}")))?;
    let mut facts = SpecFacts {
        module: file.name.to_string(),
        sha256: sha256_hex(&source),
        ..SpecFacts::default()
    };
    let op_names: BTreeSet<&str> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Op(o) => Some(o.name),
            _ => None,
        })
        .collect();
    let names_set: std::collections::HashSet<&str> = op_names.iter().copied().collect();
    for c in extract_claim_cases(&file).map_err(|e| EvidenceError(format!("spec claims: {e}")))? {
        facts.claims.push((c.id.to_string(), c.op_name));
    }
    for d in &file.declarations {
        match d {
            Declaration::Example(ex) => {
                let op = find_operation_in_expr(&ex.body, &names_set).unwrap_or("");
                facts.examples.push((ex.name.to_string(), op.to_string()));
            }
            Declaration::Op(o) => {
                let has_secret = file.declarations.iter().any(|x| {
                    matches!(x, Declaration::Labels(l) if l.name == o.name && l.rows.iter().any(|r| r.is_secret))
                });
                let has_steps_bound = file.declarations.iter().any(|x| {
                    matches!(x, Declaration::Bound(b) if b.name == o.name && b.rows.iter().any(|r| r.name == "steps"))
                });
                let labelled = |param: &str| {
                    file.declarations.iter().any(|x| {
                        matches!(x, Declaration::Labels(l) if l.name == o.name
                            && l.rows.iter().any(|r| r.name.segments.first() == Some(&param)))
                    })
                };
                let labelled_public = !has_secret && o.params.iter().all(|p| labelled(p.name));
                facts.ops.push(OpFacts {
                    name: o.name.to_string(),
                    has_secret,
                    has_steps_bound,
                    labelled_public,
                });
            }
            _ => {}
        }
    }
    Ok(facts)
}

/// Does an exception name `unsafe` or foreign-function code?  Any construct
/// whose name mentions either (case-insensitively) counts, so renaming the
/// construct (`Unsafe_Code`, `ffi_call`, `UNSAFE-block`) cannot hide it.
pub fn is_unsafe_bearing(construct: &str) -> bool {
    let c = construct.to_ascii_lowercase();
    c.contains("unsafe") || c.contains("ffi")
}

/// Parse the effect certificate strictly: every field the report relies on must
/// be present and of the right type.  A malformed certificate used to default to
/// "no effects, no exceptions", i.e. clean.
fn load_cert(path: &Path) -> Result<CertFacts, EvidenceError> {
    let v = parse(path)?;
    let what = "effect certificate";
    if v.as_object().is_none() {
        return err(format!("{what} must be a JSON object"));
    }
    let label = req_text(&v, "label", what)?;
    let list = |key: &str| -> Result<&[JsonValue], EvidenceError> {
        match v.get(key) {
            Some(x) => x
                .as_array()
                .ok_or_else(|| EvidenceError(format!("{what}: `{key}` must be an array"))),
            None => err(format!("{what}: missing `{key}`")),
        }
    };
    let mut declared_effects = Vec::new();
    for e in list("declared_effects")? {
        match e.as_str() {
            Some(s) => declared_effects.push(s.to_string()),
            None => return err(format!("{what}: `declared_effects` entries must be strings")),
        }
    }
    let mut facts = CertFacts {
        label,
        declared_effects,
        ..CertFacts::default()
    };
    for ex in list("banned_construct_exceptions")? {
        if ex.as_object().is_none() {
            return err(format!("{what}: `banned_construct_exceptions` entries must be objects"));
        }
        let construct = req_text(ex, "construct", what)?;
        let why = req_text(ex, "justification", what)?;
        if is_unsafe_bearing(&construct) {
            facts.unsafe_exceptions.push(why);
        } else {
            facts.other_exceptions.push(construct);
        }
    }
    Ok(facts)
}

/// Does Rust source text contain the keyword `unsafe` outside comments and
/// string/char literals?  A small lexer, deliberately conservative: it knows
/// line comments, nested block comments, `"..."`, `b"..."`, `c"..."`, raw
/// strings `r#"..."#` / `br#"..."#` / `cr#"..."#` with any number of `#`, and
/// char/byte literals, and treats every other occurrence of the whole word
/// `unsafe` (including inside attributes and macros) as unsafe code.
///
/// A char literal is exactly `'` + (one escape sequence | one character) + `'`
/// (escapes: `\'` `\"` `\\` `\n` `\r` `\t` `\0` `\xHH` `\u{...}`).  A `'` that
/// does not start a char literal must start a lifetime or label (`'a`,
/// `'static`, `'outer:`).
///
/// **Fails closed** (audit N1): when the lexer cannot positively tokenise the
/// file (unterminated comment, string or char literal, malformed escape, a
/// stray quote), it returns `true`, so a file the checker cannot understand is
/// treated as containing `unsafe` rather than silently skipped.
pub fn has_unsafe_token(src: &str) -> bool {
    scan_unsafe(src).unwrap_or(true)
}

/// `Some(found)` when the whole text tokenised, `None` when it did not.
fn scan_unsafe(src: &str) -> Option<bool> {
    let c: Vec<char> = src.chars().collect();
    let n = c.len();
    let mut i = 0;
    while i < n {
        match c[i] {
            '/' if c.get(i + 1) == Some(&'/') => {
                while i < n && c[i] != '\n' {
                    i += 1;
                }
            }
            '/' if c.get(i + 1) == Some(&'*') => {
                let mut depth = 1usize;
                i += 2;
                while i < n && depth > 0 {
                    if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                        depth += 1;
                        i += 2;
                    } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if depth > 0 {
                    return None;
                }
            }
            '"' => i = skip_string(&c, i + 1)?,
            '\'' => i = skip_quote(&c, i, false)?,
            ch if ch.is_alphabetic() || ch == '_' => {
                let start = i;
                while i < n && (c[i].is_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
                let word: String = c[start..i].iter().collect();
                match (word.as_str(), c.get(i)) {
                    ("b", Some('\'')) => {
                        i = skip_quote(&c, i, true)?;
                        continue;
                    }
                    ("b" | "c", Some('"')) => {
                        i = skip_string(&c, i + 1)?;
                        continue;
                    }
                    ("r" | "br" | "cr", Some('#' | '"')) => {
                        let mut hashes = 0;
                        let mut j = i;
                        while c.get(j) == Some(&'#') {
                            hashes += 1;
                            j += 1;
                        }
                        if c.get(j) == Some(&'"') {
                            i = skip_raw_string(&c, j + 1, hashes)?;
                            continue;
                        }
                    }
                    _ => {}
                }
                if word == "unsafe" {
                    return Some(true);
                }
            }
            _ => i += 1,
        }
    }
    Some(false)
}

/// Skip a `'` at `c[i]`: a char/byte literal or, when `byte` is false, a
/// lifetime/label.  Returns the index after it, or `None` when malformed.
fn skip_quote(c: &[char], i: usize, byte: bool) -> Option<usize> {
    let after = i + 1;
    let end = match c.get(after)? {
        '\\' => skip_escape(c, after + 1)?,
        '\'' | '\n' | '\r' => return None,
        _ => after + 1,
    };
    if c.get(end) == Some(&'\'') {
        return Some(end + 1);
    }
    // Not a literal: a lifetime or label, which is an identifier right after
    // the quote.  A byte literal can never be one.
    let first = *c.get(after)?;
    if byte || !(first.is_alphabetic() || first == '_') {
        return None;
    }
    let mut j = after;
    while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
        j += 1;
    }
    Some(j)
}

/// Skip the escape body after a backslash at `c[i - 1]`; returns the index
/// after it, or `None` for an unknown or malformed escape.
fn skip_escape(c: &[char], i: usize) -> Option<usize> {
    match c.get(i)? {
        '\'' | '"' | '\\' | 'n' | 'r' | 't' | '0' => Some(i + 1),
        'x' => (c.get(i + 1)?.is_ascii_hexdigit() && c.get(i + 2)?.is_ascii_hexdigit()).then_some(i + 3),
        'u' => {
            if c.get(i + 1) != Some(&'{') {
                return None;
            }
            let mut j = i + 2;
            let digits_start = j;
            while c.get(j)?.is_ascii_hexdigit() || c[j] == '_' {
                j += 1;
            }
            (j > digits_start && c[j] == '}').then_some(j + 1)
        }
        _ => None,
    }
}

/// Skip a string body (the opening quote is already consumed).
fn skip_string(c: &[char], mut i: usize) -> Option<usize> {
    while i < c.len() {
        match c[i] {
            '\\' => i += 2,
            '"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn skip_raw_string(c: &[char], mut i: usize, hashes: usize) -> Option<usize> {
    while i < c.len() {
        if c[i] == '"' && (0..hashes).all(|k| c.get(i + 1 + k) == Some(&'#')) {
            return Some(i + 1 + hashes);
        }
        i += 1;
    }
    None
}

/// Every `src/**/*.rs` file (symlinks are not followed) whose code contains
/// `unsafe`, as paths relative to `src`, sorted.
pub fn scan_unsafe_sources(package_dir: &Path) -> Result<Vec<String>, EvidenceError> {
    fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<String>) -> Result<(), EvidenceError> {
        if depth > 32 {
            return err("src tree is nested too deeply");
        }
        let entries = std::fs::read_dir(dir)
            .map_err(|e| EvidenceError(format!("cannot read {}: {e}", dir.display())))?;
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let rel_name = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let ty = entry
                .file_type()
                .map_err(|e| EvidenceError(format!("cannot stat {}: {e}", path.display())))?;
            if ty.is_dir() {
                walk(&path, &rel_name, depth + 1, out)?;
            } else if ty.is_file() && name.ends_with(".rs") {
                let bytes = read_capped(&path, MAX_MANIFEST_BYTES)?;
                if has_unsafe_token(&String::from_utf8_lossy(&bytes)) {
                    out.push(rel_name);
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(&package_dir.join("src"), "", 0, &mut out)?;
    Ok(out)
}

/// A manifest-supplied spec path must stay inside the repository: relative,
/// made only of normal components (no `..`, no root or drive prefix).
fn check_spec_path(relative: &str) -> Result<(), EvidenceError> {
    use std::path::Component;
    if relative.is_empty() || relative.contains('\\') {
        return err(format!("spec path {relative:?} is empty or contains a backslash"));
    }
    let p = Path::new(relative);
    if p.is_absolute() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return err(format!("spec path {relative:?} must be relative with no `..` components"));
    }
    Ok(())
}

/// Locate the Warrant document named by a manifest by walking up from the
/// package directory (the manifest stores a repository-relative path).
fn find_spec(package_dir: &Path, relative: &str) -> Result<PathBuf, EvidenceError> {
    check_spec_path(relative)?;
    let start = package_dir
        .canonicalize()
        .map_err(|e| EvidenceError(format!("package dir {}: {e}", package_dir.display())))?;
    for dir in start.ancestors() {
        let candidate = dir.join(relative);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    err(format!("spec {relative} not found above {}", package_dir.display()))
}

/// The Warrant document a package's evidence must describe: `ct-compare` is
/// specified by `code/specs/ct-compare.warrant`.  Pinning the path to the
/// package name stops a manifest from naming some other document.
fn expected_spec(package_dir: &Path) -> Result<String, EvidenceError> {
    let canon = package_dir
        .canonicalize()
        .map_err(|e| EvidenceError(format!("package dir {}: {e}", package_dir.display())))?;
    let name = canon
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| EvidenceError("package dir has no name".into()))?;
    Ok(format!("code/specs/{name}.warrant"))
}

/// A `not-live` proof entry is an acknowledged absence of proof, and it makes
/// the report *weaker* (no proof block, an unapproved exemption), so a manifest
/// must not be able to reach it by itself.  Each such entry must carry no
/// certificate, the label `not-live` and a non-empty blocker note, and its
/// operation must be declared `proofs_not_live=<reason>` in the package's
/// reviewed `evidence.bindings`.  Without that declaration the entry is
/// rejected, so dropping a proof to `not-live` is a visible change to a second
/// file, not a one-line edit to a generated manifest.
fn check_not_live_entries(package_dir: &Path, proofs: &[ProofEntry]) -> Result<(), EvidenceError> {
    let not_live: Vec<&ProofEntry> = proofs.iter().filter(|p| p.verdict == "not-live").collect();
    if not_live.is_empty() {
        return Ok(());
    }
    let path = package_dir.join("evidence.bindings");
    let declared = if path.is_file() {
        let bytes = read_capped(&path, MAX_MANIFEST_BYTES)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| EvidenceError(format!("{} is not UTF-8", path.display())))?;
        parse_evidence_bindings(&text)
            .map_err(|e| EvidenceError(format!("{}: {e}", path.display())))?
            .proofs_not_live
    } else {
        BTreeMap::new()
    };
    for p in not_live {
        if !p.certificates.is_empty() {
            return err(format!("{}: a `not-live` entry cannot cite certificates", p.id));
        }
        if p.label != "not-live" {
            return err(format!("{}: a `not-live` entry must carry the label `not-live`, not {:?}", p.id, p.label));
        }
        if p.note.trim().is_empty() {
            return err(format!("{}: a `not-live` entry must record the blocker in `note`", p.id));
        }
        if !declared.contains_key(&p.op) {
            return err(format!(
                "{}: recorded `not-live` but operation `{}` is not declared `proofs_not_live=<reason>` in {}",
                p.id,
                p.op,
                path.display()
            ));
        }
    }
    Ok(())
}

/// Record `id`, failing when it was already seen or collides with an
/// attestation id.
fn claim_id(seen: &mut BTreeSet<String>, id: &str, what: &str) -> Result<(), EvidenceError> {
    if id.is_empty() {
        return err(format!("{what} entry has an empty id"));
    }
    if ATTESTATION_IDS.contains(&id) || id.starts_with(UNVERIFIED_PREFIX) {
        return err(format!("{what} id `{id}` collides with a reserved attestation id"));
    }
    if !seen.insert(id.to_string()) {
        return err(format!("duplicate evidence id `{id}` ({what})"));
    }
    Ok(())
}

impl Evidence {
    /// Load every manifest under `<package_dir>/evidence`.
    pub fn load(package_dir: &Path) -> Result<Evidence, EvidenceError> {
        let dir = package_dir.join("evidence");
        let mut seen: BTreeSet<String> = BTreeSet::new();

        let mut manifest_sha256 = BTreeMap::new();
        let mut pinned = BTreeMap::new();
        let tpath = dir.join("test-manifest.json");
        let (tv, tdigest) = parse_with_digest(&tpath)?;
        manifest_sha256.insert("test".to_string(), tdigest);
        schema(&tv, "warrant-test-evidence-v1", &tpath)?;
        let mut tests = Vec::new();
        for t in tv
            .get("tests")
            .and_then(|x| x.as_array())
            .ok_or_else(|| EvidenceError("test manifest has no tests".into()))?
        {
            let what = "test manifest";
            let id = req_text(t, "id", what)?;
            claim_id(&mut seen, &id, what)?;
            record_pinned(t, &id, &mut pinned)?;
            tests.push(TestEntry {
                id,
                kind: req_text(t, "kind", what)?,
                op: text(t, "op"),
                claim: text(t, "claim"),
                verdict: req_text(t, "verdict", what)?,
                cases: num(t, "cases")?,
                iterations: num(t, "iterations")?,
                admitted: num(t, "admitted")?,
                equal_cases: num(t, "equal_cases")?,
                caught_by: text(t, "caught_by"),
            });
        }
        let spec_rel = req_text(&tv, "spec", "test manifest")?;

        // The proof manifest is required: every package's Warrant document
        // declares `labels` and `bound` rows, so there is always at least one
        // obligation to account for, as a verdict or as a `not-live` entry.
        let ppath = dir.join("proof-manifest.json");
        let (pv, pdigest) = parse_with_digest(&ppath)?;
        manifest_sha256.insert("proof".to_string(), pdigest);
        schema(&pv, "warrant-proof-evidence-v1", &ppath)?;
        let mut proofs = Vec::new();
        for p in pv
            .get("proofs")
            .and_then(|x| x.as_array())
            .ok_or_else(|| EvidenceError("proof manifest has no proofs".into()))?
        {
            let what = "proof manifest";
            let id = req_text(p, "id", what)?;
            claim_id(&mut seen, &id, what)?;
            record_pinned(p, &id, &mut pinned)?;
            let mut certificates = Vec::new();
            for c in p.get("certificates").and_then(|x| x.as_array()).unwrap_or(&[]) {
                let kind = req_text(c, "kind", what)?;
                if kind != TRIVIAL_CERT_KIND && kind != "miter" {
                    return err(format!("{id}: unknown certificate kind `{kind}`"));
                }
                certificates.push(CertRef {
                    id: req_text(c, "id", what)?,
                    kind,
                });
            }
            proofs.push(ProofEntry {
                id,
                op: text(p, "op"),
                function: text(p, "function"),
                kind: req_text(p, "kind", what)?,
                label: text(p, "label"),
                verdict: req_text(p, "verdict", what)?,
                max_len: num(p, "max_len")?,
                certificates,
                obligation_sha256: text(p, "obligation_sha256"),
                note: text(p, "note"),
            });
        }

        // Benchmarks are optional the same way: no manifest, no benchmark
        // evidence (and the matrix then cannot cite any).
        let bpath = dir.join("bench-manifest.json");
        let bv = if bpath.is_file() {
            let (bv, bdigest) = parse_with_digest(&bpath)?;
            manifest_sha256.insert("bench".to_string(), bdigest);
            schema(&bv, "warrant-bench-evidence-v1", &bpath)?;
            Some(bv)
        } else {
            None
        };
        let mut bench = Vec::new();
        for t in bv
            .as_ref()
            .and_then(|b| b.get("tests"))
            .and_then(|x| x.as_array())
            .unwrap_or(&[])
        {
            let what = "bench manifest";
            let id = req_text(t, "id", what)?;
            claim_id(&mut seen, &id, what)?;
            record_pinned(t, &id, &mut pinned)?;
            bench.push((id, req_text(t, "verdict", what)?));
        }

        // The Warrant document is the package's own, named identically by every
        // manifest, and every manifest describes the same module.
        let want_spec = expected_spec(package_dir)?;
        let proof_spec = req_text(&pv, "spec", "proof manifest")?;
        for (who, got) in [("test", &spec_rel), ("proof", &proof_spec)] {
            if *got != want_spec {
                return err(format!("{who} manifest names spec {got:?}, expected {want_spec:?}"));
            }
        }
        let spec_path = find_spec(package_dir, &want_spec)?;
        let spec = load_spec(&spec_path)?;
        for (who, v) in [("test", Some(&tv)), ("proof", Some(&pv)), ("bench", bv.as_ref())] {
            let Some(v) = v else { continue };
            let module = req_text(v, "module", &format!("{who} manifest"))?;
            if module != spec.module {
                return err(format!(
                    "{who} manifest is for module {module:?} but the spec defines {:?}",
                    spec.module
                ));
            }
        }

        check_not_live_entries(package_dir, &proofs)?;

        let cert = load_cert(&package_dir.join("effect_certificate.json"))?;
        // The binding covers every `src/**/*.rs` (audit N2), with the framing
        // the producers share via `test_bootstrap::manifest::frame_sources`.
        let source_framed = coding_adventures_test_bootstrap::manifest::source_tree_framing(&package_dir.join("src"))
            .map_err(|e| EvidenceError(format!("cannot bind evidence to {}/src: {e}", package_dir.display())))?;
        let unsafe_source_files = scan_unsafe_sources(package_dir)?;
        if !unsafe_source_files.is_empty() && cert.unsafe_exceptions.is_empty() {
            return err(format!(
                "source contains `unsafe` ({}) but the effect certificate declares no unsafe exception",
                unsafe_source_files.join(", ")
            ));
        }

        // The assumption catalogue comes from the same repository root as the
        // Warrant document (`<root>/code/specs/<name>.warrant`), and that root
        // must lie strictly above the package: a package cannot ship its own
        // catalogue (or spec tree) and be checked against it.
        let spec_root = spec_path
            .ancestors()
            .nth(want_spec.split('/').count())
            .ok_or_else(|| EvidenceError(format!("spec {} has no repository root", spec_path.display())))?;
        let catalogue = CatalogueSet::load_for_package(spec_root, package_dir)
            .map_err(|e| EvidenceError(e.to_string()))?;
        let package_id = format!(
            "rust/{}",
            want_spec
                .strip_prefix("code/specs/")
                .and_then(|s| s.strip_suffix(".warrant"))
                .ok_or_else(|| EvidenceError("package directory has no usable name".into()))?
        );

        Ok(Evidence {
            tests,
            test_spec_sha256: text(&tv, "spec_sha256"),
            test_source_sha256: text(&tv, "source_sha256"),
            // An absent (optional) bench manifest has no source binding to go
            // stale; a present one must match, exactly as before.
            bench_source_sha256: match bv.as_ref() {
                Some(b) => text(b, "source_sha256"),
                None => sha256_hex(&source_framed),
            },
            unsafe_source_files,
            proofs,
            proof_spec_sha256: text(&pv, "spec_sha256"),
            proof_source_sha256: text(&pv, "source_sha256"),
            bench,
            hw_instruction_counters: bv
                .as_ref()
                .map(|b| text(b, "hw_instruction_counters"))
                .unwrap_or_else(|| "not-live".to_string()),
            spec,
            cert,
            source_sha256: sha256_hex(&source_framed),
            proofs_dir: dir.join("proofs"),
            package_id,
            catalogue,
            manifest_sha256,
            pinned,
        })
    }

    /// Which manifest holds evidence id `id`, and the verdict it records.
    pub fn manifest_entry(&self, id: &str) -> Option<(&'static str, &str)> {
        if let Some(t) = self.tests.iter().find(|t| t.id == id) {
            return Some(("test", t.verdict.as_str()));
        }
        if let Some(p) = self.proofs.iter().find(|p| p.id == id) {
            return Some(("proof", p.verdict.as_str()));
        }
        self.bench.iter().find(|(b, _)| b == id).map(|(_, v)| ("bench", v.as_str()))
    }

    /// Every id a report may cite, with the verdict it carries.
    pub fn known_ids(&self) -> BTreeMap<String, String> {
        let mut ids = BTreeMap::new();
        for t in &self.tests {
            ids.insert(t.id.clone(), t.verdict.clone());
        }
        for p in &self.proofs {
            ids.insert(p.id.clone(), p.verdict.clone());
        }
        for (id, verdict) in &self.bench {
            ids.insert(id.clone(), verdict.clone());
        }
        for id in self.attestation_ids() {
            ids.insert(id, "pass".into());
        }
        ids
    }

    /// Attestations that follow from the effect certificate.  Each exists only
    /// while the certificate supports it, so a claim of "no unsafe" disappears
    /// the moment an `unsafe_code` exception is declared.
    pub fn attestation_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        let declared = &self.cert.declared_effects;
        let has = |prefix: &str| declared.iter().any(|d| d.starts_with(prefix));
        if self.cert.label == "proved" {
            ids.insert("effect-certificate::proved".to_string());
        }
        if self.cert.unsafe_exceptions.is_empty() && self.unsafe_source_files.is_empty() {
            ids.insert("analyzer::no-unsafe-or-ffi".into());
        } else {
            ids.insert("effect-certificate::unsafe-exception".into());
        }
        for (prefix, id) in [
            ("proc:", "capability-audit::proc-empty"),
            ("fs:", "capability-audit::fs-empty"),
            ("ui:", "capability-audit::ui-empty"),
            ("stdout:", "capability-audit::stdout-empty"),
            ("rand:", "capability-audit::rand-empty"),
            ("thread:", "capability-audit::thread-empty"),
        ] {
            if !has(prefix) {
                ids.insert(id.into());
            }
        }
        if !has("fs:") && !has("proc:") {
            ids.insert("capability-audit::no-external-resources".into());
        }
        if declared.is_empty() {
            ids.insert("capability-audit::no-database-sinks".into());
        }
        if self.cert.other_exceptions.is_empty() {
            ids.insert("capability-audit::banned-constructs-clean".into());
        }
        if self.spec_all_operands_public() {
            ids.insert(NO_SECRET_OPERANDS_ATTESTATION.into());
        }
        ids
    }

    /// Does the Warrant document declare at least one operation and label
    /// every operand of every operation `public`?  (Read from the spec on disk,
    /// so a report cannot assert it.)
    pub fn spec_all_operands_public(&self) -> bool {
        !self.spec.ops.is_empty() && self.spec.ops.iter().all(|o| o.labelled_public)
    }

    fn entries<'a>(&'a self, kind: &'a str, op: &'a str) -> impl Iterator<Item = &'a TestEntry> {
        self.tests.iter().filter(move |t| t.kind == kind && t.op == op)
    }

    fn all_pass_nonempty<'a>(entries: impl Iterator<Item = &'a TestEntry>) -> bool {
        let v: Vec<_> = entries.collect();
        !v.is_empty() && v.iter().all(|t| t.verdict == "pass")
    }

    fn proof_obligations<'a>(&'a self, op: &'a str) -> impl Iterator<Item = &'a ProofEntry> {
        self.proofs
            .iter()
            .filter(move |p| p.op == op && (p.kind == "constant-time" || p.kind == "steps-bound"))
    }

    /// Package facts for the weakness matrix, every one read from a manifest.
    pub fn audit_context(&self, package_id: &str) -> PackageAuditContext {
        self.audit_context_with_profile(package_id, AuditProfile::Comparison)
    }

    /// [`Evidence::audit_context`] for a package whose attestation rows are
    /// not about comparison primitives.
    pub fn audit_context_with_profile(
        &self,
        package_id: &str,
        profile: AuditProfile,
    ) -> PackageAuditContext {
        // Every constant-time and steps-bound obligation is considered, live or
        // not (as on main before the generalisation): a `not-live` or
        // `unchecked` entry is an operation with no proof, and it must block a
        // discharge that the other operations' passing proofs would otherwise
        // support.  Only a proven entry (verdict `pass`, ranked label) is ever
        // *cited* as evidence.
        let ct: Vec<&ProofEntry> = self.proofs.iter().filter(|p| p.kind == "constant-time").collect();
        let steps: Vec<&ProofEntry> = self.proofs.iter().filter(|p| p.kind == "steps-bound").collect();
        let proven = |p: &ProofEntry| p.verdict == "pass" && label_rank(&p.label).is_some();
        let ids_of = |v: &[&ProofEntry], keep: &dyn Fn(&ProofEntry) -> bool| -> Vec<String> {
            v.iter().filter(|p| keep(p)).map(|p| p.id.clone()).collect()
        };
        let (ct_unproven, steps_unproven) = (
            ids_of(&ct, &|p| !proven(p)),
            ids_of(&steps, &|p| !proven(p)),
        );
        let not_live = |kind: &str| -> Vec<&ProofEntry> {
            self.proofs
                .iter()
                .filter(|p| p.kind == kind && p.verdict == "not-live")
                .collect()
        };
        let (ct_nl, steps_nl) = (not_live("constant-time"), not_live("steps-bound"));
        let mut notes: Vec<&str> = ct_nl.iter().chain(steps_nl.iter()).map(|p| p.note.as_str()).collect();
        notes.sort_unstable();
        notes.dedup();
        let proof_ok = |v: &[&ProofEntry]| !v.is_empty() && v.iter().all(|p| proven(p));
        let bench_ct: Vec<&(String, String)> = self
            .bench
            .iter()
            .filter(|(id, _)| id.starts_with("bench/ct_"))
            .collect();
        let bench_ok = !bench_ct.is_empty() && bench_ct.iter().all(|(_, v)| v == "pass");
        let weakest = |v: &[&ProofEntry]| {
            v.iter()
                .map(|p| p.label.clone())
                .min_by_key(|l| label_rank(l).unwrap_or(0))
                .unwrap_or_default()
        };
        let evidence = AuditEvidence {
            unsafe_exceptions: self.cert.unsafe_exceptions.clone(),
            step_bound_ids: ids_of(&steps, &proven),
            step_bound_label: weakest(&steps),
            alloc_ids: bench_ct.iter().map(|(id, _)| id.clone()).collect(),
            ct_proof_ids: ids_of(&ct, &proven),
            ct_proof_label: weakest(&ct),
            bench_ids: bench_ct.iter().map(|(id, _)| id.clone()).collect(),
            hw_counters: self.hw_instruction_counters.clone(),
            // Bounds-checking is claimed for the lowered code only when every
            // obligation ran; one unproven operation leaves the row open.
            oob_ids: if ct_unproven.is_empty() && steps_unproven.is_empty() {
                ct.iter().chain(steps.iter()).map(|p| p.id.clone()).collect()
            } else {
                Vec::new()
            },
            oob_label: weakest(&ct),
            ct_unproven,
            step_unproven: steps_unproven,
            ct_not_live: ct_nl.iter().map(|p| p.id.clone()).collect(),
            step_not_live: steps_nl.iter().map(|p| p.id.clone()).collect(),
            not_live_note: notes.join(" | "),
            no_secret_operand_ids: if self.spec_all_operands_public() {
                vec![NO_SECRET_OPERANDS_ATTESTATION.to_string()]
            } else {
                Vec::new()
            },
            profile,
        };
        PackageAuditContext {
            package_id: package_id.to_string(),
            is_safe_rust: self.cert.unsafe_exceptions.is_empty() && self.unsafe_source_files.is_empty(),
            declared_capabilities: self.cert.declared_effects.clone(),
            has_wir_timing_proof: proof_ok(&ct),
            has_instruction_benchmarks: bench_ok,
            has_zero_alloc_proof: bench_ok,
            has_pbt_coverage: Self::all_pass_nonempty(self.tests.iter().filter(|t| t.kind == "property")),
            has_fuzz_coverage: Self::all_pass_nonempty(self.tests.iter().filter(|t| t.kind == "fuzz")),
            evidence,
        }
    }

    /// Names of the operations the Warrant document states claims about: the
    /// ones whose exhaustive, property and fuzz evidence is *required*.
    fn claimed_ops(&self) -> BTreeSet<&str> {
        self.spec.claims.iter().map(|(_, op)| op.as_str()).collect()
    }

    /// Does every claimed operation have a non-empty, all-passing entry set of
    /// each of `kinds`?  Per operation, not pooled: one operation's passing
    /// suites must not paper over another operation with none.
    fn per_op_ok(&self, kinds: &[&str]) -> bool {
        let ops = self.claimed_ops();
        !ops.is_empty()
            && ops.iter().all(|op| {
                kinds.iter().all(|k| Self::all_pass_nonempty(self.entries(k, op)))
            })
    }

    /// Does every operation of the document have at least one constant-time or
    /// steps-bound obligation, all of which passed with a ranked label?
    /// (`unknown`, `falsified`, `exceeded`, `unchecked` have no rank.)
    fn proofs_ok(&self) -> bool {
        !self.spec.ops.is_empty()
            && self.spec.ops.iter().all(|op| {
                let v: Vec<&ProofEntry> = self.proof_obligations(&op.name).collect();
                !v.is_empty() && v.iter().all(|p| p.verdict == "pass" && label_rank(&p.label).is_some())
            })
    }

    /// Rung statuses computed from which kinds of evidence exist and pass.
    pub fn rungs(&self) -> RungsSummary {
        let status = |ok: bool| if ok { "pass" } else { "fail" }.to_string();
        let kind_ok =
            |k: &[&str]| Self::all_pass_nonempty(self.tests.iter().filter(|t| k.contains(&t.kind.as_str())));
        // E5 is `pass` only when every operation of the document has at
        // least one obligation and every obligation passed with a ranked label
        // (the hardened `proofs_ok`).  An obligation recorded `not-live`
        // (attempted, refused by the tooling, blocker in `note`) is an
        // acknowledged absence, so E5 reads `not-live` -- but only when every
        // *other* obligation is a ranked pass.  Anything else (a verdict that
        // is not `pass`, `unchecked`, an unranked label, an operation with no
        // obligation at all) is `fail`: never weaker than `proofs_ok`.
        let e5 = {
            let ok_or_not_live = |p: &ProofEntry| {
                p.verdict == "not-live" || (p.verdict == "pass" && label_rank(&p.label).is_some())
            };
            let covered = !self.spec.ops.is_empty()
                && self.spec.ops.iter().all(|op| self.proof_obligations(&op.name).next().is_some());
            let all_fine = self
                .spec
                .ops
                .iter()
                .all(|op| self.proof_obligations(&op.name).all(ok_or_not_live));
            if !covered || !all_fine {
                "fail".to_string()
            } else if self.proofs_ok() {
                "pass".to_string()
            } else {
                "not-live".to_string()
            }
        };
        RungsSummary {
            e0: status(self.cert.label == "proved" && self.cert.declared_effects.is_empty()),
            e1: status(kind_ok(&["example"])),
            // No manifest of simulated-host or integration runs exists.
            e2: "not-live".into(),
            e3: status(self.per_op_ok(&["property", "exhaustive"])),
            e4: status(self.per_op_ok(&["fuzz"])),
            e5,
        }
    }
}

/// Order of proof labels: `proved` is strongest, `bounded(k)` ranks by `k`.
/// Anything else (`unknown`, `falsified`, `unchecked`) has no rank.
pub fn label_rank(label: &str) -> Option<u64> {
    if label == "proved" {
        return Some(u64::MAX);
    }
    label
        .strip_prefix("bounded(")
        .and_then(|r| r.strip_suffix(')'))
        .and_then(|k| k.parse::<u64>().ok())
}

// ---------------------------------------------------------------------------
// Claim derivation
// ---------------------------------------------------------------------------

fn status(ok: bool) -> String {
    if ok { "pass" } else { "fail" }.to_string()
}

/// Digest of the certificates backing `op`: SHA-256 over the sorted, unique
/// certificate ids, newline terminated.  Each id is itself the SHA-256 of the
/// certificate's `cnf ++ 0x00 ++ drat` bytes, which `verify_evidence`
/// recomputes from the files on disk.
pub fn certificate_digest(ev: &Evidence, op: &str) -> Option<String> {
    let ids: BTreeSet<&str> = ev
        .proof_obligations(op)
        .flat_map(|p| p.certificates.iter().map(|c| c.id.as_str()))
        .collect();
    if ids.is_empty() {
        return None;
    }
    let mut joined = String::new();
    for id in ids {
        joined.push_str(id);
        joined.push('\n');
    }
    Some(format!("drat:sha256:{}", sha256_hex(joined.as_bytes())))
}

/// How the certificates backing `op` should be described in the report:
/// [`TRIVIAL_MARK`] when any of them is a miter that collapsed to a constant
/// (valid, but the SAT solver and DRAT checker had nothing to do), else
/// [`MITER_MARK`].  The mark is derived, never typed in, and `--check` fails a
/// report that omits it.
pub fn certificate_kind(ev: &Evidence, op: &str) -> &'static str {
    let trivial = ev
        .proof_obligations(op)
        .flat_map(|p| p.certificates.iter())
        .any(|c| c.kind == TRIVIAL_CERT_KIND);
    if trivial { TRIVIAL_MARK } else { MITER_MARK }
}

/// Derive one claim per Warrant operation from the manifests.
pub fn derive_claims(ev: &Evidence) -> Result<Vec<ClaimEvidence>, EvidenceError> {
    let mut claims = Vec::new();
    for op in &ev.spec.ops {
        let obligations: Vec<&ProofEntry> = ev.proof_obligations(&op.name).collect();
        let Some(first) = obligations.first() else {
            return err(format!("operation `{}` has no proof obligations in the manifest", op.name));
        };
        let symbol = first.function.clone();
        // Checked: a hostile manifest must not wrap the totals (audit N3).
        let sum = |kind: &str, f: fn(&TestEntry) -> u64| -> Result<u64, EvidenceError> {
            ev.entries(kind, &op.name).try_fold(0u64, |acc, t| {
                acc.checked_add(f(t)).ok_or_else(|| {
                    EvidenceError(format!("{kind} counts for operation `{}` overflow", op.name))
                })
            })
        };
        let weakest = obligations
            .iter()
            .map(|p| p.label.clone())
            .min_by_key(|l| label_rank(l).unwrap_or(0))
            .unwrap_or_default();
        let proofs_pass = obligations.iter().all(|p| p.verdict == "pass");
        let proof_label = if proofs_pass && label_rank(&weakest).is_some() {
            weakest
        } else if obligations.iter().any(|p| p.label == "falsified") {
            "falsified".to_string()
        } else {
            "unknown".to_string()
        };
        // The strings the evidence layer has always emitted are kept as
        // *seeds*: each must resolve to a catalogue id or alias, so the
        // migration to catalogue ids cannot lose one of them.
        let buffers = obligations.iter().any(|p| p.max_len > 0 && p.label != "proved");
        let mut legacy = vec!["compiler.preserves_timing".to_string()];
        if buffers {
            legacy.push("memory.unaliased".to_string());
        }
        // Rows the prover does not discharge stay visible as assumptions
        // (`heap-bound:unchecked` is an alias of `wir.bound.heap-unchecked`).
        for p in ev.proofs.iter().filter(|p| p.op == op.name && p.verdict == "unchecked") {
            legacy.push(format!("{}:unchecked", p.kind));
        }
        // Every obligation recorded `not-live` means no proof exists for this
        // operation: the claim carries no proof block at all (never a label),
        // and the manifest's note says why.
        let proof = if obligations.iter().all(|p| p.verdict == "not-live") {
            None
        } else {
            let certificate = certificate_digest(ev, &op.name)
                .ok_or_else(|| EvidenceError(format!("operation `{}` has no certificates", op.name)))?;
            // Rule AC-4, applied by construction: every catalogue entry whose
            // `applies_to` matches this label, these claim kinds and this
            // package, closed over the catalogue edges.
            let kinds = op_claim_kinds(ev, &op.name);
            let (assumptions, discharged) =
                derive_claim_assumptions(&ev.catalogue, &ev.package_id, &proof_label, &kinds, &legacy, HOST_STACK)
                    .map_err(|e| EvidenceError(format!("operation `{}`: {e}", op.name)))?;
            Some(ProofEvidence {
                remaining_by_layer: remaining_by_layer(&ev.catalogue.catalogue, &assumptions),
                label: proof_label,
                assumptions,
                certificate,
                certificate_kind: certificate_kind(ev, &op.name).to_string(),
                stack: HOST_STACK.to_string(),
                rung: BASELINE_RUNG.to_string(),
                discharged,
            })
        };

        let negctl_ok = Evidence::all_pass_nonempty(ev.entries("negctl", &op.name));
        let mut evidence_ids: Vec<String> = ev
            .tests
            .iter()
            .filter(|t| t.op == op.name)
            .map(|t| t.id.clone())
            .chain(
                ev.proofs
                    .iter()
                    .filter(|p| p.op == op.name && p.is_live())
                    .map(|p| p.id.clone()),
            )
            .collect();
        evidence_ids.sort();
        claims.push(ClaimEvidence {
            id: format!("{symbol}_soundness"),
            symbol,
            example: status(Evidence::all_pass_nonempty(ev.entries("example", &op.name))),
            property: PropertyEvidence {
                cases: sum("property", |t| t.cases)?,
                status: status(
                    Evidence::all_pass_nonempty(ev.entries("property", &op.name)) && negctl_ok,
                ),
            },
            exhaustive: Some(ExhaustiveEvidence {
                cases: sum("exhaustive", |t| t.cases)?,
                status: status(Evidence::all_pass_nonempty(ev.entries("exhaustive", &op.name))),
            }),
            fuzz: Some(FuzzEvidence {
                iterations: sum("fuzz", |t| t.iterations)?,
                status: status(Evidence::all_pass_nonempty(ev.entries("fuzz", &op.name))),
            }),
            proof,
            evidence: evidence_ids,
        });
    }
    Ok(claims)
}

// ---------------------------------------------------------------------------
// Fail-closed verification
// ---------------------------------------------------------------------------

/// Failure modes of cross-checking a report against its evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceCheckError {
    /// An evidence id is absent from every manifest.
    UnknownEvidence(String),
    /// A cited manifest entry did not pass.
    FailingEvidence(String),
    /// A count, label or digest disagrees with the manifests.
    EvidenceMismatch(String),
    /// A certificate file is missing, mis-hashed, or rejected by the DRAT checker.
    BadCertificate(String),
    /// The Warrant document declares something with no evidence.
    UncoveredClaim(String),
    /// A manifest was produced from a different spec or source than is on disk.
    StaleEvidence(String),
    /// The report claims no-unsafe while the certificate declares an exception.
    UnsafeContradiction(String),
    /// The manifests could not be read.
    Load(String),
    /// The same claim id appears twice in the report.
    DuplicateId(String),
    /// A trivial (collapsed-miter) certificate is presented without the
    /// `certificate_kind: "trivial-miter"` marking.
    TrivialCertificateUnmarked(String),
    /// A VER13 assumption rule (AC-1..AC-14) failed.
    Assumption(AssumptionCheckError),
}

impl fmt::Display for EvidenceCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (name, m) = match self {
            Self::UnknownEvidence(m) => ("unknown evidence id", m),
            Self::FailingEvidence(m) => ("cited evidence did not pass", m),
            Self::EvidenceMismatch(m) => ("report disagrees with evidence manifest", m),
            Self::BadCertificate(m) => ("bad certificate", m),
            Self::UncoveredClaim(m) => ("Warrant declaration without evidence", m),
            Self::StaleEvidence(m) => ("stale evidence", m),
            Self::UnsafeContradiction(m) => ("unsafe claim contradicts declared exception", m),
            Self::Load(m) => ("cannot load evidence", m),
            Self::DuplicateId(m) => ("duplicate id", m),
            Self::TrivialCertificateUnmarked(m) => ("trivial certificate presented as a full proof", m),
            Self::Assumption(a) => return write!(f, "{a}"),
        };
        write!(f, "{name}: {m}")
    }
}

impl std::error::Error for EvidenceCheckError {}

/// Memory-safety CWE classes that "safe Rust" would exclude.
pub const MEMORY_SAFETY_CWES: [u32; 6] = [416, 787, 125, 415, 476, 190];

/// Evidence ids that are not machine-checked attestations (VER04h section 10).
pub const UNVERIFIED_PREFIX: &str = "unverified::";

/// Cross-check `report` against `ev`.  On success returns the ids that were
/// accepted only as `unverified::` attestations, so callers can surface them.
pub fn verify_evidence(
    report: &AssuranceReport,
    ev: &Evidence,
) -> Result<Vec<String>, EvidenceCheckError> {
    use EvidenceCheckError as E;

    // 0. The report is about the package the evidence describes.
    if report.package != ev.package_id {
        return Err(E::EvidenceMismatch(format!(
            "report is for {} but the evidence is for {}",
            report.package, ev.package_id
        )));
    }

    // 1. Staleness: manifests must describe the spec and source on disk.
    if ev.test_spec_sha256 != ev.spec.sha256 {
        return Err(E::StaleEvidence(format!(
            "test manifest was generated from spec {} but the spec on disk is {}",
            ev.test_spec_sha256, ev.spec.sha256
        )));
    }
    if ev.proof_spec_sha256 != ev.spec.sha256 {
        return Err(E::StaleEvidence("proof manifest was generated from a different spec".into()));
    }
    if ev.proof_source_sha256 != ev.source_sha256 {
        return Err(E::StaleEvidence(
            "proof manifest was generated from different source (src/**/*.rs)".into(),
        ));
    }

    if ev.test_source_sha256 != ev.source_sha256 {
        return Err(E::StaleEvidence(
            "test manifest was generated from different source (src/**/*.rs)".into(),
        ));
    }
    if ev.bench_source_sha256 != ev.source_sha256 {
        return Err(E::StaleEvidence(
            "bench manifest was generated from different source (src/**/*.rs)".into(),
        ));
    }

    // 1b. Source/certificate cross-check: `unsafe` in the code needs a declared
    //     exception, whatever the certificate's own fields say.
    if !ev.unsafe_source_files.is_empty() && ev.cert.unsafe_exceptions.is_empty() {
        return Err(E::UnsafeContradiction(format!(
            "source contains `unsafe` ({}) but the effect certificate declares no unsafe exception",
            ev.unsafe_source_files.join(", ")
        )));
    }

    // 1c. Every discharged obligation cites a certificate and is bound to the
    //     obligation it proves.  (Negative controls and `unchecked` rows prove
    //     nothing and are exempt.)  Full recomputation of the binding from the
    //     source is follow-up WD-009 (VER04h section 10).
    for p in ev.proofs.iter().filter(|p| p.kind == "constant-time" || p.kind == "steps-bound") {
        if p.verdict != "pass" {
            continue;
        }
        if p.certificates.is_empty() {
            return Err(E::BadCertificate(format!("{}: verdict pass but no certificate is cited", p.id)));
        }
        let h = &p.obligation_sha256;
        if h.len() != 64 || !h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(E::BadCertificate(format!(
                "{}: obligation_sha256 is missing or not 64 lowercase hex digits",
                p.id
            )));
        }
    }

    // 2. Certificates: file present, name == sha256(cnf ++ 0 ++ drat), and the
    //    DRAT checker still accepts it.
    let mut seen = BTreeSet::new();
    for p in &ev.proofs {
        for c in &p.certificates {
            if !seen.insert(c.id.clone()) {
                continue;
            }
            let cnf = read_capped(&ev.proofs_dir.join(format!("{}.cnf", c.id)), MAX_CERTIFICATE_BYTES)
                .map_err(|e| E::BadCertificate(format!("{}: cnf unreadable: {}", c.id, e.0)))?;
            let drat = read_capped(&ev.proofs_dir.join(format!("{}.drat", c.id)), MAX_CERTIFICATE_BYTES)
                .map_err(|e| E::BadCertificate(format!("{}: drat unreadable: {}", c.id, e.0)))?;
            let mut bytes = cnf.clone();
            bytes.push(0);
            bytes.extend_from_slice(&drat);
            if sha256_hex(&bytes) != c.id {
                return Err(E::BadCertificate(format!(
                    "{}: file contents hash to {}",
                    c.id,
                    sha256_hex(&bytes)
                )));
            }
            let (cnf_s, drat_s) = (String::from_utf8_lossy(&cnf), String::from_utf8_lossy(&drat));
            drat_check::check(&cnf_s, &drat_s)
                .map_err(|e| E::BadCertificate(format!("{}: DRAT checker rejected: {e}", c.id)))?;
        }
    }

    // 3. Coverage: everything the Warrant document declares has evidence.
    for (name, op) in &ev.spec.claims {
        for kind in ["exhaustive", "property", "fuzz"] {
            let id = format!("warrant/{}/{kind}/{name}", ev.spec.module);
            if !ev.tests.iter().any(|t| t.id == id) {
                return Err(E::UncoveredClaim(format!("claim `{name}` (op {op}) has no {kind} entry")));
            }
        }
    }
    for (name, _) in &ev.spec.examples {
        let id = format!("warrant/{}/example/{name}", ev.spec.module);
        if !ev.tests.iter().any(|t| t.id == id) {
            return Err(E::UncoveredClaim(format!("example `{name}` has no entry")));
        }
    }
    for op in &ev.spec.ops {
        let has = |kind: &str| ev.proofs.iter().any(|p| p.op == op.name && p.kind == kind);
        if op.has_secret && !has("constant-time") {
            return Err(E::UncoveredClaim(format!("op `{}` has secret labels but no constant-time proof", op.name)));
        }
        if op.has_steps_bound && !has("steps-bound") {
            return Err(E::UncoveredClaim(format!("op `{}` has a steps bound but no bound proof", op.name)));
        }
        if !report.claims.iter().any(|c| {
            ev.proofs.iter().any(|p| p.op == op.name && p.function == c.symbol)
        }) {
            return Err(E::UncoveredClaim(format!("op `{}` has no claim in the report", op.name)));
        }
    }

    // 3b. A declared unsafe exception forbids a "no unsafe" exclusion.  Judged
    //     on structured fields only (status, CWE class, evidence ids), never on
    //     the free-text reason, which a rewording would defeat:
    //       * no row may cite the no-unsafe attestation;
    //       * no memory-safety row may be `excluded`;
    //       * the classes the bounds-checked proofs do not cover (416, 415, 476,
    //         190) must stay unapproved exemptions;
    //       * 125/787 may be `discharged` only on a proof-manifest id.
    if !ev.cert.unsafe_exceptions.is_empty() {
        let proof_ids: BTreeSet<&str> = ev.proofs.iter().map(|p| p.id.as_str()).collect();
        for w in &report.weaknesses {
            let memory = MEMORY_SAFETY_CWES.contains(&w.cwe);
            let bounds_only = matches!(w.cwe, 125 | 787);
            let cites_attestation = w.evidence.iter().any(|e| e == "analyzer::no-unsafe-or-ffi");
            let excluded = w.status == WeaknessStatus::Excluded && memory;
            let uncovered_class = memory && !bounds_only && w.status != WeaknessStatus::Exemption;
            let unproven_bounds = bounds_only
                && w.status == WeaknessStatus::Discharged
                && !w.evidence.iter().any(|e| proof_ids.contains(e.as_str()));
            if cites_attestation || excluded || uncovered_class || unproven_bounds {
                return Err(E::UnsafeContradiction(format!(
                    "CWE-{} is {} without evidence compatible with the declared unsafe_code exception",
                    w.cwe,
                    w.status.as_str()
                )));
            }
        }
    }
    // 4. Every evidence id the report cites exists and passed.
    let known = ev.known_ids();
    let mut unverified = Vec::new();
    let mut check_ids = |ids: &[String]| -> Result<(), E> {
        for id in ids {
            if id.starts_with(UNVERIFIED_PREFIX) {
                unverified.push(id.clone());
                continue;
            }
            match known.get(id) {
                None => return Err(E::UnknownEvidence(id.clone())),
                Some(v) if v != "pass" => {
                    return Err(E::FailingEvidence(format!("{id} has verdict {v}")))
                }
                Some(_) => {}
            }
        }
        Ok(())
    };
    for c in &report.claims {
        check_ids(&c.evidence)?;
    }
    for w in &report.weaknesses {
        check_ids(&w.evidence)?;
    }
    for x in &report.exemptions {
        check_ids(&x.evidence)?;
    }

    // 4b. VER13 assumption rules (AC-1..AC-14), before the field-by-field
    //     comparison below, so a broken assumption list is named by the rule it
    //     breaks rather than reported as a generic mismatch.  AC-10 (unused
    //     entries) is a warning here; `--strict` makes the CLI fail on it.
    check_assumptions(report, ev, false).map_err(E::Assumption)?;

    // 5. Counts, labels and digests equal what the manifests say.  Claims are
    //    matched by exact id; an id may appear once, every derived claim must be
    //    present, and nothing else may be.
    let derived = derive_claims(ev).map_err(|e| E::Load(e.0))?;
    let mut claim_ids = BTreeSet::new();
    for c in &report.claims {
        if !claim_ids.insert(c.id.as_str()) {
            return Err(E::DuplicateId(format!("claim `{}` appears more than once", c.id)));
        }
    }
    for want in &derived {
        let Some(got) = report.claims.iter().find(|c| c.id == want.id) else {
            return Err(E::UncoveredClaim(format!("report has no claim `{}`", want.id)));
        };
        if let (Some(wp), got_proof) = (&want.proof, got.proof.as_ref()) {
            if wp.certificate_kind == TRIVIAL_MARK
                && got_proof.map(|p| p.certificate_kind.as_str()) != Some(TRIVIAL_MARK)
            {
                return Err(E::TrivialCertificateUnmarked(format!(
                    "claim {} is backed by a collapsed-miter certificate and must say certificate_kind \"{TRIVIAL_MARK}\"",
                    want.id
                )));
            }
        }
        let fields: Vec<(&str, String, String)> = vec![
            ("symbol", got.symbol.clone(), want.symbol.clone()),
            ("example status", got.example.clone(), want.example.clone()),
            ("property cases", got.property.cases.to_string(), want.property.cases.to_string()),
            ("property status", got.property.status.clone(), want.property.status.clone()),
            (
                "exhaustive",
                format!("{:?}", got.exhaustive),
                format!("{:?}", want.exhaustive),
            ),
            ("fuzz", format!("{:?}", got.fuzz), format!("{:?}", want.fuzz)),
            ("proof", format!("{:?}", got.proof), format!("{:?}", want.proof)),
            ("evidence ids", format!("{:?}", got.evidence), format!("{:?}", want.evidence)),
        ];
        for (what, g, w) in fields {
            if g != w {
                return Err(E::EvidenceMismatch(format!(
                    "claim {}: {what} is {g} but the manifest says {w}",
                    want.id
                )));
            }
        }
    }
    for c in &report.claims {
        if !derived.iter().any(|d| d.id == c.id) {
            return Err(E::UnknownEvidence(format!("claim `{}` is backed by no manifest entry", c.id)));
        }
    }
    if report.claims.len() != derived.len() {
        return Err(E::EvidenceMismatch(format!(
            "report has {} claims but the manifests derive {}",
            report.claims.len(),
            derived.len()
        )));
    }
    // 6. Weakness rows that rest on a proof kind are re-derived against every
    //    obligation of that kind, live or not.  A report whose CWE-400 row
    //    claims a discharge, or whose CWE-208 / CWE-400 row cites proof ids,
    //    while any steps-bound / constant-time obligation is not a ranked
    //    pass (`not-live`, `unchecked`, `fail`...) is rejected here, so the
    //    checker enforces what the generator does.
    for (cwe, kind) in [(400u32, "steps-bound"), (208u32, "constant-time")] {
        let of_kind: Vec<&ProofEntry> = ev.proofs.iter().filter(|p| p.kind == kind).collect();
        let unproven: Vec<&str> = of_kind
            .iter()
            .filter(|p| !(p.verdict == "pass" && label_rank(&p.label).is_some()))
            .map(|p| p.id.as_str())
            .collect();
        if unproven.is_empty() {
            continue;
        }
        for w in report.weaknesses.iter().filter(|w| w.cwe == cwe) {
            let cites_proof = w.evidence.iter().any(|e| of_kind.iter().any(|p| p.id == *e));
            if w.status == WeaknessStatus::Discharged || cites_proof {
                return Err(E::EvidenceMismatch(format!(
                    "CWE-{cwe} is {} on {kind} proofs, but {} obligation(s) are not proven ({})",
                    w.status.as_str(),
                    unproven.len(),
                    unproven.join(", ")
                )));
            }
        }
    }
    // 6b. The "not applicable" CWE-208 exemption is re-derived, not taken
    //     from the report: the spec on disk must label every operand public
    //     and the proof manifest must hold no constant-time obligation at all.
    //     (Citing the attestation where it does not hold already failed step 4
    //     as an unknown id; this catches the exemption id used without it.)
    //
    //     Both directions are closed: the exemption id re-derives the facts, and
    //     a CWE-208 row citing the attestation must carry that exemption (the
    //     attestation is evidence for "not applicable" and for nothing else, so
    //     citing it under, say, the gate-level exemption is a forged
    //     classification even when every fact it names is true).
    let cites_attestation = report
        .weaknesses
        .iter()
        .any(|w| w.cwe == 208 && w.evidence.iter().any(|e| e == NO_SECRET_OPERANDS_ATTESTATION));
    let has_not_applicable = report.exemptions.iter().any(|x| x.id == NOT_APPLICABLE_208_EXEMPTION);
    if cites_attestation && !has_not_applicable {
        return Err(E::EvidenceMismatch(format!(
            "a CWE-208 row cites {NO_SECRET_OPERANDS_ATTESTATION} but the report does not carry \
             {NOT_APPLICABLE_208_EXEMPTION}; the attestation supports only that exemption"
        )));
    }
    if has_not_applicable {
        let ct: Vec<&str> =
            ev.proofs.iter().filter(|p| p.kind == "constant-time").map(|p| p.id.as_str()).collect();
        if !ev.spec_all_operands_public() || !ct.is_empty() {
            return Err(E::EvidenceMismatch(format!(
                "{NOT_APPLICABLE_208_EXEMPTION} needs every operand labelled public and no constant-time \
                 obligation, but the spec {} and the proof manifest has {} constant-time obligation(s)",
                if ev.spec_all_operands_public() { "does" } else { "does not" },
                ct.len()
            )));
        }
    }
    let want_rungs = ev.rungs();
    if report.rungs != want_rungs {
        return Err(E::EvidenceMismatch(format!(
            "rungs are {:?} but the manifests say {:?}",
            report.rungs, want_rungs
        )));
    }

    Ok(unverified)
}

// ---------------------------------------------------------------------------
// Package identity
// ---------------------------------------------------------------------------

/// The `version = "..."` of the `[package]` table of a `Cargo.toml`.
///
/// Only that table is consulted: a `version = "..."` line in `[dependencies]`
/// (spelled one key per line), in a `[dependencies.foo]` sub-table or in
/// `[workspace.package]` is not the package's version and must not be reported
/// as it.  Comments and blank lines are skipped; `version.workspace = true` is
/// refused because the version then lives elsewhere.
pub fn package_version(cargo_toml: &str) -> Result<String, EvidenceError> {
    let mut in_package = false;
    for raw in cargo_toml.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_package = line.split('#').next().map(str::trim_end) == Some("[package]");
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key == "version.workspace" {
            return err("Cargo.toml [package] inherits its version from the workspace".to_string());
        }
        if key != "version" {
            continue;
        }
        if let Some(rest) = value.trim().strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                let version = &rest[..end];
                let tail = rest[end + 1..].trim();
                if !version.is_empty() && (tail.is_empty() || tail.starts_with('#')) {
                    return Ok(version.to_string());
                }
            }
        }
        return err("Cargo.toml [package] has a malformed `version`".to_string());
    }
    err("Cargo.toml has no `version = \"...\"` in its [package] table".to_string())
}
