//! # Integration Tests for ln

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::ln_tool::create_link;
use std::path::PathBuf;

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("ln.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load ln.json");
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
    fn symbolic_flag() {
        match parse_argv(&["ln", "-s", "target"]) {
            ParserOutput::Parse(result) => {
                assert_eq!(result.flags.get("symbolic"), Some(&serde_json::json!(true)));
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn help_returns_help() {
        match parse_argv(&["ln", "--help"]) {
            ParserOutput::Help(_) => {}
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version_returns_version() {
        match parse_argv(&["ln", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

#[cfg(test)]
mod business_logic {
    use super::*;
    use std::fs;

    #[test]
    fn hard_link() {
        let target = std::env::temp_dir().join("ln_int_target");
        let link = std::env::temp_dir().join("ln_int_hard");
        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&link);
        fs::write(&target, "hello").unwrap();
        let t = target.to_string_lossy().into_owned();
        let l = link.to_string_lossy().into_owned();
        assert!(create_link(&t, &l, false, false).is_ok());
        assert_eq!(fs::read_to_string(&link).unwrap(), "hello");
        fs::remove_file(&target).ok();
        fs::remove_file(&link).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link() {
        let target = std::env::temp_dir().join("ln_int_sym_target");
        let link = std::env::temp_dir().join("ln_int_sym_link");
        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&link);
        fs::write(&target, "hello").unwrap();
        let t = target.to_string_lossy().into_owned();
        let l = link.to_string_lossy().into_owned();
        assert!(create_link(&t, &l, true, false).is_ok());
        let read_link = fs::read_link(&link).unwrap();
        assert_eq!(read_link.to_string_lossy(), t);
        fs::remove_file(&target).ok();
        fs::remove_file(&link).ok();
    }
}
