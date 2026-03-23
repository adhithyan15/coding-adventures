//! # Integration Tests for sha256sum
//!
//! These tests verify that the `sha256sum` JSON spec integrates
//! correctly with CLI Builder, and that the SHA-256 implementation
//! produces correct hashes matching known test vectors from NIST.

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::sha256sum_tool::compute_sha256;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn spec_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("sha256sum.json").to_string_lossy().into_owned()
}

fn make_parser() -> Parser {
    let spec = load_spec_from_file(&spec_path()).expect("failed to load sha256sum.json");
    Parser::new(spec)
}

fn parse_argv(argv: &[&str]) -> ParserOutput {
    let parser = make_parser();
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    parser.parse(&args).expect("parse failed")
}

// ---------------------------------------------------------------------------
// Test: Spec loads
// ---------------------------------------------------------------------------

#[cfg(test)]
mod spec_loading {
    use super::*;

    #[test]
    fn spec_loads() {
        assert!(load_spec_from_file(&spec_path()).is_ok());
    }
}

// ---------------------------------------------------------------------------
// Test: CLI parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cli_parsing {
    use super::*;

    #[test]
    fn parse_with_file() {
        match parse_argv(&["sha256sum", "file.txt"]) {
            ParserOutput::Parse(_) => {}
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    #[test]
    fn help() {
        match parse_argv(&["sha256sum", "--help"]) {
            ParserOutput::Help(h) => assert!(h.text.contains("sha256sum")),
            other => panic!("expected Help, got {:?}", other),
        }
    }

    #[test]
    fn version() {
        match parse_argv(&["sha256sum", "--version"]) {
            ParserOutput::Version(v) => assert_eq!(v.version, "1.0.0"),
            _ => panic!("expected Version"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test: Business logic — known test vectors
// ---------------------------------------------------------------------------

#[cfg(test)]
mod business_logic {
    use super::*;

    #[test]
    fn sha256_empty() {
        assert_eq!(
            compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hello() {
        assert_eq!(
            compute_sha256(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_abc() {
        assert_eq!(
            compute_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_quick_brown_fox() {
        assert_eq!(
            compute_sha256(b"The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    #[test]
    fn sha256_hello_world() {
        assert_eq!(
            compute_sha256(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn deterministic() {
        let h1 = compute_sha256(b"test");
        let h2 = compute_sha256(b"test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn output_is_64_hex_chars() {
        let hash = compute_sha256(b"anything");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn case_sensitive() {
        assert_ne!(compute_sha256(b"hello"), compute_sha256(b"Hello"));
    }

    #[test]
    fn multi_block_message() {
        // 128 bytes — spans two 64-byte blocks
        let data = vec![b'X'; 128];
        let hash = compute_sha256(&data);
        assert_eq!(hash.len(), 64);
    }
}
