//! # Integration Tests for env
//!
//! These tests verify environment building, formatting, assignment
//! parsing, and command execution.

use unix_tools::env_tool::*;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Tests: Assignment parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod parsing {
    use super::*;

    #[test]
    fn simple_assignment() {
        let result = parse_assignment("FOO=bar");
        assert_eq!(result, Some(("FOO".into(), "bar".into())));
    }

    #[test]
    fn assignment_with_equals_in_value() {
        let result = parse_assignment("X=a=b=c");
        assert_eq!(result, Some(("X".into(), "a=b=c".into())));
    }

    #[test]
    fn assignment_empty_value() {
        let result = parse_assignment("EMPTY=");
        assert_eq!(result, Some(("EMPTY".into(), "".into())));
    }

    #[test]
    fn no_equals_sign() {
        let result = parse_assignment("JUST_A_NAME");
        assert_eq!(result, None);
    }

    #[test]
    fn empty_key() {
        let result = parse_assignment("=value");
        assert_eq!(result, None);
    }

    #[test]
    fn path_like_value() {
        let result = parse_assignment("PATH=/usr/bin:/bin:/sbin");
        assert_eq!(result, Some(("PATH".into(), "/usr/bin:/bin:/sbin".into())));
    }
}

// ---------------------------------------------------------------------------
// Tests: Environment building
// ---------------------------------------------------------------------------

#[cfg(test)]
mod building {
    use super::*;

    #[test]
    fn ignore_env_starts_empty() {
        let opts = EnvOptions {
            ignore_env: true,
            ..Default::default()
        };
        let env = build_environment(&opts);
        assert!(env.is_empty());
    }

    #[test]
    fn ignore_env_with_set_vars() {
        let opts = EnvOptions {
            ignore_env: true,
            set_vars: vec![("FOO".into(), "bar".into())],
            ..Default::default()
        };
        let env = build_environment(&opts);
        assert_eq!(env.len(), 1);
        assert_eq!(env["FOO"], "bar");
    }

    #[test]
    fn unset_removes_variable() {
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
    fn set_overrides_existing() {
        let opts = EnvOptions {
            ignore_env: true,
            set_vars: vec![
                ("X".into(), "old".into()),
                ("X".into(), "new".into()),
            ],
            ..Default::default()
        };
        let env = build_environment(&opts);
        assert_eq!(env["X"], "new");
    }

    #[test]
    fn default_inherits_current_env() {
        let opts = EnvOptions::default();
        let env = build_environment(&opts);
        assert!(!env.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Tests: Formatting
// ---------------------------------------------------------------------------

#[cfg(test)]
mod formatting {
    use super::*;

    #[test]
    fn format_with_newlines() {
        let mut env = BTreeMap::new();
        env.insert("A".into(), "1".into());
        env.insert("B".into(), "2".into());
        let output = format_environment(&env, false);
        assert_eq!(output, "A=1\nB=2\n");
    }

    #[test]
    fn format_with_null_terminator() {
        let mut env = BTreeMap::new();
        env.insert("X".into(), "y".into());
        let output = format_environment(&env, true);
        assert_eq!(output, "X=y\0");
    }

    #[test]
    fn format_empty() {
        let env = BTreeMap::new();
        let output = format_environment(&env, false);
        assert!(output.is_empty());
    }

    #[test]
    fn format_sorted_output() {
        let mut env = BTreeMap::new();
        env.insert("ZZZ".into(), "last".into());
        env.insert("AAA".into(), "first".into());
        env.insert("MMM".into(), "middle".into());
        let output = format_environment(&env, false);
        let lines: Vec<&str> = output.trim().split('\n').collect();
        assert_eq!(lines[0], "AAA=first");
        assert_eq!(lines[1], "MMM=middle");
        assert_eq!(lines[2], "ZZZ=last");
    }
}

// ---------------------------------------------------------------------------
// Tests: Command execution
// ---------------------------------------------------------------------------

#[cfg(test)]
mod execution {
    use super::*;

    #[test]
    fn run_echo() {
        let mut env = BTreeMap::new();
        env.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
        let cmd = vec!["echo".into(), "hello".into()];
        let result = run_command(&cmd, &env, &None);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_nonexistent_fails() {
        let env = BTreeMap::new();
        let cmd = vec!["not_a_command_xyz".into()];
        let result = run_command(&cmd, &env, &None);
        assert!(result.is_err());
    }

    #[test]
    fn run_empty_command_fails() {
        let env = BTreeMap::new();
        let cmd: Vec<String> = vec![];
        let result = run_command(&cmd, &env, &None);
        assert!(result.is_err());
    }

    #[test]
    fn run_with_custom_env() {
        let mut env = BTreeMap::new();
        env.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
        env.insert("MY_CUSTOM_VAR".into(), "test_value".into());
        let cmd = vec!["echo".into(), "ok".into()];
        let result = run_command(&cmd, &env, &None);
        assert_eq!(result.unwrap(), 0);
    }
}
