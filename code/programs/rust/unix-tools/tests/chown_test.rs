//! # Integration Tests for chown
//!
//! These tests verify ownership spec parsing, UID/GID resolution,
//! and file ownership changes. Many tests require Unix (cfg(unix))
//! since chown is a Unix-specific operation.

use unix_tools::chown_tool::*;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("chown_integ_{}", name))
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
// Tests: Owner spec parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod parsing {
    use super::*;

    #[test]
    fn owner_only() {
        let spec = parse_owner_spec("alice").unwrap();
        assert_eq!(spec.owner, Some("alice".into()));
        assert_eq!(spec.group, None);
    }

    #[test]
    fn owner_and_group() {
        let spec = parse_owner_spec("alice:staff").unwrap();
        assert_eq!(spec.owner, Some("alice".into()));
        assert_eq!(spec.group, Some("staff".into()));
    }

    #[test]
    fn group_only() {
        let spec = parse_owner_spec(":staff").unwrap();
        assert_eq!(spec.owner, None);
        assert_eq!(spec.group, Some("staff".into()));
    }

    #[test]
    fn owner_with_empty_group() {
        let spec = parse_owner_spec("alice:").unwrap();
        assert_eq!(spec.owner, Some("alice".into()));
        assert_eq!(spec.group, Some("".into()));
    }

    #[test]
    fn numeric_owner() {
        let spec = parse_owner_spec("1000").unwrap();
        assert_eq!(spec.owner, Some("1000".into()));
    }

    #[test]
    fn numeric_owner_and_group() {
        let spec = parse_owner_spec("1000:100").unwrap();
        assert_eq!(spec.owner, Some("1000".into()));
        assert_eq!(spec.group, Some("100".into()));
    }

    #[test]
    fn empty_spec_fails() {
        assert!(parse_owner_spec("").is_err());
    }
}

// ---------------------------------------------------------------------------
// Tests: UID/GID resolution
// ---------------------------------------------------------------------------

#[cfg(test)]
mod resolution {
    use super::*;

    #[test]
    fn resolve_numeric_uid() {
        assert_eq!(resolve_uid("0").unwrap(), 0);
    }

    #[test]
    fn resolve_numeric_uid_large() {
        assert_eq!(resolve_uid("65534").unwrap(), 65534);
    }

    #[test]
    fn resolve_invalid_user() {
        assert!(resolve_uid("definitely_not_a_real_user_xyz").is_err());
    }

    #[test]
    fn resolve_numeric_gid() {
        assert_eq!(resolve_gid("0").unwrap(), 0);
    }

    #[test]
    fn resolve_invalid_group() {
        assert!(resolve_gid("definitely_not_a_real_group_xyz").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resolve_root_user() {
        // "root" should exist on all Unix systems
        let uid = resolve_uid("root");
        assert_eq!(uid.unwrap(), 0);
    }
}

// ---------------------------------------------------------------------------
// Tests: File ownership (Unix-only, requires actual file operations)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[cfg(test)]
mod filesystem {
    use super::*;

    #[test]
    fn chown_nonexistent_file() {
        let spec = OwnerSpec { owner: Some("root".into()), group: None };
        let result = chown_file("/tmp/chown_integ_nonexistent_xyz", &spec, &ChownOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn chown_to_own_uid() {
        let path = temp_path("own_uid");
        cleanup(&path);
        fs::write(&path, "test").unwrap();

        let uid = unsafe { libc::getuid() };
        let spec = OwnerSpec { owner: Some(uid.to_string()), group: None };
        let result = chown_file(&path, &spec, &ChownOptions::default());
        assert!(result.is_ok());

        cleanup(&path);
    }

    #[test]
    fn chown_to_own_uid_and_gid() {
        let path = temp_path("own_uid_gid");
        cleanup(&path);
        fs::write(&path, "test").unwrap();

        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        let spec = OwnerSpec {
            owner: Some(uid.to_string()),
            group: Some(gid.to_string()),
        };
        let result = chown_file(&path, &spec, &ChownOptions::default());
        assert!(result.is_ok());

        cleanup(&path);
    }

    #[test]
    fn chown_verbose_output() {
        let path = temp_path("verbose");
        cleanup(&path);
        fs::write(&path, "test").unwrap();

        let uid = unsafe { libc::getuid() };
        let spec = OwnerSpec { owner: Some(uid.to_string()), group: None };
        let opts = ChownOptions { verbose: true, ..Default::default() };
        let result = chown_file(&path, &spec, &opts);
        let messages = result.unwrap();
        assert!(!messages.is_empty());
        assert!(messages[0].contains("ownership"));

        cleanup(&path);
    }

    #[test]
    fn chown_recursive() {
        let dir = temp_path("recursive");
        cleanup(&dir);
        fs::create_dir_all(format!("{}/sub", dir)).unwrap();
        fs::write(format!("{}/sub/file.txt", dir), "data").unwrap();

        let uid = unsafe { libc::getuid() };
        let spec = OwnerSpec { owner: Some(uid.to_string()), group: None };
        let opts = ChownOptions { recursive: true, ..Default::default() };
        let result = chown_file(&dir, &spec, &opts);
        assert!(result.is_ok());

        cleanup(&dir);
    }

    #[test]
    fn chown_group_only() {
        let path = temp_path("group_only");
        cleanup(&path);
        fs::write(&path, "test").unwrap();

        let gid = unsafe { libc::getgid() };
        let spec = OwnerSpec { owner: None, group: Some(gid.to_string()) };
        let result = chown_file(&path, &spec, &ChownOptions::default());
        assert!(result.is_ok());

        cleanup(&path);
    }
}
