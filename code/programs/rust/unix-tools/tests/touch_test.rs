//! # Integration Tests for touch

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::touch_tool::touch_file;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("touch.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load touch.json");
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
    fn no_create_flag() {
        match parse_argv(&["touch", "-c", "file"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("no_create"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["touch", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["touch", "--version"]) {
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
    fn create_new_file() {
        let path = std::env::temp_dir().join("touch_int_new");
        let _ = fs::remove_file(&path);
        let path_str = path.to_string_lossy().into_owned();
        assert!(touch_file(&path_str, false).is_ok());
        assert!(Path::new(&path_str).exists());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn no_create_skips() {
        let path = std::env::temp_dir().join("touch_int_nocreate");
        let _ = fs::remove_file(&path);
        let path_str = path.to_string_lossy().into_owned();
        let result = touch_file(&path_str, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
        assert!(!Path::new(&path_str).exists());
    }
}
