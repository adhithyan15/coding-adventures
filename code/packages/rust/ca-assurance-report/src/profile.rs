//! # Audit profiles are data (VER04h section 9, item 22)
//!
//! Three rows of the weakness matrix are *static attestations*: CWE-326
//! (inadequate key size), CWE-327 (broken cipher) and CWE-407 (algorithmic
//! complexity) are excluded for a package like `gf256` or `wasm-leb128` by a
//! sentence that says why ("no keys: the package takes and generates no key
//! material ..."), not by a derived fact.  Which sentence is true depends on what
//! the package *is*, so the matrix used to carry one `match` arm per kind of
//! package, and every new adopter edited this crate.
//!
//! An analogy: the matrix is a customs officer, and the profile is the traveller's
//! declaration form.  The form says what the traveller carries and in what words;
//! the officer still weighs the luggage.  Here the *wording* and the *declaration
//! that a row is excluded* come from the package's own
//! `assurance-profile.json`, while everything that decides whether a declaration
//! may stand (the checks against the effect certificate, the Warrant document and
//! the test manifest) stays in [`crate::matrix`] and cannot be edited by data.
//!
//! ```text
//!  assurance-profile.json  --(sha256 of the exact bytes)-->  report.profile_sha256
//!        |                                                        |
//!        |  name, kind, rows{326,327,407[,409]}                   |  --check recomputes
//!        v                                                        v  and compares
//!  AuditProfile  --(wording only)-->  matrix  <--(evidence decides)--  Evidence
//! ```
//!
//! ## What a profile may and may not say
//!
//! * `kind` is a closed vocabulary with meaning in code: `transform` (nothing
//!   expands attacker-controlled input) or `decompressor`, whose CWE-409 row can
//!   never be excluded.
//! * `rows` must declare 326, 327 and 407, and may declare 409 for a `transform`
//!   only.  No other CWE is declarable: memory safety, CWE-208, CWE-400 and the
//!   capability rows are computed from evidence and a file cannot touch them.
//! * `applicability` has exactly one value, `excluded`.  A profile cannot declare
//!   a row discharged, an exemption or approved.
//! * Every object has exactly the keys documented; an unknown key is refused (a
//!   typo must not silently drop a claim).

use std::collections::BTreeMap;
use std::path::Path;

use coding_adventures_sha256::sha256_hex;

use crate::evidence::{read_capped, MAX_MANIFEST_BYTES};
use crate::json::JsonValue;

/// File name, beside the package's `Cargo.toml`.
pub const PROFILE_FILE: &str = "assurance-profile.json";
/// The only schema string accepted.
pub const PROFILE_SCHEMA: &str = "warrant-assurance-profile-v1";
/// CWEs a profile must declare: the three static attestation rows.
pub const REQUIRED_ROWS: [u32; 3] = [326, 327, 407];
/// CWEs a profile may declare at all.
pub const DECLARABLE_ROWS: [u32; 4] = [326, 327, 407, 409];
/// Longest reason text, in bytes.
pub const MAX_REASON_BYTES: usize = 2048;
/// Suffix the matrix appends itself to rows resting on `unverified::` evidence;
/// a profile may not spell it, so a reason cannot fake or duplicate the marker.
pub const RESERVED_SUFFIX: &str = "[attestation not machine-checked";

/// A profile file that cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError(pub String);

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assurance profile: {}", self.0)
    }
}

impl std::error::Error for ProfileError {}

fn bad<T>(msg: impl Into<String>) -> Result<T, ProfileError> {
    Err(ProfileError(msg.into()))
}

/// What the package does to attacker-controlled input.  The meaning of each
/// value is code, not data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProfileKind {
    /// Nothing expands its input.  CWE-409 is excluded unless the Warrant
    /// document declares an `output` bound row.
    #[default]
    Transform,
    /// Expands attacker-controlled input (a decompressor).  CWE-409 applies by
    /// definition: it is discharged from `output`/`heap` evidence or the report
    /// cannot be generated.
    Decompressor,
}

impl std::str::FromStr for ProfileKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "transform" => Ok(ProfileKind::Transform),
            "decompressor" => Ok(ProfileKind::Decompressor),
            other => Err(format!("unknown kind `{other}` (transform | decompressor)")),
        }
    }
}

impl ProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileKind::Transform => "transform",
            ProfileKind::Decompressor => "decompressor",
        }
    }
}

/// The audit profile of one package, parsed from `assurance-profile.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditProfile {
    /// Label the report records (`[a-z0-9-]{1,64}`).
    pub name: String,
    pub kind: ProfileKind,
    /// CWE -> the reason sentence of a declared exclusion.
    rows: BTreeMap<u32, String>,
    /// SHA-256 (hex) of the exact bytes the profile was parsed from.
    pub sha256: String,
}

impl AuditProfile {
    /// Does a package with this profile expand attacker-controlled input?
    /// CWE-409 is then applicable by definition, whatever the document says.
    pub fn decompresses(&self) -> bool {
        self.kind == ProfileKind::Decompressor
    }

    /// The declared reason for `cwe`, if the profile declares that row.
    pub fn row_reason(&self, cwe: u32) -> Option<&str> {
        self.rows.get(&cwe).map(String::as_str)
    }

    /// Hold the profile to the pin a package's BUILD line passes (`--profile NAME`
    /// and `--profile-kind KIND`).  `name` and `kind` are both author data, so the
    /// name alone pins only a label; the kind is what the security rule keys on
    /// (a decompressor's CWE-409 is never excluded), so the BUILD line pins it too
    /// and an author cannot flip it in the file and regenerate.
    pub fn verify_pin(&self, name: Option<&str>, kind: Option<ProfileKind>) -> Result<(), ProfileError> {
        if let Some(want) = name {
            if want != self.name {
                return bad(format!("--profile `{want}` but {PROFILE_FILE} is named `{}`", self.name));
            }
        }
        if let Some(want) = kind {
            if want != self.kind {
                return bad(format!(
                    "--profile-kind `{}` but {PROFILE_FILE} declares kind `{}`",
                    want.as_str(),
                    self.kind.as_str()
                ));
            }
        }
        Ok(())
    }

    /// Read `<package_dir>/assurance-profile.json`.  A missing file is an
    /// error: there is no default profile to fall back on.
    pub fn load(package_dir: &Path) -> Result<AuditProfile, ProfileError> {
        let path = package_dir.join(PROFILE_FILE);
        let bytes = read_capped(&path, MAX_MANIFEST_BYTES).map_err(|e| ProfileError(e.0))?;
        AuditProfile::from_bytes(&bytes)
    }

    /// Parse and validate a profile.  The digest is taken over `bytes` exactly,
    /// before any parsing, so it always describes what was read.
    pub fn from_bytes(bytes: &[u8]) -> Result<AuditProfile, ProfileError> {
        let sha256 = sha256_hex(bytes);
        let text = std::str::from_utf8(bytes).or_else(|_| bad("not UTF-8"))?;
        let v = JsonValue::parse(text).or_else(|e| bad(format!("not valid JSON: {e}")))?;
        let obj = v.as_object().ok_or_else(|| ProfileError("must be a JSON object".into()))?;
        exact_keys(obj, &["schema", "name", "kind", "rows"], "profile")?;

        match obj.get("schema").and_then(|s| s.as_str()) {
            Some(PROFILE_SCHEMA) => {}
            Some(other) => return bad(format!("schema `{other}` is not `{PROFILE_SCHEMA}`")),
            None => return bad(format!("missing `schema` (`{PROFILE_SCHEMA}`)")),
        }

        let name = string_field(obj, "name")?;
        if name.is_empty() || name.len() > 64 || !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
            return bad(format!("`name` {name:?} must match [a-z0-9-]{{1,64}}"));
        }
        let kind: ProfileKind = string_field(obj, "kind")?.parse().map_err(ProfileError)?;

        let rows_obj = obj
            .get("rows")
            .and_then(|r| r.as_object())
            .ok_or_else(|| ProfileError("missing `rows` object".into()))?;
        let mut rows = BTreeMap::new();
        for (key, row) in rows_obj {
            let cwe: u32 = key
                .parse()
                .ok()
                .filter(|c: &u32| c.to_string() == *key)
                .ok_or_else(|| ProfileError(format!("row key `{key}` is not a CWE number")))?;
            if !DECLARABLE_ROWS.contains(&cwe) {
                return bad(format!(
                    "CWE-{cwe} cannot be declared by a profile (declarable: 326, 327, 407, 409); \
                     the matrix computes that row from evidence"
                ));
            }
            let row_obj = row
                .as_object()
                .ok_or_else(|| ProfileError(format!("row {cwe} must be an object")))?;
            exact_keys(row_obj, &["applicability", "reason"], &format!("row {cwe}"))?;
            match string_field(row_obj, "applicability")?.as_str() {
                "excluded" => {}
                other => {
                    return bad(format!(
                        "row {cwe}: applicability `{other}` is not supported; a profile can only declare `excluded`"
                    ))
                }
            }
            let reason = string_field(row_obj, "reason")?;
            check_reason(cwe, &reason)?;
            rows.insert(cwe, reason);
        }
        for cwe in REQUIRED_ROWS {
            if !rows.contains_key(&cwe) {
                return bad(format!("missing required row {cwe}: nothing defaults to a sentence"));
            }
        }
        // The rule that a decompressor's CWE-409 is never excluded: such a
        // profile cannot even be loaded with a 409 exclusion in it.
        if kind == ProfileKind::Decompressor && rows.contains_key(&409) {
            return bad(
                "a `decompressor` cannot declare CWE-409 excluded: the class applies and must be \
                 discharged from `output`/`heap` bound rows",
            );
        }
        Ok(AuditProfile { name, kind, rows, sha256 })
    }
}

fn exact_keys(obj: &BTreeMap<String, JsonValue>, keys: &[&str], what: &str) -> Result<(), ProfileError> {
    for k in obj.keys() {
        if !keys.contains(&k.as_str()) {
            return bad(format!("{what} has unknown key `{k}`"));
        }
    }
    for k in keys {
        if !obj.contains_key(*k) {
            return bad(format!("{what} is missing `{k}`"));
        }
    }
    Ok(())
}

fn string_field(obj: &BTreeMap<String, JsonValue>, key: &str) -> Result<String, ProfileError> {
    match obj.get(key) {
        Some(JsonValue::String(s)) => Ok(s.clone()),
        Some(_) => bad(format!("`{key}` must be a string")),
        None => bad(format!("missing `{key}`")),
    }
}

/// One line of printable ASCII, no padding, not the reserved suffix.
fn check_reason(cwe: u32, reason: &str) -> Result<(), ProfileError> {
    if reason.is_empty() || reason.len() > MAX_REASON_BYTES {
        return bad(format!("row {cwe}: reason must be 1 to {MAX_REASON_BYTES} bytes"));
    }
    if !reason.bytes().all(|b| (b' '..=b'~').contains(&b)) {
        return bad(format!("row {cwe}: reason must be one line of printable ASCII"));
    }
    if reason.trim() != reason {
        return bad(format!("row {cwe}: reason has leading or trailing whitespace"));
    }
    if reason.contains(RESERVED_SUFFIX) {
        return bad(format!("row {cwe}: reason may not contain the reserved marker `{RESERVED_SUFFIX}`"));
    }
    Ok(())
}
