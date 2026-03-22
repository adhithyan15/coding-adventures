//! # Integration Tests for rm

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::rm_tool::remove_path;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("rm.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load rm.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        assert!(load_spec_from_file(&spec_path()).is_ok());
    }
}

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn force_flag() {
        match parse_argv(&["rm", "-f", "file"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("force"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn recursive_flag() {
        match parse_argv(&["rm", "-r", "dir"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("recursive"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["rm", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["rm", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn remove_file() {
        let path = std::env::temp_dir().join("rm_int_file");
        fs::write(&path, "data").unwrap();
        let p = path.to_string_lossy().into_owned();
        assert!(remove_path(&p, false, false, false).is_ok());
        assert!(!Path::new(&p).exists());
    }

    #[test]
    fn force_nonexistent() {
        assert!(remove_path("/tmp/rm_int_nonexistent_xyz", false, false, true).is_ok());
    }

    #[test]
    fn recursive_dir() {
        let dir = std::env::temp_dir().join("rm_int_dir");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/file.txt"), "data").unwrap();
        let p = dir.to_string_lossy().into_owned();
        assert!(remove_path(&p, true, false, false).is_ok());
        assert!(!Path::new(&p).exists());
    }
}
