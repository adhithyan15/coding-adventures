//! # env — Print Environment or Run Command in Modified Environment
//!
//! This module implements the business logic for the `env` command.
//! `env` can do two things:
//!
//! 1. **Print** the current environment variables (like `printenv`)
//! 2. **Run a command** with a modified environment
//!
//! ## How It Works
//!
//! ```text
//!     env                          Print all environment variables
//!     env VAR=value command        Set VAR, then run command
//!     env -i command               Start with empty environment
//!     env -u VAR command           Remove VAR from environment
//!     env -C /path command         Change directory, then run command
//! ```
//!
//! ## Environment Variables
//!
//! Every Unix process has an "environment" — a set of key=value pairs
//! that configure its behavior. Common examples:
//!
//! ```text
//!     Variable    Typical Value          Purpose
//!     ──────────  ─────────────────────  ──────────────────────
//!     PATH        /usr/bin:/bin           Where to find programs
//!     HOME        /home/user             User's home directory
//!     SHELL       /bin/bash              User's default shell
//!     LANG        en_US.UTF-8            Locale settings
//! ```
//!
//! ## Flags
//!
//! ```text
//!     Flag            Field          Effect
//!     ──────────────  ───────────    ──────────────────────────────────
//!     -i, --ignore    ignore_env     Start with a completely empty env
//!     -u, --unset     unset_vars     Remove specific variables
//!     -0, --null      null_terminator Terminate output lines with NUL
//!     -C, --chdir     chdir          Change to directory before running
//! ```

use std::collections::BTreeMap;
use std::process::Command;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling how `env` behaves.
#[derive(Debug, Clone, Default)]
pub struct EnvOptions {
    /// Start with an empty environment (-i).
    pub ignore_env: bool,
    /// Variables to remove from the environment (-u).
    pub unset_vars: Vec<String>,
    /// Use NUL instead of newline as output terminator (-0).
    pub null_terminator: bool,
    /// Change to this directory before running the command (-C).
    pub chdir: Option<String>,
    /// Variables to set (NAME=VALUE pairs).
    pub set_vars: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Environment Building
// ---------------------------------------------------------------------------

/// Build the environment map from current env + options.
///
/// The process is layered, like an onion:
///
/// ```text
///     Layer 1: Start with current environment (or empty if -i)
///     Layer 2: Remove variables specified by -u
///     Layer 3: Add/override variables from NAME=VALUE pairs
/// ```
///
/// Using a BTreeMap gives us sorted output (matching GNU env behavior).
pub fn build_environment(opts: &EnvOptions) -> BTreeMap<String, String> {
    // --- Layer 1: Base environment ---
    let mut env: BTreeMap<String, String> = if opts.ignore_env {
        BTreeMap::new()
    } else {
        std::env::vars().collect()
    };

    // --- Layer 2: Set variables ---
    // Variables from the command line (NAME=VALUE) are added first.
    for (key, value) in &opts.set_vars {
        env.insert(key.clone(), value.clone());
    }

    // --- Layer 3: Unset variables ---
    // -u flags take priority: if you both set and unset a variable,
    // the unset wins. This matches GNU env behavior.
    for var in &opts.unset_vars {
        env.remove(var);
    }

    env
}

/// Format environment variables for output.
///
/// Each variable is printed as `KEY=VALUE` followed by either a
/// newline or a NUL byte (if -0 is specified).
///
/// ```text
///     Normal:     HOME=/home/user\nPATH=/usr/bin\n
///     With -0:    HOME=/home/user\0PATH=/usr/bin\0
/// ```
pub fn format_environment(env: &BTreeMap<String, String>, null_terminator: bool) -> String {
    let terminator = if null_terminator { '\0' } else { '\n' };
    let mut output = String::new();

    for (key, value) in env {
        output.push_str(key);
        output.push('=');
        output.push_str(value);
        output.push(terminator);
    }

    output
}

/// Parse a `NAME=VALUE` string into a (name, value) pair.
///
/// ```text
///     "FOO=bar"      → ("FOO", "bar")
///     "PATH=/a:/b"   → ("PATH", "/a:/b")
///     "EMPTY="       → ("EMPTY", "")
/// ```
///
/// Returns None if the string doesn't contain '='.
pub fn parse_assignment(s: &str) -> Option<(String, String)> {
    let pos = s.find('=')?;
    let key = s[..pos].to_string();
    let value = s[pos + 1..].to_string();

    if key.is_empty() {
        return None;
    }

    Some((key, value))
}

// ---------------------------------------------------------------------------
// Command Execution
// ---------------------------------------------------------------------------

/// Run a command with the given environment.
///
/// This replaces the child process's environment entirely with the
/// provided map. If `chdir` is set, we change the child's working
/// directory before execution.
///
/// ```text
///     env -i FOO=bar /usr/bin/printenv FOO
///     → Sets up empty env with only FOO=bar
///     → Spawns /usr/bin/printenv with arg "FOO"
///     → Output: "bar"
/// ```
pub fn run_command(
    command: &[String],
    env: &BTreeMap<String, String>,
    chdir: &Option<String>,
) -> Result<i32, String> {
    if command.is_empty() {
        return Err("env: no command specified".to_string());
    }

    let program = &command[0];
    let args = &command[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    // --- Set the environment ---
    // We clear the inherited environment and set only what we computed.
    cmd.env_clear();
    for (key, value) in env {
        cmd.env(key, value);
    }

    // --- Change directory if requested ---
    if let Some(dir) = chdir {
        cmd.current_dir(dir);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("env: '{}': {}", program, e))?;

    Ok(status.code().unwrap_or(125))
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_assignment tests ---

    #[test]
    fn parse_simple_assignment() {
        let result = parse_assignment("FOO=bar");
        assert_eq!(result, Some(("FOO".into(), "bar".into())));
    }

    #[test]
    fn parse_assignment_with_equals_in_value() {
        let result = parse_assignment("PATH=/a:/b=c");
        assert_eq!(result, Some(("PATH".into(), "/a:/b=c".into())));
    }

    #[test]
    fn parse_assignment_empty_value() {
        let result = parse_assignment("EMPTY=");
        assert_eq!(result, Some(("EMPTY".into(), "".into())));
    }

    #[test]
    fn parse_assignment_no_equals() {
        let result = parse_assignment("NOEQUALS");
        assert_eq!(result, None);
    }

    #[test]
    fn parse_assignment_empty_key() {
        let result = parse_assignment("=value");
        assert_eq!(result, None);
    }

    // --- build_environment tests ---

    #[test]
    fn build_env_ignore_clears_everything() {
        let opts = EnvOptions {
            ignore_env: true,
            set_vars: vec![("FOO".into(), "bar".into())],
            ..Default::default()
        };
        let env = build_environment(&opts);
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn build_env_unset_removes_variable() {
        let opts = EnvOptions {
            ignore_env: true,
            set_vars: vec![
                ("KEEP".into(), "yes".into()),
                ("DROP".into(), "no".into()),
            ],
            unset_vars: vec!["DROP".into()],
            ..Default::default()
        };
        let env = build_environment(&opts);
        assert!(env.contains_key("KEEP"));
        assert!(!env.contains_key("DROP"));
    }

    #[test]
    fn build_env_set_overrides() {
        let opts = EnvOptions {
            ignore_env: true,
            set_vars: vec![
                ("X".into(), "first".into()),
                ("X".into(), "second".into()),
            ],
            ..Default::default()
        };
        let env = build_environment(&opts);
        // Last set wins
        assert_eq!(env.get("X"), Some(&"second".to_string()));
    }

    // --- format_environment tests ---

    #[test]
    fn format_with_newline() {
        let mut env = BTreeMap::new();
        env.insert("A".into(), "1".into());
        env.insert("B".into(), "2".into());
        let output = format_environment(&env, false);
        assert_eq!(output, "A=1\nB=2\n");
    }

    #[test]
    fn format_with_null() {
        let mut env = BTreeMap::new();
        env.insert("X".into(), "y".into());
        let output = format_environment(&env, true);
        assert_eq!(output, "X=y\0");
    }

    #[test]
    fn format_empty_env() {
        let env = BTreeMap::new();
        let output = format_environment(&env, false);
        assert!(output.is_empty());
    }

    // --- run_command tests ---

    #[test]
    fn run_echo_command() {
        let mut env = BTreeMap::new();
        env.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
        let cmd = vec!["echo".into(), "hello".into()];
        let result = run_command(&cmd, &env, &None);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_nonexistent_command() {
        let env = BTreeMap::new();
        let cmd = vec!["not_a_real_command_xyz123".into()];
        let result = run_command(&cmd, &env, &None);
        assert!(result.is_err());
    }

    #[test]
    fn run_no_command() {
        let env = BTreeMap::new();
        let cmd: Vec<String> = vec![];
        let result = run_command(&cmd, &env, &None);
        assert!(result.is_err());
    }

    #[test]
    fn build_env_default_inherits_current() {
        let opts = EnvOptions::default();
        let env = build_environment(&opts);
        // Should have at least some variables from the current process
        assert!(!env.is_empty());
    }

    #[test]
    fn format_environment_sorted() {
        let mut env = BTreeMap::new();
        env.insert("ZZZ".into(), "last".into());
        env.insert("AAA".into(), "first".into());
        let output = format_environment(&env, false);
        // BTreeMap is sorted, so AAA should come first
        assert!(output.starts_with("AAA=first"));
    }
}
