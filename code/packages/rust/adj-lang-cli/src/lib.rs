//! Shared surface for the `adj-lang-cli` binaries.
//!
//! Two programs now read `.adj` sources and print JSON: `adj-lang-cli`, which
//! answers a query, and `adj-verify`, which re-checks the answer's trail. They
//! must agree on three things that are easy to get subtly wrong twice:
//!
//! 1. **JSON escaping** ([`esc`]) — a quoted span is untrusted text lifted from
//!    a spidered page. An unescaped newline in line-oriented output lets that
//!    span forge a `step 3 verified` line in the trail it appears in.
//! 2. **The sensitive channel** ([`payload`], [`query_echo`]) — echoed values
//!    can be chart text. A second binary that re-implemented the check would
//!    eventually disagree with the first, and the disagreement would be silent.
//! 3. **The import sandbox** ([`FsProvider`]) — `import` must not escape the
//!    program's own directory. Duplicating a path-containment check is how one
//!    copy ends up missing the `starts_with` guard.
//!
//! So these live in one place and both binaries link against it. The rule of
//! thumb this encodes: a security check that exists twice exists zero times.

use std::fs;
use std::path::{Path, PathBuf};

use adj_lang::ImportProvider;

/// JSON-escape a string body.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Longest echoed payload the abstention object will carry.
///
/// `ADJ-REASON-MATH.md` §E.4 requires echoed payloads to be length-capped. The
/// reason is not display tidiness: these fields carry the CALLER'S OWN INPUT
/// back out into an artifact that is designed to be replayed and shared, and an
/// unbounded echo is both an amplification and a bigger blast radius for
/// whatever the caller happened to paste in.
pub const ABSTENTION_FIELD_CAP: usize = 256;

/// `true` when this run's input is marked sensitive, so echoed payloads must be
/// withheld (`ADJ_SENSITIVE_INPUT=1`).
///
/// §E.4 requires redaction on a sensitive channel, and names the case: in the
/// medical arm an unresolved surface form can be free text lifted from a chart,
/// and the trail it lands in travels. The `reason` and `explanation` still tell
/// you WHAT went wrong — only the echoed values are withheld, so an abstention
/// stays actionable without carrying PHI along with it.
pub fn sensitive_input() -> bool {
    // Resolved ONCE per process. Two reasons beyond speed: the warning below
    // should be emitted once rather than once per rendered field (it printed
    // four times for a single query before this), and a security decision that
    // is re-read mid-run could in principle disagree with itself.
    static DECIDED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *DECIDED.get_or_init(decide_sensitive_input)
}

fn decide_sensitive_input() -> bool {
    let Ok(raw) = std::env::var("ADJ_SENSITIVE_INPUT") else {
        return false;
    };
    let v = raw.trim();
    if v.is_empty() {
        return false;
    }
    if matches!(
        v.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on" | "y" | "enabled"
    ) {
        return true;
    }
    if matches!(
        v.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off" | "n" | "disabled"
    ) {
        return false;
    }
    // FAIL CLOSED, LOUDLY. A security toggle whose misspelling is
    // indistinguishable from being unset is a footgun for exactly the
    // deployment it exists to protect: `ADJ_SENSITIVE_INPUT=pls` would have
    // silently emitted chart text. Anyone who set the variable at all meant
    // something by it, so an unrecognized value is treated as SENSITIVE and
    // the operator is told.
    eprintln!(
        "warning: ADJ_SENSITIVE_INPUT={raw:?} is not a recognized boolean; \
         treating this run as SENSITIVE and redacting echoed payloads."
    );
    true
}

/// Render a query string for output, honouring the sensitive channel.
///
/// # Why this exists as a helper rather than a call at each site
///
/// The first draft redacted the abstention object's fields and left the
/// surrounding `"query"` / `"queries"` echoes alone. Those echoes contain the
/// SAME values — for a recall abstention the goal *is* the query, and a lookup
/// query string spells out the table and the key — so the redacted secret was
/// reprinted verbatim two lines above an object claiming to have redacted it.
/// That is worse than no redaction: it advertises a protection that is not
/// there.
///
/// Routing every query echo through one function is the fix that stays fixed —
/// a new renderer cannot forget a check it never had to make.
pub fn query_echo(q: &str) -> String {
    if sensitive_input() {
        return "[redacted]".to_string();
    }
    esc(q)
}

/// Prepare one echoed payload: redact it on a sensitive channel, otherwise cap
/// its length (marking the truncation, so a reader never mistakes a cut string
/// for the whole value) and JSON-escape it.
pub fn payload(v: &str) -> String {
    if sensitive_input() {
        return "[redacted]".to_string();
    }
    if v.chars().count() > ABSTENTION_FIELD_CAP {
        let head: String = v.chars().take(ABSTENTION_FIELD_CAP).collect();
        return esc(&format!("{head}…(truncated)"));
    }
    esc(v)
}

pub struct FsProvider {
    /// The sandbox root: canonicalized directory of the top-level program. No
    /// import may resolve outside this subtree.
    pub root: PathBuf,
}

impl FsProvider {
    /// Canonicalize `p` and confirm it lies within the sandbox `root`.
    fn checked_canonical(&self, p: &Path) -> Result<String, String> {
        let abs = fs::canonicalize(p).map_err(|e| format!("{}: {e}", p.display()))?;
        if !abs.starts_with(&self.root) {
            return Err(format!(
                "{} escapes the import root {}",
                abs.display(),
                self.root.display()
            ));
        }
        Ok(abs.to_string_lossy().into_owned())
    }
}

impl ImportProvider for FsProvider {
    fn resolve(&self, importer: &str, literal: &str) -> Result<String, String> {
        if Path::new(literal).is_absolute() {
            return Err(format!("import path must be relative, got {literal:?}"));
        }
        // Reject NUL and other obviously hostile bytes before touching the FS.
        if literal.contains('\0') {
            return Err("import path contains a NUL byte".to_string());
        }
        let importer_dir = Path::new(importer)
            .parent()
            .ok_or_else(|| format!("importer {importer:?} has no parent directory"))?;
        self.checked_canonical(&importer_dir.join(literal))
    }

    fn load(&self, canonical: &str) -> Result<String, String> {
        // `canonical` came from `resolve`/the root, already inside `root`; read
        // it. (Re-checking would re-canonicalize an already-canonical path.)
        fs::read_to_string(canonical).map_err(|e| format!("{canonical}: {e}"))
    }
}

