//! # chmod — Change File Permissions
//!
//! This module implements the business logic for the `chmod` command.
//! `chmod` changes the access permissions of files and directories.
//!
//! ## Unix Permission Model
//!
//! Every file has three sets of permissions, one for each class:
//!
//! ```text
//!     Class    Meaning          Typical Abbreviation
//!     ───────  ───────────────  ────────────────────
//!     User     The file owner   u
//!     Group    The file's group g
//!     Other    Everyone else    o
//!     All      All three        a
//! ```
//!
//! Each class has three permission bits:
//!
//! ```text
//!     Permission  Bit  Octal  Meaning
//!     ──────────  ───  ─────  ──────────────────────
//!     Read        r    4      Can read file contents
//!     Write       w    2      Can modify file contents
//!     Execute     x    1      Can execute as program
//! ```
//!
//! ## Octal Mode
//!
//! Permissions can be specified as a 3-digit octal number:
//!
//! ```text
//!     chmod 755 file
//!           │││
//!           ││└── Other: r-x (5 = 4+1)
//!           │└─── Group: r-x (5 = 4+1)
//!           └──── User:  rwx (7 = 4+2+1)
//! ```
//!
//! ## Symbolic Mode
//!
//! Permissions can also be specified symbolically:
//!
//! ```text
//!     chmod u+x file       Add execute for user
//!     chmod go-w file      Remove write for group and other
//!     chmod a=rw file      Set read+write for all, remove execute
//!     chmod u+x,g-w file   Multiple changes separated by commas
//! ```
//!
//! ## Flags
//!
//! ```text
//!     Flag              Field       Effect
//!     ────────────────  ──────────  ──────────────────────────────
//!     -R, --recursive   recursive   Change permissions recursively
//!     -v, --verbose     verbose     Print each file processed
//!     -c, --changes     changes     Like verbose but only for changes
//!     -f, --silent      silent      Suppress most error messages
//! ```

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling how `chmod` behaves.
#[derive(Debug, Clone, Default)]
pub struct ChmodOptions {
    /// Change permissions recursively for directories (-R).
    pub recursive: bool,
    /// Print a diagnostic for every file processed (-v).
    pub verbose: bool,
    /// Like verbose but only report actual changes (-c).
    pub changes: bool,
    /// Suppress most error messages (-f).
    pub silent: bool,
}

// ---------------------------------------------------------------------------
// Permission Parsing
// ---------------------------------------------------------------------------

/// Represents a parsed permission change.
///
/// A symbolic mode string like `u+rwx,go-w` produces multiple
/// `PermissionChange` values, one per comma-separated clause.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionChange {
    /// Which classes to modify: 'u', 'g', 'o', or 'a'.
    pub who: Vec<char>,
    /// The operation: '+' (add), '-' (remove), '=' (set exactly).
    pub op: char,
    /// Which permission bits: 'r', 'w', 'x'.
    pub perms: Vec<char>,
}

/// Parse an octal mode string (e.g., "755") into a u32 permission value.
///
/// ```text
///     "755" → 0o755 → 493 (decimal)
///     "644" → 0o644 → 420 (decimal)
///     "777" → 0o777 → 511 (decimal)
/// ```
///
/// Returns None if the string is not a valid octal number.
pub fn parse_octal_mode(s: &str) -> Option<u32> {
    // Must be all digits 0-7
    if s.is_empty() || !s.chars().all(|c| c >= '0' && c <= '7') {
        return None;
    }
    u32::from_str_radix(s, 8).ok()
}

/// Parse a symbolic mode string (e.g., "u+x,go-w") into permission changes.
///
/// ## Grammar
///
/// ```text
///     mode_string  = clause (',' clause)*
///     clause       = who* op perm*
///     who          = 'u' | 'g' | 'o' | 'a'
///     op           = '+' | '-' | '='
///     perm         = 'r' | 'w' | 'x'
/// ```
///
/// If no `who` is specified, it defaults to 'a' (all).
pub fn parse_symbolic_mode(s: &str) -> Result<Vec<PermissionChange>, String> {
    let mut changes = Vec::new();

    for clause in s.split(',') {
        if clause.is_empty() {
            continue;
        }

        let chars: Vec<char> = clause.chars().collect();
        let mut i = 0;

        // --- Parse 'who' part ---
        let mut who = Vec::new();
        while i < chars.len() && "ugoa".contains(chars[i]) {
            who.push(chars[i]);
            i += 1;
        }
        if who.is_empty() {
            who = vec!['a']; // Default to 'all'
        }

        // --- Parse operator ---
        if i >= chars.len() || !"+-=".contains(chars[i]) {
            return Err(format!("chmod: invalid mode: '{}'", s));
        }
        let op = chars[i];
        i += 1;

        // --- Parse permissions ---
        let mut perms = Vec::new();
        while i < chars.len() && "rwxXst".contains(chars[i]) {
            perms.push(chars[i]);
            i += 1;
        }

        changes.push(PermissionChange { who, op, perms });
    }

    if changes.is_empty() {
        return Err(format!("chmod: invalid mode: '{}'", s));
    }

    Ok(changes)
}

/// Apply a permission change to an existing mode.
///
/// This is the core logic that translates symbolic changes into
/// actual permission bit manipulation.
///
/// ```text
///     Current: rwxr-xr-x (0o755)
///     Change:  go-x
///     Result:  rwxr--r-- (0o744)
///
///     How: For each 'who' in [g, o]:
///          For each 'perm' in [x]:
///              Clear the corresponding bit
/// ```
pub fn apply_symbolic_change(current_mode: u32, change: &PermissionChange) -> u32 {
    let mut mode = current_mode;

    // --- Build the permission bits to modify ---
    let mut bits = 0u32;
    for p in &change.perms {
        match p {
            'r' => bits |= 4,
            'w' => bits |= 2,
            'x' => bits |= 1,
            _ => {}
        }
    }

    // --- Apply to each 'who' class ---
    for w in &change.who {
        let shifts: Vec<u32> = match w {
            'u' => vec![6],
            'g' => vec![3],
            'o' => vec![0],
            'a' => vec![6, 3, 0],
            _ => vec![],
        };

        for shift in shifts {
            let shifted_bits = bits << shift;
            match change.op {
                '+' => mode |= shifted_bits,
                '-' => mode &= !shifted_bits,
                '=' => {
                    // Clear all bits for this class, then set the new ones
                    let mask = 7u32 << shift;
                    mode = (mode & !mask) | shifted_bits;
                }
                _ => {}
            }
        }
    }

    mode
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Change the permissions of a file.
///
/// The `mode_str` can be either octal ("755") or symbolic ("u+x").
///
/// # Returns
///
/// A vector of messages describing what was done (for -v/-c flags).
#[cfg(unix)]
pub fn chmod_file(
    path: &str,
    mode_str: &str,
    opts: &ChmodOptions,
) -> Result<Vec<String>, String> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        if opts.silent {
            return Ok(vec![]);
        }
        return Err(format!(
            "chmod: cannot access '{}': No such file or directory",
            path
        ));
    }

    let mut messages = Vec::new();

    // --- Get current permissions ---
    let metadata = fs::metadata(file_path)
        .map_err(|e| format!("chmod: '{}': {}", path, e))?;
    let current_mode = metadata.permissions().mode() & 0o7777;

    // --- Calculate new mode ---
    let new_mode = if let Some(octal) = parse_octal_mode(mode_str) {
        octal
    } else {
        let changes = parse_symbolic_mode(mode_str)?;
        let mut mode = current_mode;
        for change in &changes {
            mode = apply_symbolic_change(mode, change);
        }
        mode
    };

    // --- Apply new permissions ---
    let new_perms = fs::Permissions::from_mode(new_mode);
    fs::set_permissions(file_path, new_perms)
        .map_err(|e| format!("chmod: '{}': {}", path, e))?;

    // --- Generate messages ---
    if opts.verbose || (opts.changes && current_mode != new_mode) {
        messages.push(format!(
            "mode of '{}' changed from {:04o} to {:04o}",
            path, current_mode, new_mode
        ));
    }

    // --- Recurse into directories ---
    if opts.recursive && file_path.is_dir() {
        let entries = fs::read_dir(file_path)
            .map_err(|e| format!("chmod: '{}': {}", path, e))?;

        for entry_result in entries {
            let entry = entry_result
                .map_err(|e| format!("chmod: error reading '{}': {}", path, e))?;
            let child_path = entry.path();
            let child_msgs = chmod_file(
                &child_path.to_string_lossy(),
                mode_str,
                opts,
            )?;
            messages.extend(child_msgs);
        }
    }

    Ok(messages)
}

/// Non-Unix stub so the code compiles on all platforms.
#[cfg(not(unix))]
pub fn chmod_file(
    path: &str,
    _mode_str: &str,
    _opts: &ChmodOptions,
) -> Result<Vec<String>, String> {
    Err(format!("chmod: not supported on this platform ({})", path))
}

/// Format a mode as a human-readable permission string.
///
/// ```text
///     0o755 → "rwxr-xr-x"
///     0o644 → "rw-r--r--"
///     0o700 → "rwx------"
/// ```
pub fn format_mode(mode: u32) -> String {
    let mut s = String::with_capacity(9);

    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }

    s
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_octal_mode tests ---

    #[test]
    fn parse_octal_755() {
        assert_eq!(parse_octal_mode("755"), Some(0o755));
    }

    #[test]
    fn parse_octal_644() {
        assert_eq!(parse_octal_mode("644"), Some(0o644));
    }

    #[test]
    fn parse_octal_000() {
        assert_eq!(parse_octal_mode("000"), Some(0));
    }

    #[test]
    fn parse_octal_777() {
        assert_eq!(parse_octal_mode("777"), Some(0o777));
    }

    #[test]
    fn parse_octal_invalid() {
        assert_eq!(parse_octal_mode("899"), None);
        assert_eq!(parse_octal_mode("abc"), None);
        assert_eq!(parse_octal_mode(""), None);
    }

    // --- parse_symbolic_mode tests ---

    #[test]
    fn parse_symbolic_u_plus_x() {
        let changes = parse_symbolic_mode("u+x").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].who, vec!['u']);
        assert_eq!(changes[0].op, '+');
        assert_eq!(changes[0].perms, vec!['x']);
    }

    #[test]
    fn parse_symbolic_go_minus_w() {
        let changes = parse_symbolic_mode("go-w").unwrap();
        assert_eq!(changes[0].who, vec!['g', 'o']);
        assert_eq!(changes[0].op, '-');
        assert_eq!(changes[0].perms, vec!['w']);
    }

    #[test]
    fn parse_symbolic_multiple_clauses() {
        let changes = parse_symbolic_mode("u+x,g-w,o=r").unwrap();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn parse_symbolic_default_who_is_all() {
        let changes = parse_symbolic_mode("+x").unwrap();
        assert_eq!(changes[0].who, vec!['a']);
    }

    #[test]
    fn parse_symbolic_a_equals_rwx() {
        let changes = parse_symbolic_mode("a=rwx").unwrap();
        assert_eq!(changes[0].who, vec!['a']);
        assert_eq!(changes[0].op, '=');
        assert_eq!(changes[0].perms, vec!['r', 'w', 'x']);
    }

    #[test]
    fn parse_symbolic_invalid() {
        assert!(parse_symbolic_mode("").is_err());
        assert!(parse_symbolic_mode("xyz").is_err());
    }

    // --- apply_symbolic_change tests ---

    #[test]
    fn apply_user_plus_execute() {
        let change = PermissionChange {
            who: vec!['u'],
            op: '+',
            perms: vec!['x'],
        };
        // Start with rw-r--r-- (0o644)
        let result = apply_symbolic_change(0o644, &change);
        assert_eq!(result, 0o744);
    }

    #[test]
    fn apply_group_other_minus_write() {
        let change = PermissionChange {
            who: vec!['g', 'o'],
            op: '-',
            perms: vec!['w'],
        };
        // Start with rwxrwxrwx (0o777)
        let result = apply_symbolic_change(0o777, &change);
        assert_eq!(result, 0o755);
    }

    #[test]
    fn apply_all_equals_read() {
        let change = PermissionChange {
            who: vec!['a'],
            op: '=',
            perms: vec!['r'],
        };
        // Start with rwxrwxrwx (0o777)
        let result = apply_symbolic_change(0o777, &change);
        assert_eq!(result, 0o444);
    }

    #[test]
    fn apply_other_plus_rwx() {
        let change = PermissionChange {
            who: vec!['o'],
            op: '+',
            perms: vec!['r', 'w', 'x'],
        };
        // Start with rwx------ (0o700)
        let result = apply_symbolic_change(0o700, &change);
        assert_eq!(result, 0o707);
    }

    #[test]
    fn apply_equals_clears_and_sets() {
        let change = PermissionChange {
            who: vec!['u'],
            op: '=',
            perms: vec!['r'],
        };
        // Start with rwx------ (0o700)
        let result = apply_symbolic_change(0o700, &change);
        // User becomes r-- only
        assert_eq!(result, 0o400);
    }

    // --- format_mode tests ---

    #[test]
    fn format_mode_755() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
    }

    #[test]
    fn format_mode_644() {
        assert_eq!(format_mode(0o644), "rw-r--r--");
    }

    #[test]
    fn format_mode_000() {
        assert_eq!(format_mode(0o000), "---------");
    }

    #[test]
    fn format_mode_777() {
        assert_eq!(format_mode(0o777), "rwxrwxrwx");
    }

    #[test]
    fn format_mode_700() {
        assert_eq!(format_mode(0o700), "rwx------");
    }

    // --- chmod_file tests (unix only) ---

    #[cfg(unix)]
    #[test]
    fn chmod_octal_on_real_file() {
        let dir = std::env::temp_dir().join("chmod_test_octal");
        let _ = fs::remove_file(&dir);
        fs::write(&dir, "test").unwrap();

        let result = chmod_file(
            &dir.to_string_lossy(),
            "644",
            &ChmodOptions::default(),
        );
        assert!(result.is_ok());

        let meta = fs::metadata(&dir).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o644);

        let _ = fs::remove_file(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn chmod_symbolic_on_real_file() {
        let dir = std::env::temp_dir().join("chmod_test_symbolic");
        let _ = fs::remove_file(&dir);
        fs::write(&dir, "test").unwrap();

        // First set to 644, then add execute for user
        let _ = chmod_file(&dir.to_string_lossy(), "644", &ChmodOptions::default());
        let result = chmod_file(&dir.to_string_lossy(), "u+x", &ChmodOptions::default());
        assert!(result.is_ok());

        let meta = fs::metadata(&dir).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o744);

        let _ = fs::remove_file(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn chmod_nonexistent_file() {
        let result = chmod_file(
            "/tmp/chmod_test_nonexistent_xyz",
            "755",
            &ChmodOptions::default(),
        );
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn chmod_verbose_reports_change() {
        let dir = std::env::temp_dir().join("chmod_test_verbose");
        let _ = fs::remove_file(&dir);
        fs::write(&dir, "test").unwrap();

        let opts = ChmodOptions { verbose: true, ..Default::default() };
        let result = chmod_file(&dir.to_string_lossy(), "755", &opts);
        let messages = result.unwrap();
        assert!(!messages.is_empty());
        assert!(messages[0].contains("mode of"));

        let _ = fs::remove_file(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn chmod_silent_suppresses_error() {
        let opts = ChmodOptions { silent: true, ..Default::default() };
        let result = chmod_file("/tmp/chmod_test_nonexistent", "755", &opts);
        assert!(result.is_ok());
    }
}
