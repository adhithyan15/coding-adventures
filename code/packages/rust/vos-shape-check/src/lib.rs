//! # vos-shape-check: the component shape checker (VOS-D002, VOS03)
//!
//! The verified OS (VOS00) is built from *components*: small, total, bounded,
//! by-value step functions whose contracts are proved on the compiled RV32
//! image.  That proof only works if the source stays inside a narrow shape:
//! no heap, no global state, no recursion, no unbounded loop, no division, no
//! panic, fixed-width types, no code the analyzer cannot see.  VOS01 section 1.4
//! lists those rules as SH1 to SH15; VOS02 adds SH17 and SH18 for the *ghost
//! face*.  Existing analyzers check little of it (VER08a bans `unsafe` and
//! foreign linkage and sees `std` capability use, and that is nearly all), so
//! this crate is the missing, VOS-owned check.
//!
//! ## What it is, and is not
//!
//! It is a **lexical** check: it tokenises the source and `Cargo.toml` and
//! looks for tokens.  A pass means "no occurrence the lexer can see".  It does
//! not expand macros (it bans them), resolve names, or run anything.  What a
//! lexical check cannot see is written down rule by rule in
//! `code/specs/VOS03-component-shape-checker.md` section 6, and is the job of
//! the image-level companions (VOS-026, VOS-028).
//!
//! ## Layout
//!
//! ```text
//!   lexer        source text  -> tokens (with lines), bracket pairing
//!   toml         Cargo.toml   -> flattened  path = value  entries
//!   manifest     vos.toml (class, kind, evidence features), Cargo facts
//!   syntax       attributes, item extents, the #[cfg(test)] mask
//!   items        struct / enum / type definitions
//!   rules        SH1-SH4, SH6-SH11, SH15 over tokens (and which class binds what)
//!   graph        SH5 recursion over a by-name call graph
//!   ghost        SH13, SH14, SH17, SH18 (the ghost face)
//!   cargo_rules  SH12, SH15 (Cargo.toml), crate-root SH1/SH2, VT02
//!   report       findings, deterministic rendering, exit codes
//! ```
//!
//! The library is **pure**: text in, [`Report`] out.  Only the binary
//! (`src/main.rs`, `src/loader.rs`) touches the filesystem.
//!
//! ## Example
//!
//! ```
//! use vos_shape_check::{check_crate, CrateInput, Policy, SourceFile};
//!
//! let input = CrateInput {
//!     cargo_toml: Some("[package]\nname = \"vos-demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n".into()),
//!     vos_toml: None,
//!     root_entries: vec!["Cargo.toml".into(), "src".into()],
//!     sources: vec![SourceFile {
//!         path: "src/lib.rs".into(),
//!         text: "#![no_std]\n#![forbid(unsafe_code)]\npub fn half(x: u8) -> u8 { x >> 1 }\n".into(),
//!     }],
//! };
//! assert!(check_crate(&input, &Policy::default()).passed());
//! ```

#![forbid(unsafe_code)]

pub mod cargo_rules;
pub mod ghost;
pub mod graph;
pub mod items;
pub mod lexer;
pub mod manifest;
pub mod report;
pub mod rules;
pub mod syntax;
pub mod toml;

use std::collections::BTreeSet;

use manifest::{Class, ClassSource, CARGO_TOML_MAX_BYTES};
use report::finish;
pub use report::{Finding, Report, Rule};
use rules::Ctx;
use syntax::FileCtx;

/// The initial central evidence-feature list (VOS02 section 7.3).  The
/// authority is `vos-link`; its list is passed in through [`Policy`].
pub const DEFAULT_EVIDENCE_FEATURES: &[&str] = &["ghost", "cov"];

/// Per-file cap of VOS03 section 1.
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
/// Most source files in one crate.
pub const MAX_SOURCE_FILES: usize = 512;
/// Most source bytes in one crate.
pub const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;

/// One source file: crate-relative `/`-separated path and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// For example `src/lib.rs`.
    pub path: String,
    /// The file text.
    pub text: String,
}

/// Everything the checker reads about one crate, already in memory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrateInput {
    /// `Cargo.toml` text, if the file exists.
    pub cargo_toml: Option<String>,
    /// `vos.toml` text, if the file exists.
    pub vos_toml: Option<String>,
    /// Names of the entries of the crate root (to find `build.rs`).
    pub root_entries: Vec<String>,
    /// Every `*.rs` file under `src/`.
    pub sources: Vec<SourceFile>,
}

/// Policy supplied by the caller (the central lists of VOS02 section 7.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The central evidence-feature list.
    pub evidence_features: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            evidence_features: DEFAULT_EVIDENCE_FEATURES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }
}

fn input_finding(out: &mut Vec<Finding>, file: &str, line: u32, message: impl Into<String>) {
    out.push(Finding {
        rule: Rule::Vt03,
        file: file.to_string(),
        line,
        message: message.into(),
    });
}

/// Check one crate.  Pure: the same input always yields the same report.
pub fn check_crate(input: &CrateInput, policy: &Policy) -> Report {
    let mut early: Vec<Finding> = Vec::new();

    // vos.toml: class, kind, declared evidence features.
    let mut class = Class::K;
    let mut class_source = ClassSource::Default;
    let mut kind = None;
    let mut declared: Vec<String> = Vec::new();
    if let Some(text) = &input.vos_toml {
        match manifest::parse_vos_toml(text) {
            Ok(v) => {
                if let Some(c) = v.class {
                    class = c;
                    class_source = ClassSource::VosToml;
                }
                kind = v.kind;
                for name in &v.evidence_features {
                    if !policy.evidence_features.contains(name) {
                        early.push(Finding {
                            rule: Rule::Vt01,
                            file: "vos.toml".into(),
                            line: 1,
                            message: format!(
                                "evidence feature `{name}` is not in the central list"
                            ),
                        });
                    }
                }
                declared = v.evidence_features;
            }
            Err((line, message)) => early.push(Finding {
                rule: Rule::Vt01,
                file: "vos.toml".into(),
                line,
                message,
            }),
        }
    }

    // Cargo.toml.
    let mut package = None;
    let cargo = match &input.cargo_toml {
        None => {
            input_finding(&mut early, "Cargo.toml", 1, "Cargo.toml is missing");
            None
        }
        Some(text) if text.len() > CARGO_TOML_MAX_BYTES => {
            input_finding(&mut early, "Cargo.toml", 1, "Cargo.toml is over 64 KiB");
            None
        }
        Some(text) => match manifest::parse_cargo(text) {
            Ok(c) => {
                package = c.name.clone();
                Some(c)
            }
            Err(e) => {
                input_finding(
                    &mut early,
                    "Cargo.toml",
                    e.line,
                    format!("unparsable Cargo.toml: {}", e.message),
                );
                None
            }
        },
    };

    // Source caps and lexing.
    let mut sources: Vec<&SourceFile> = input.sources.iter().collect();
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    let total: usize = sources.iter().map(|s| s.text.len()).sum();
    if sources.len() > MAX_SOURCE_FILES || total > MAX_TOTAL_BYTES {
        input_finding(
            &mut early,
            "src",
            1,
            "source file count or total size is over the cap (512 files, 8 MiB)",
        );
        sources.clear();
    }
    let mut files: Vec<FileCtx> = Vec::new();
    for s in sources {
        if s.text.len() > MAX_SOURCE_BYTES {
            input_finding(&mut early, &s.path, 1, "source file is over 1 MiB");
            continue;
        }
        match FileCtx::load(&s.path, &s.text) {
            Ok(f) => files.push(f),
            Err(e) => input_finding(
                &mut early,
                &s.path,
                e.line,
                format!("cannot lex: {} (fail closed)", e.message),
            ),
        }
    }

    // A component crate is a library: `src/lib.rs` is its one root.  Binaries
    // (`src/main.rs`, `src/bin/**`) are second roots the root checks would
    // not cover, so they are findings (SH15), not alternative roots.
    let root = files.iter().find(|f| f.path == "src/lib.rs");
    if root.is_none() && !input.sources.iter().any(|s| s.path == "src/lib.rs") {
        input_finding(&mut early, "src/lib.rs", 1, "no crate root (src/lib.rs)");
    }
    let binaries: Vec<&str> = input
        .sources
        .iter()
        .map(|s| s.path.as_str())
        .filter(|p| *p == "src/main.rs" || p.starts_with("src/bin/"))
        .collect();

    let mut evidence: BTreeSet<String> = policy.evidence_features.iter().cloned().collect();
    evidence.extend(declared);
    let types: Vec<items::TypeDef> = items::collect_types(&files);
    let im = rules::interior_mutable(&types);
    let mut cx = Ctx {
        class,
        kind,
        files: &files,
        types,
        cargo: cargo.as_ref(),
        evidence: &evidence,
        out: early,
    };
    let all = cx.files;
    for f in all {
        rules::file_rules(&mut cx, f, &im);
        cargo_rules::attr_rules(&mut cx, f);
    }
    if let Some(r) = root {
        cargo_rules::root_rules(&mut cx, r);
    }
    for p in binaries {
        cx.add(
            Rule::Sh15,
            p,
            1,
            "binary target file; a component crate is a library",
        );
    }
    graph::recursion(&mut cx);
    ghost::ghost_rules(&mut cx);
    if let Some(c) = cargo.as_ref() {
        cargo_rules::manifest_rules(&mut cx, c, &input.root_entries);
    }
    let (findings, dropped) = finish(cx.out);
    Report {
        package,
        class,
        class_source,
        kind,
        findings,
        dropped,
    }
}
