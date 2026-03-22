//! # Integration Tests for mv
//!
//! These tests verify that the `mv` JSON spec integrates correctly
//! with CLI Builder, and that the move/rename business logic handles
//! renaming, moving into directories, and no-clobber correctly.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::mv_tool::{move_file, MvOptions};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("mv.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load mv.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("mv_integ_{}", name))
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
        assert!(spec.is_ok(), "mv.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_basic_move() {
        match parse_argv(&["mv", "src.txt", "dst.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["mv", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("mv"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["mv", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_with_no_clobber() {
        match parse_argv(&["mv", "-n", "src", "dst"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("no_clobber"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_force() {
        match parse_argv(&["mv", "-f", "src", "dst"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("force"), Some(&serde_json::json!(true)));
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
    fn move_rename_file() {
        let src = temp_path("rename_src");
        let dst = temp_path("rename_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "data").unwrap();
        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "data");

        cleanup(&dst);
    }

    #[test]
    fn move_nonexistent_source_fails() {
        let result = move_file(
            "/tmp/mv_integ_nonexistent_xyz",
            "/tmp/mv_integ_dst_xyz",
            &MvOptions::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No such file"));
    }

    #[test]
    fn no_clobber_prevents_overwrite() {
        let src = temp_path("nc_src");
        let dst = temp_path("nc_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new data").unwrap();
        fs::write(&dst, "original data").unwrap();

        let opts = MvOptions { no_clobber: true, ..Default::default() };
        assert!(move_file(&src, &dst, &opts).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "original data");
        // Source should still exist (move was skipped)
        assert!(Path::new(&src).exists());

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn move_into_directory() {
        let src = temp_path("into_dir_src");
        let dst_dir = temp_path("into_dir_dst");
        cleanup(&src);
        cleanup(&dst_dir);

        fs::write(&src, "data").unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        assert!(move_file(&src, &dst_dir, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());

        let expected = Path::new(&dst_dir).join("mv_integ_into_dir_src");
        assert!(expected.exists());
        assert_eq!(fs::read_to_string(expected).unwrap(), "data");

        cleanup(&dst_dir);
    }

    #[test]
    fn move_overwrites_by_default() {
        let src = temp_path("overwrite_src");
        let dst = temp_path("overwrite_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::write(&src, "new data").unwrap();
        fs::write(&dst, "old data").unwrap();

        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new data");

        cleanup(&dst);
    }

    #[test]
    fn move_directory() {
        let src = temp_path("dir_mv_src");
        let dst = temp_path("dir_mv_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::create_dir_all(format!("{}/sub", src)).unwrap();
        fs::write(format!("{}/sub/file.txt", src), "nested").unwrap();

        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert_eq!(
            fs::read_to_string(format!("{}/sub/file.txt", dst)).unwrap(),
            "nested"
        );

        cleanup(&dst);
    }

    #[test]
    fn move_preserves_content() {
        let src = temp_path("preserve_src");
        let dst = temp_path("preserve_dst");
        cleanup(&src);
        cleanup(&dst);

        let content = "line1\nline2\nline3\n";
        fs::write(&src, content).unwrap();
        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert_eq!(fs::read_to_string(&dst).unwrap(), content);

        cleanup(&dst);
    }

    #[test]
    fn move_empty_directory() {
        let src = temp_path("empty_dir_src");
        let dst = temp_path("empty_dir_dst");
        cleanup(&src);
        cleanup(&dst);

        fs::create_dir_all(&src).unwrap();
        assert!(move_file(&src, &dst, &MvOptions::default()).is_ok());
        assert!(!Path::new(&src).exists());
        assert!(Path::new(&dst).is_dir());

        cleanup(&dst);
    }
}
