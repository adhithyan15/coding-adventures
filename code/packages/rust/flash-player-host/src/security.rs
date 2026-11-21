//! Mandatory CWE-94 / CWE-78 / CWE-117 host security defenses (`SWF00` §7.5).
//!
//! Provides:
//! - Permanent neutralization of `fscommand("exec", ...)` as a no-op with diagnostic audit recording.
//! - Zero-allocation URI scheme sanitization for `getURL` / `navigateToURL`, permitting only
//!   `http:`, `https:`, or relative URIs while blocking `javascript:`, `file:`, `data:`,
//!   `vbscript:`, and all other schemes (case-insensitive, whitespace/control-trimmed).
//! - Bounded diagnostic [`SecurityAuditEvent`] log with control-character sanitization (CWE-117).

#![forbid(unsafe_code)]

use crate::error::{PlayerHostError, MAX_AUDIT_EVENTS, MAX_URI_LEN};

/// Classification of security policy audit events emitted by the host shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityAuditKind {
    /// `fscommand("exec", ...)` (or equivalent dangerous verb) was blocked as a no-op.
    FsCommandExecBlocked,
    /// A benign `fscommand` UI hint was intercepted as a sandboxed no-op.
    FsCommandHandled,
    /// A `getURL` / `navigateToURL` request was blocked by the URI scheme filter.
    UriBlocked,
    /// A `getURL` / `navigateToURL` request passed the URI scheme filter.
    UriAllowed,
    /// Frame script execution hit the VM instruction limit (`max_steps <= 2_000_000`).
    ScriptTimeout,
    /// Frame script attempted prototype pollution and was blocked by `avm-bytecode-lifter`.
    PrototypePollutionBlocked,
}

/// Diagnostic audit record captured inside the sandboxed host shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAuditEvent {
    /// 1-based timeline frame index when the event occurred.
    pub frame: u32,
    /// Audit classification.
    pub kind: SecurityAuditKind,
    /// Sanitized single-line diagnostic description (ASCII control chars removed).
    pub detail: String,
}

impl SecurityAuditEvent {
    /// Returns the 1-based frame number of this audit event.
    #[inline]
    pub fn audit_frame(&self) -> u32 {
        self.frame
    }

    /// Returns the audit event classification.
    #[inline]
    pub fn audit_kind(&self) -> SecurityAuditKind {
        self.kind
    }

    /// Returns the sanitized diagnostic detail string.
    #[inline]
    pub fn audit_detail(&self) -> &str {
        &self.detail
    }
}

/// Disposition returned after evaluating an `fscommand(command, args)` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsCommandOutcome {
    /// Dangerous command (`exec`, etc.) permanently disabled as a no-op (CWE-78).
    BlockedNoOp,
    /// Benign UI hint safely neutralized as a sandboxed no-op (zero ambient authority).
    HandledNoOp,
}

/// Bounded diagnostic audit event log (CWE-400 / CWE-117 safe).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecurityAuditLog {
    events: Vec<SecurityAuditEvent>,
}

impl SecurityAuditLog {
    /// Creates an empty audit log.
    pub fn log_empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Appends a sanitized audit event, dropping oldest entries if `MAX_AUDIT_EVENTS` is reached.
    pub fn log_record(&mut self, frame: u32, kind: SecurityAuditKind, raw_detail: &str) {
        if self.events.len() >= MAX_AUDIT_EVENTS {
            self.events.remove(0);
        }
        self.events.push(SecurityAuditEvent {
            frame,
            kind,
            detail: sanitize_audit_detail(raw_detail),
        });
    }

    /// Borrows all recorded audit events.
    pub fn log_events(&self) -> &[SecurityAuditEvent] {
        &self.events
    }

    /// Clears all recorded audit events.
    pub fn log_clear(&mut self) {
        self.events.clear();
    }
}

/// Strips ASCII control characters and caps length to prevent CWE-117 log injection.
fn sanitize_audit_detail(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(256));
    for ch in raw.chars().take(256) {
        if ch.is_ascii_control() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Trims leading and trailing Unicode whitespace and ASCII control characters (`<= 0x20`, `0x7F`).
#[inline]
pub fn trim_whitespace_and_control(input: &str) -> &str {
    input.trim_matches(|c: char| c.is_ascii_control() || c.is_whitespace())
}

/// Returns `true` if `command` resolves to the disabled `exec` (or `save`) verb.
///
/// Zero heap allocation (`VER07`), case-insensitive, whitespace/control-trimmed.
#[inline]
pub fn is_fscommand_exec(command: &str) -> bool {
    let trimmed = trim_whitespace_and_control(command);
    trimmed.eq_ignore_ascii_case("exec") || trimmed.eq_ignore_ascii_case("save")
}

/// Evaluates an `fscommand(command, args)` request with zero ambient authority and zero heap allocation.
///
/// `fscommand("exec", ...)` is permanently disabled (`FsCommandOutcome::BlockedNoOp`);
/// no process is ever spawned.
#[inline]
pub fn evaluate_fscommand(command: &str, _args: &str) -> FsCommandOutcome {
    if is_fscommand_exec(command) {
        FsCommandOutcome::BlockedNoOp
    } else {
        FsCommandOutcome::HandledNoOp
    }
}

/// Validates and trims a `getURL` / `navigateToURL` URI with zero heap allocations (`VER07`).
///
/// ## Policy (CWE-94 / CWE-79 / CWE-20)
///
/// 1. Leading and trailing whitespace and ASCII control characters (`0x00..=0x1F`, `0x7F`) are trimmed.
/// 2. Empty URIs and URIs longer than `MAX_URI_LEN` (`2048`) return [`PlayerHostError::InvalidUri`].
/// 3. Any embedded ASCII control character anywhere inside the trimmed URI (such as `\t`, `\n`, `\r`, `\0`,
///    historically used to smuggle `java\tscript:`) is rejected with [`PlayerHostError::InvalidUri`].
/// 4. Windows UNC / backslash-prefixed paths (`\\` or `\`) are rejected with [`PlayerHostError::InvalidUri`].
/// 5. If a URI scheme delimiter `:` appears before any `/`, `?`, or `#`:
///    - The scheme must obey RFC 3986 syntax (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`).
///    - **Only** `http` and `https` (case-insensitive) are permitted.
///    - All other schemes (`javascript:`, `file:`, `data:`, `vbscript:`, `about:`, `ftp:`, etc.)
///      return [`PlayerHostError::BlockedUriScheme`].
/// 6. URIs without a scheme prefix before `/`, `?`, or `#` are treated as relative references and permitted
///    (provided they do not begin with `//` protocol-relative authority spoofing or backslashes).
pub fn sanitize_navigation_uri(raw_uri: &str) -> Result<&str, PlayerHostError> {
    let trimmed = trim_whitespace_and_control(raw_uri);
    if trimmed.is_empty() || trimmed.len() > MAX_URI_LEN {
        return Err(PlayerHostError::InvalidUri);
    }

    if trimmed.starts_with('\\') || trimmed.starts_with("//") || trimmed.starts_with("/\\") {
        return Err(PlayerHostError::BlockedUriScheme);
    }

    // Reject embedded ASCII control characters (prevents `java\tscript:` / `java\nscript:` smuggling)
    // and embedded backslashes (WHATWG authority spoofing).
    for b in trimmed.bytes() {
        if b < 0x20 || b == 0x7F || b == b'\\' {
            return Err(PlayerHostError::InvalidUri);
        }
    }

    // Scan up to the first RFC 3986 delimiter among `:`, `/`, `?`, `#`.
    let mut colon_pos: Option<usize> = None;
    for (idx, b) in trimmed.bytes().enumerate() {
        match b {
            b':' => {
                colon_pos = Some(idx);
                break;
            }
            b'/' | b'?' | b'#' => {
                // Path/query/fragment delimiter reached before any colon -> relative URI.
                break;
            }
            _ => {}
        }
    }

    if let Some(idx) = colon_pos {
        if idx == 0 {
            return Err(PlayerHostError::BlockedUriScheme);
        }
        let scheme = &trimmed[..idx];
        let scheme_bytes = scheme.as_bytes();
        if !scheme_bytes[0].is_ascii_alphabetic() {
            return Err(PlayerHostError::InvalidUri);
        }
        for &sb in &scheme_bytes[1..] {
            if !(sb.is_ascii_alphanumeric() || sb == b'+' || sb == b'-' || sb == b'.') {
                return Err(PlayerHostError::InvalidUri);
            }
        }
        if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
            let rest = &trimmed[idx + 1..];
            if !rest.starts_with("//") || rest.len() <= 2 {
                return Err(PlayerHostError::InvalidUri);
            }
            Ok(trimmed)
        } else {
            Err(PlayerHostError::BlockedUriScheme)
        }
    } else {
        // Relative URI (no scheme before `/`, `?`, `#`).
        Ok(trimmed)
    }
}
