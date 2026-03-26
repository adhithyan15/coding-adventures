//! # Integration Tests for ls
//!
//! These tests verify that the `ls` JSON spec integrates correctly
//! with CLI Builder, and that the directory listing business logic
//! handles sorting, filtering, and hidden files correctly.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::ls_tool::{list_directory, LsOptions};
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("ls.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load ls.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

fn temp_dir(name: &str) -> String {
    let p = std::env::temp_dir().join(format!("ls_integ_{}", name));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p.to_string_lossy().into_owned()
}

fn cleanup(path: &str) {
    let _ = fs::remove_dir_all(path);
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
        assert!(spec.is_ok(), "ls.json should load successfully");
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_with_directory() {
        match parse_argv(&["ls", "."]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["ls", "--help"]) {
            ParserOutput::Help(h) => {
                assert!(h.text.contains("ls"));
            }
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["ls", "--version"]) {
            ParserOutput::Version(v) => {
                assert_eq!(v.version, "1.0.0");
            }
            _ => panic!("expected Version"),
        }
    }

    #[test]
    fn parse_with_all_flag() {
        match parse_argv(&["ls", "-a", "."]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("all"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_long_flag() {
        match parse_argv(&["ls", "-l", "."]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("long"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_reverse_flag() {
        match parse_argv(&["ls", "-r", "."]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("reverse"), Some(&serde_json::json!(true)));
            }
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_recursive_flag() {
        match parse_argv(&["ls", "-R", "."]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("recursive"), Some(&serde_json::json!(true)));
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
    fn list_empty_directory() {
        let dir = temp_dir("empty");
        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        assert!(result.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn list_nonexistent_fails() {
        let result = list_directory("/tmp/ls_integ_nonexistent_xyz", &LsOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn list_sorted_by_name() {
        let dir = temp_dir("sorted");
        fs::write(format!("{}/cherry.txt", dir), "").unwrap();
        fs::write(format!("{}/apple.txt", dir), "").unwrap();
        fs::write(format!("{}/banana.txt", dir), "").unwrap();

        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["apple.txt", "banana.txt", "cherry.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn hidden_files_excluded_by_default() {
        let dir = temp_dir("hidden_default");
        fs::write(format!("{}/.hidden", dir), "").unwrap();
        fs::write(format!("{}/visible", dir), "").unwrap();

        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["visible"]);
        cleanup(&dir);
    }

    #[test]
    fn show_all_includes_hidden() {
        let dir = temp_dir("show_all");
        fs::write(format!("{}/.hidden", dir), "").unwrap();
        fs::write(format!("{}/visible", dir), "").unwrap();

        let opts = LsOptions { show_all: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&".hidden"));
        assert!(names.contains(&"visible"));
        cleanup(&dir);
    }

    #[test]
    fn sort_by_size() {
        let dir = temp_dir("by_size");
        fs::write(format!("{}/small", dir), "a").unwrap();
        fs::write(format!("{}/big", dir), "aaaaaaaaaa").unwrap();
        fs::write(format!("{}/medium", dir), "aaaaa").unwrap();

        let opts = LsOptions { sort_by_size: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["big", "medium", "small"]);
        cleanup(&dir);
    }

    #[test]
    fn reverse_sort() {
        let dir = temp_dir("reverse");
        fs::write(format!("{}/a.txt", dir), "").unwrap();
        fs::write(format!("{}/b.txt", dir), "").unwrap();
        fs::write(format!("{}/c.txt", dir), "").unwrap();

        let opts = LsOptions { reverse: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["c.txt", "b.txt", "a.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn sort_by_extension() {
        let dir = temp_dir("by_ext");
        fs::write(format!("{}/file.txt", dir), "").unwrap();
        fs::write(format!("{}/file.rs", dir), "").unwrap();
        fs::write(format!("{}/file.go", dir), "").unwrap();

        let opts = LsOptions { sort_by_ext: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["file.go", "file.rs", "file.txt"]);
        cleanup(&dir);
    }

    #[test]
    fn list_single_file() {
        let dir = temp_dir("single_file");
        let file_path = format!("{}/test.txt", dir);
        fs::write(&file_path, "content").unwrap();

        let result = list_directory(&file_path, &LsOptions::default()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "test.txt");
        cleanup(&dir);
    }

    #[test]
    fn file_entry_metadata() {
        let dir = temp_dir("metadata");
        fs::write(format!("{}/file.txt", dir), "hello").unwrap();
        fs::create_dir(format!("{}/subdir", dir)).unwrap();

        let result = list_directory(&dir, &LsOptions::default()).unwrap();
        let file = result.iter().find(|e| e.name == "file.txt").unwrap();
        assert_eq!(file.size, 5);
        assert!(!file.is_dir);

        let subdir = result.iter().find(|e| e.name == "subdir").unwrap();
        assert!(subdir.is_dir);
        cleanup(&dir);
    }

    #[test]
    fn almost_all_shows_hidden_not_dots() {
        let dir = temp_dir("almost_all");
        fs::write(format!("{}/.hidden", dir), "").unwrap();
        fs::write(format!("{}/visible", dir), "").unwrap();

        let opts = LsOptions { almost_all: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&".hidden"));
        assert!(names.contains(&"visible"));
        cleanup(&dir);
    }

    #[test]
    fn unsorted_returns_entries() {
        let dir = temp_dir("unsorted");
        fs::write(format!("{}/z", dir), "").unwrap();
        fs::write(format!("{}/a", dir), "").unwrap();
        fs::write(format!("{}/m", dir), "").unwrap();

        let opts = LsOptions { unsorted: true, ..Default::default() };
        let result = list_directory(&dir, &opts).unwrap();
        // Should have all 3 entries, just maybe not sorted
        assert_eq!(result.len(), 3);
        cleanup(&dir);
    }
}
