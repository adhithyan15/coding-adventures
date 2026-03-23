//! # Integration Tests for chmod
//!
//! These tests verify permission parsing (octal and symbolic),
//! permission application, and file permission changes.

use unix_tools::chmod_tool::*;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("chmod_integ_{}", name))
        .to_string_lossy()
        .into_owned()
}

fn cleanup(path: &str) {
    let p = Path::new(path);
    if p.is_dir() {
        let _ = fs::remove_dir_all(p);
    } else {
        let _ = fs::remove_file(p);
    }
}

// ---------------------------------------------------------------------------
// Tests: Octal parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod octal_parsing {
    use super::*;

    #[test]
    fn parse_755() {
        assert_eq!(parse_octal_mode("755"), Some(0o755));
    }

    #[test]
    fn parse_644() {
        assert_eq!(parse_octal_mode("644"), Some(0o644));
    }

    #[test]
    fn parse_000() {
        assert_eq!(parse_octal_mode("000"), Some(0));
    }

    #[test]
    fn parse_777() {
        assert_eq!(parse_octal_mode("777"), Some(0o777));
    }

    #[test]
    fn invalid_octal_digits() {
        assert_eq!(parse_octal_mode("899"), None);
    }

    #[test]
    fn invalid_alphabetic() {
        assert_eq!(parse_octal_mode("abc"), None);
    }

    #[test]
    fn empty_string() {
        assert_eq!(parse_octal_mode(""), None);
    }
}

// ---------------------------------------------------------------------------
// Tests: Symbolic parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod symbolic_parsing {
    use super::*;

    #[test]
    fn user_plus_execute() {
        let changes = parse_symbolic_mode("u+x").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].who, vec!['u']);
        assert_eq!(changes[0].op, '+');
        assert_eq!(changes[0].perms, vec!['x']);
    }

    #[test]
    fn group_other_minus_write() {
        let changes = parse_symbolic_mode("go-w").unwrap();
        assert_eq!(changes[0].who, vec!['g', 'o']);
    }

    #[test]
    fn all_equals_rwx() {
        let changes = parse_symbolic_mode("a=rwx").unwrap();
        assert_eq!(changes[0].perms, vec!['r', 'w', 'x']);
    }

    #[test]
    fn multiple_clauses() {
        let changes = parse_symbolic_mode("u+x,g-w,o=r").unwrap();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn default_who_is_all() {
        let changes = parse_symbolic_mode("+x").unwrap();
        assert_eq!(changes[0].who, vec!['a']);
    }

    #[test]
    fn invalid_mode() {
        assert!(parse_symbolic_mode("").is_err());
    }
}

// ---------------------------------------------------------------------------
// Tests: Permission application
// ---------------------------------------------------------------------------

#[cfg(test)]
mod application {
    use super::*;

    #[test]
    fn add_user_execute_to_644() {
        let change = PermissionChange {
            who: vec!['u'],
            op: '+',
            perms: vec!['x'],
        };
        assert_eq!(apply_symbolic_change(0o644, &change), 0o744);
    }

    #[test]
    fn remove_group_other_write_from_777() {
        let change = PermissionChange {
            who: vec!['g', 'o'],
            op: '-',
            perms: vec!['w'],
        };
        assert_eq!(apply_symbolic_change(0o777, &change), 0o755);
    }

    #[test]
    fn set_all_to_read_only() {
        let change = PermissionChange {
            who: vec!['a'],
            op: '=',
            perms: vec!['r'],
        };
        assert_eq!(apply_symbolic_change(0o777, &change), 0o444);
    }

    #[test]
    fn add_all_permissions_to_other() {
        let change = PermissionChange {
            who: vec!['o'],
            op: '+',
            perms: vec!['r', 'w', 'x'],
        };
        assert_eq!(apply_symbolic_change(0o700, &change), 0o707);
    }

    #[test]
    fn set_user_read_only_from_rwx() {
        let change = PermissionChange {
            who: vec!['u'],
            op: '=',
            perms: vec!['r'],
        };
        assert_eq!(apply_symbolic_change(0o700, &change), 0o400);
    }

    #[test]
    fn remove_all_execute() {
        let change = PermissionChange {
            who: vec!['a'],
            op: '-',
            perms: vec!['x'],
        };
        assert_eq!(apply_symbolic_change(0o777, &change), 0o666);
    }
}

// ---------------------------------------------------------------------------
// Tests: format_mode
// ---------------------------------------------------------------------------

#[cfg(test)]
mod formatting {
    use super::*;

    #[test]
    fn format_755() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
    }

    #[test]
    fn format_644() {
        assert_eq!(format_mode(0o644), "rw-r--r--");
    }

    #[test]
    fn format_000() {
        assert_eq!(format_mode(0o000), "---------");
    }

    #[test]
    fn format_777() {
        assert_eq!(format_mode(0o777), "rwxrwxrwx");
    }
}

// ---------------------------------------------------------------------------
// Tests: chmod_file (Unix-only filesystem tests)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[cfg(test)]
mod filesystem {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn chmod_octal_sets_permissions() {
        let path = temp_path("octal_set");
        cleanup(&path);
        fs::write(&path, "test").unwrap();

        chmod_file(&path, "755", &ChmodOptions::default()).unwrap();
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o755);

        cleanup(&path);
    }

    #[test]
    fn chmod_symbolic_adds_execute() {
        let path = temp_path("sym_add");
        cleanup(&path);
        fs::write(&path, "test").unwrap();
        chmod_file(&path, "644", &ChmodOptions::default()).unwrap();

        chmod_file(&path, "u+x", &ChmodOptions::default()).unwrap();
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o744);

        cleanup(&path);
    }

    #[test]
    fn chmod_nonexistent_file_fails() {
        let result = chmod_file(
            "/tmp/chmod_integ_nonexistent_xyz",
            "755",
            &ChmodOptions::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn chmod_verbose_produces_messages() {
        let path = temp_path("verbose_msg");
        cleanup(&path);
        fs::write(&path, "test").unwrap();

        let opts = ChmodOptions { verbose: true, ..Default::default() };
        let messages = chmod_file(&path, "755", &opts).unwrap();
        assert!(!messages.is_empty());
        assert!(messages[0].contains("mode of"));

        cleanup(&path);
    }

    #[test]
    fn chmod_recursive_changes_directory() {
        let dir = temp_path("recursive_dir");
        cleanup(&dir);
        fs::create_dir_all(format!("{}/sub", dir)).unwrap();
        fs::write(format!("{}/sub/file.txt", dir), "data").unwrap();

        let opts = ChmodOptions { recursive: true, ..Default::default() };
        let result = chmod_file(&dir, "755", &opts);
        assert!(result.is_ok());

        let meta = fs::metadata(format!("{}/sub/file.txt", dir)).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o755);

        cleanup(&dir);
    }

    #[test]
    fn chmod_silent_on_nonexistent() {
        let opts = ChmodOptions { silent: true, ..Default::default() };
        let result = chmod_file("/tmp/chmod_integ_silent_noexist", "755", &opts);
        assert!(result.is_ok());
    }
}
