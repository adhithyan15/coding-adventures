//! # Integration Tests for cp
//!
//! These tests verify that the `cp` JSON spec integrates correctly
//! with CLI Builder, and that the copy business logic handles
//! single files, directories, no-clobber, and force correctly.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::cp_tool::{copy_file, CpOptions};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("cp.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load cp.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("cp_integ_{}", name))
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
// Test: Spec loads
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        let spec = load_spec_from_file(&spec_path());
        assert!(spec.is_ok(), "cp.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_basic_copy() {
        match parse_argv(&["cp", "src.txt", "dst.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["cp", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("cp"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["cp", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_with_recursive_flag() {
        match parse_argv(&["cp", "-R", "dir1", "dir2"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("recursive"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_no_clobber() {
        match parse_argv(&["cp", "-n", "src", "dst"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("no_clobber"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn copy_single_file() {
        let src = temp_path("bl_src");
        let dst = temp_path("bl_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "hello world").unwrap();
        let opts = CpOptions::default();
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello world");
        // Source should still exist (it's a copy, not move)
        assert!(Path::new(&src).exists());

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_nonexistent_source_fails() {
        let result = copy_file(
            "/tmp/cp_integ_nonexistent_xyz",
            "/tmp/cp_integ_dst_xyz",
            &CpOptions::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No such file"));
    }

    #[test]
    fn no_clobber_prevents_overwrite() {
        let src = temp_path("bl_nc_src");
        let dst = temp_path("bl_nc_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new content").unwrap();
        fs::write(&dst, "original content").unwrap();

        let opts = CpOptions { no_clobber: true, ..Default::default() };
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "original content");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn force_removes_and_copies() {
        let src = temp_path("bl_force_src");
        let dst = temp_path("bl_force_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new data").unwrap();
        fs::write(&dst, "old data").unwrap();

        let opts = CpOptions { force: true, ..Default::default() };
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new data");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_directory_without_recursive_fails() {
        let src_dir = temp_path("bl_dir_norec");
        cleanup(&src_dir);
        fs::create_dir_all(&src_dir).unwrap();

        let result = copy_file(&src_dir, "/tmp/cp_integ_dst_dir", &CpOptions::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not specified"));

        cleanup(&src_dir);
    }

    #[test]
    fn copy_directory_recursively() {
        let src_dir = temp_path("bl_dir_rec");
        let dst_dir = temp_path("bl_dir_rec_dst");
        cleanup(&src_dir);
        cleanup(&dst_dir);

        fs::create_dir_all(format!("{}/sub", src_dir)).unwrap();
        fs::write(format!("{}/file1.txt", src_dir), "file1").unwrap();
        fs::write(format!("{}/sub/file2.txt", src_dir), "file2").unwrap();

        let opts = CpOptions { recursive: true, ..Default::default() };
        assert!(copy_file(&src_dir, &dst_dir, &opts).is_ok());
        assert!(Path::new(&dst_dir).is_dir());
        assert_eq!(
            fs::read_to_string(format!("{}/file1.txt", dst_dir)).unwrap(),
            "file1"
        );
        assert_eq!(
            fs::read_to_string(format!("{}/sub/file2.txt", dst_dir)).unwrap(),
            "file2"
        );

        cleanup(&src_dir);
        cleanup(&dst_dir);
    }

    #[test]
    fn copy_into_existing_directory() {
        let src = temp_path("bl_into_dir_src");
        let dst_dir = temp_path("bl_into_dir_dst");
        cleanup(&src);
        cleanup(&dst_dir);

        fs::write(&src, "data").unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        assert!(copy_file(&src, &dst_dir, &CpOptions::default()).is_ok());

        let expected = Path::new(&dst_dir).join("cp_integ_bl_into_dir_src");
        assert!(expected.exists());

        cleanup(&src);
        cleanup(&dst_dir);
    }

    #[test]
    fn copy_overwrites_by_default() {
        let src = temp_path("bl_overwrite_src");
        let dst = temp_path("bl_overwrite_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "newer").unwrap();
        fs::write(&dst, "older").unwrap();

        assert!(copy_file(&src, &dst, &CpOptions::default()).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "newer");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_empty_directory() {
        let src = temp_path("bl_empty_dir_src");
        let dst = temp_path("bl_empty_dir_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::create_dir_all(&src).unwrap();
        let opts = CpOptions { recursive: true, ..Default::default() };
        assert!(copy_file(&src, &dst, &opts).is_ok());
        assert!(Path::new(&dst).is_dir());

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn copy_preserves_content_exactly() {
        let src = temp_path("bl_exact_src");
        let dst = temp_path("bl_exact_dst");
        cleanup(&src);
        cleanup(&dst);

        let content = "line1\nline2\nline3\n";
        fs::write(&src, content).unwrap();
        assert!(copy_file(&src, &dst, &CpOptions::default()).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), content);

        cleanup(&src);
        cleanup(&dst);
    }
}
