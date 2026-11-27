//! The small text files that steer the gate: the chain, the sign-off and the allow-list.
//!
//! # One strict line format
//!
//! All three are written like PIF itself (VER32 4.1): ASCII, one line per entry, fields
//! separated by exactly one space, every line ended by LF, no blank lines, no comments, no
//! leading or trailing space. The first line is `<magic> 1`. The strictness is deliberate:
//! a lenient parser here would be a second, unreviewed notion of "the same chain".
//!
//! ```text
//!   pif-chain 1                         pif-signoff 1                  pif-allow 1 (<role> <digest>)
//!   prelude sha256:<64 hex>             base prelude sha256:<hex>      sha256:<64 hex>
//!   adl     sha256:<64 hex>             base model   sha256:<hex>      sha256:<64 hex>
//!   model   sha256:<64 hex>             base-environment <hex>
//!   claim   sha256:<64 hex>             goal <claim-id> <hex>
//!                                       goals-sha256 <hex>
//! ```
//!
//! VER31 does not yet define a sign-off file, so the format above is this crate's reading of
//! what VER32 E1, E6, G6 and PIF-020 need a sign-off to carry. It is the only place the gate
//! invents a format, and the README says so.
//!
//! # Roles
//!
//! A chain is the PIF prelude, then ADL layers, then libraries, then the model, then an
//! optional axioms file, and last, on the proof side, the claim file (VER32 9.2, 3.7). The
//! role of a file decides which reader rules apply to it ([`FileClass`]):
//! prelude, adl, model and axioms are *reviewed* (R-NAM-3, the ASCII class), a library is not,
//! and the claim gets the strictest class (R-AX-1, R-GOL-4).

use crate::digest::Digest;
use crate::error::CheckError;
use lean_kernel::pif::FileClass;
use std::collections::BTreeSet;

/// What a chain position means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Prelude,
    Adl,
    Library,
    Model,
    Axioms,
    Claim,
}

impl Role {
    /// The word used in chain files and reports.
    pub fn word(self) -> &'static str {
        match self {
            Role::Prelude => "prelude",
            Role::Adl => "adl",
            Role::Library => "library",
            Role::Model => "model",
            Role::Axioms => "axioms",
            Role::Claim => "claim",
        }
    }

    fn from_word(w: &str) -> Option<Role> {
        [
            Role::Prelude,
            Role::Adl,
            Role::Library,
            Role::Model,
            Role::Axioms,
            Role::Claim,
        ]
        .into_iter()
        .find(|r| r.word() == w)
    }

    /// The reader rule set for a file in this position. The axioms file is *reviewed*:
    /// it holds unchecked assumptions, so a homoglyph in its names must not be possible.
    pub fn class(self) -> FileClass {
        match self {
            Role::Library => FileClass::Library,
            Role::Claim => FileClass::Claim,
            _ => FileClass::Reviewed,
        }
    }
}

/// One position of a chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainEntry {
    pub role: Role,
    pub digest: Digest,
}

/// The longest chain: 64 imports plus the file itself (VER32 3.7).
pub const MAX_CHAIN: usize = 65;

/// Split a control file into rows of single-space-separated fields, enforcing the strict
/// line format. Returns the rows with their 1-based line numbers.
pub(crate) fn rows<'a>(
    what: &'static str,
    text: &'a str,
    magic: &str,
) -> Result<Vec<(usize, Vec<&'a str>)>, CheckError> {
    let bad = |line, why| CheckError::BadControlFile { what, line, why };
    if text.is_empty() {
        return Err(bad(1, "empty file"));
    }
    let body = text
        .strip_suffix('\n')
        .ok_or_else(|| bad(1, "the last line has no LF"))?;
    let mut out = Vec::new();
    for (i, line) in body.split('\n').enumerate() {
        let n = i + 1;
        if line.is_empty() || line.starts_with(' ') || line.ends_with(' ') || line.contains("  ") {
            return Err(bad(n, "blank line or irregular spacing"));
        }
        if !line
            .bytes()
            .all(|b| (0x21..=0x7e).contains(&b) || b == b' ')
        {
            return Err(bad(n, "a byte outside printable ASCII"));
        }
        out.push((n, line.split(' ').collect::<Vec<_>>()));
    }
    match out.first() {
        Some((_, f)) if f.as_slice() == [magic, "1"] => {}
        _ => return Err(bad(1, "missing or unknown header line")),
    }
    out.remove(0);
    Ok(out)
}

pub(crate) fn digest_at(what: &'static str, line: usize, s: &str) -> Result<Digest, CheckError> {
    Digest::parse(s).ok_or(CheckError::BadControlFile {
        what,
        line,
        why: "not a sha256 digest",
    })
}

/// Parse a chain file into its entries (shape is checked by [`check_shape`]).
pub fn parse_chain(text: &str) -> Result<Vec<ChainEntry>, CheckError> {
    let mut out = Vec::new();
    for (n, f) in rows("chain", text, "pif-chain")? {
        let (Some(role), [_, d]) = (Role::from_word(f[0]), f.as_slice()) else {
            return Err(CheckError::BadControlFile {
                what: "chain",
                line: n,
                why: "expected `<role> <digest>`",
            });
        };
        out.push(ChainEntry {
            role,
            digest: digest_at("chain", n, d)?,
        });
        if out.len() > MAX_CHAIN {
            return Err(CheckError::ChainTooLong);
        }
    }
    Ok(out)
}

/// E1 to E4 on the shape of a chain: non-empty, at most 65 files, the prelude first, roles in
/// order, once-only roles once, the claim file last.
///
/// The claim check comes first so that a claim file in a base position is reported as exactly
/// that (`ClaimNotLast`) and not as a generic ordering problem.
pub fn check_shape(chain: &[ChainEntry]) -> Result<(), CheckError> {
    if chain.is_empty() {
        return Err(CheckError::EmptyChain);
    }
    if chain.len() > MAX_CHAIN {
        return Err(CheckError::ChainTooLong);
    }
    if chain[..chain.len() - 1]
        .iter()
        .any(|e| e.role == Role::Claim)
    {
        return Err(CheckError::ClaimNotLast);
    }
    if chain[0].role != Role::Prelude {
        return Err(CheckError::PreludeNotFirst);
    }
    for (index, pair) in chain.windows(2).enumerate().map(|(i, p)| (i + 1, p)) {
        let (a, b) = (pair[0].role, pair[1].role);
        if b < a {
            return Err(CheckError::RoleOrder { index });
        }
        let once = !matches!(b, Role::Adl | Role::Library);
        if once && b == a {
            return Err(CheckError::DuplicateRole { index });
        }
    }
    Ok(())
}

/// G3: `[A-Za-z0-9_.-]{1,128}`.
pub fn claim_id_ok(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

/// A parsed sign-off: the base chain the signer fixed (E1) and what the signature covers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Signoff {
    /// The ordered base chain (prelude to model/axioms). Never contains a claim file.
    pub base: Vec<ChainEntry>,
    /// The base environment digest the signer recorded (E6, G6), if any.
    pub base_environment: Option<Digest>,
    /// The signed goals: claim id and goal digest.
    pub goals: Vec<(String, Digest)>,
    /// The signed aggregate of the goals (PIF-020), if any.
    pub goals_sha256: Option<Digest>,
}

/// Parse a sign-off file.
pub fn parse_signoff(text: &str) -> Result<Signoff, CheckError> {
    let bad = |line, why| CheckError::BadControlFile {
        what: "sign-off",
        line,
        why,
    };
    let mut s = Signoff::default();
    for (n, f) in rows("sign-off", text, "pif-signoff")? {
        match f.as_slice() {
            ["base", role, d] => {
                let role = Role::from_word(role).ok_or_else(|| bad(n, "unknown role"))?;
                if role == Role::Claim {
                    // E1: a sign-off records the base; the claim file is what is checked.
                    return Err(CheckError::ClaimNotLast);
                }
                s.base.push(ChainEntry {
                    role,
                    digest: digest_at("sign-off", n, d)?,
                });
                if s.base.len() >= MAX_CHAIN {
                    return Err(CheckError::ChainTooLong);
                }
            }
            ["base-environment", d] if s.base_environment.is_none() => {
                s.base_environment = Some(digest_at("sign-off", n, d)?);
            }
            ["goal", id, d] => {
                // G3, and no duplicates: a repeated id would let the last line win silently.
                if !claim_id_ok(id) {
                    return Err(bad(n, "bad claim id"));
                }
                if s.goals.iter().any(|(c, _)| c == id) {
                    return Err(bad(n, "duplicate goal id"));
                }
                s.goals
                    .push(((*id).to_string(), digest_at("sign-off", n, d)?));
            }
            ["goals-sha256", d] if s.goals_sha256.is_none() => {
                s.goals_sha256 = Some(digest_at("sign-off", n, d)?);
            }
            _ => return Err(bad(n, "unknown or repeated entry")),
        }
    }
    // A sign-off that omits any of these would be applied only in part, silently.
    if s.base_environment.is_none() {
        return Err(bad(0, "missing base-environment"));
    }
    if s.goals.is_empty() {
        return Err(bad(0, "missing goal lines"));
    }
    if s.goals_sha256.is_none() {
        return Err(bad(0, "missing goals-sha256"));
    }
    Ok(s)
}

/// Parse an allow-list: a header and one `<role> <digest>` per line. Binding the role matters:
/// an allow-listed model placed under the role `library` would otherwise skip the reviewed-file
/// checks (the ASCII class and the printer round trip).
pub fn parse_allow(text: &str) -> Result<BTreeSet<(Role, Digest)>, CheckError> {
    let mut out = BTreeSet::new();
    for (n, f) in rows("allow-list", text, "pif-allow")? {
        let (Some(role), [_, d]) = (Role::from_word(f[0]), f.as_slice()) else {
            return Err(CheckError::BadControlFile {
                what: "allow-list",
                line: n,
                why: "expected `<role> <digest>`",
            });
        };
        if role == Role::Claim {
            // E4: the claim file is not on the allow-list, by design.
            return Err(CheckError::ClaimNotLast);
        }
        out.insert((role, digest_at("allow-list", n, d)?));
    }
    Ok(out)
}

/// E1: the chain to load. Without a sign-off it is the chain file as written. With one, the
/// base comes **from the sign-off**; the chain file may repeat that base (it must then match
/// exactly) or list only the claim file.
pub fn effective_chain(
    chain: Vec<ChainEntry>,
    signoff: Option<&Signoff>,
) -> Result<Vec<ChainEntry>, CheckError> {
    let Some(s) = signoff else { return Ok(chain) };
    let (claim, base): (Vec<_>, Vec<_>) = chain.into_iter().partition(|e| e.role == Role::Claim);
    if !base.is_empty() && base != s.base {
        return Err(CheckError::SignoffChainMismatch);
    }
    let mut out = s.base.clone();
    out.extend(claim);
    Ok(out)
}
