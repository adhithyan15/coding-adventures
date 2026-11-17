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
//! It is a **lexical** check: it tokenises the source (with the repository's
//! grammar-driven lexer, never a hand-written one) and `Cargo.toml` and
//! looks for tokens.  A pass means "no occurrence the lexer can see".  It does
//! not expand macros (it bans them), resolve names, or run anything.  What a
//! lexical check cannot see is written down rule by rule in
//! `code/specs/VOS03-component-shape-checker.md` section 6, and is the job of
//! the image-level companions (VOS-026, VOS-028).
//!
//! ## Layout
//!
//! ```text
//!   lexer        source text  -> tokens, via the grammar-driven RS01 lexer
//!                (shebang and comment hooks, one lexer per run), bracket pairing
//!   toml         Cargo.toml   -> flattened  path = value  entries, via the
//!                repository's TOML lexer and parser (duplicates, escapes refused)
//!   manifest     vos.toml (class, kind, evidence features), Cargo facts
//!   syntax       attributes, item extents, the #[cfg(test)] mask
//!   items        struct / enum / type definitions
//!   rules        SH1-SH4, SH6-SH11, SH15 over tokens (and which class binds what)
//!   graph        SH5 recursion over a by-name call graph
//!   ghost        SH13, SH14, SH17, SH18 (the ghost face)
//!   cargo_rules  SH12, SH15 (Cargo.toml), crate-root SH1/SH2, VT02
//!   steps        a deterministic work counter (linearity tests count, not time)
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
pub mod steps;
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
/// Most source bytes in one crate (the tokens of all files are held at once).
pub const MAX_TOTAL_BYTES: usize = 1024 * 1024;

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

/// The lexing outcome of one source file (`None`: not lexed at all).
type Lexed = Option<Result<Vec<lexer::Token>, lexer::LexError>>;

/// Everything decided about one crate before its sources are lexed.
struct Prepared {
    early: Vec<Finding>,
    package: Option<String>,
    cargo: Option<manifest::CargoFacts>,
    class: Class,
    class_source: ClassSource,
    kind: Option<manifest::Kind>,
    declared: Vec<String>,
    edition: lexer::Edition,
    /// Indexes into `input.sources`, sorted by path, of the files to lex.
    docs: Vec<usize>,
}

fn prepare(input: &CrateInput, policy: &Policy) -> Prepared {
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
    // The lexical pair is chosen by the declared edition (SH4 allows 2021 and
    // 2024); anything else is an SH4 finding elsewhere and is lexed as 2021.
    let edition = match cargo.as_ref().and_then(|c| c.edition.as_deref()) {
        Some("2024") => lexer::Edition::E2024,
        _ => lexer::Edition::E2021,
    };

    // Source caps.
    let mut order: Vec<usize> = (0..input.sources.len()).collect();
    order.sort_by(|&a, &b| input.sources[a].path.cmp(&input.sources[b].path));
    let total: usize = input.sources.iter().map(|s| s.text.len()).sum();
    if input.sources.len() > MAX_SOURCE_FILES || total > MAX_TOTAL_BYTES {
        input_finding(
            &mut early,
            "src",
            1,
            "source file count or total size is over the cap (512 files, 1 MiB)",
        );
        order.clear();
    }
    let mut docs = Vec::new();
    for i in order {
        let s = &input.sources[i];
        if s.text.len() > MAX_SOURCE_BYTES {
            input_finding(&mut early, &s.path, 1, "source file is over 1 MiB");
        } else {
            docs.push(i);
        }
    }
    Prepared {
        early,
        package,
        cargo,
        class,
        class_source,
        kind,
        declared,
        edition,
        docs,
    }
}

/// Check one crate.  Pure: the same input always yields the same report.
pub fn check_crate(input: &CrateInput, policy: &Policy) -> Report {
    check_crates(std::slice::from_ref(input), policy).remove(0)
}

/// [`check_crate`] plus the number of work steps it took (see [`steps`]).
///
/// The count is deterministic (the same input always counts the same), so a
/// test can assert that the work grows linearly with the input without
/// reading a clock.  It includes lexing, which is linear by construction.
pub fn check_crate_counted(input: &CrateInput, policy: &Policy) -> (Report, u64) {
    let before = steps::steps();
    let report = check_crate(input, policy);
    (report, steps::steps().wrapping_sub(before))
}

/// Check several crates, lexing ALL their sources with one grammar lexer per
/// edition (a `GrammarLexer` costs about 175 ms to build and cannot be reused,
/// so a run builds it once; see [`lexer`]).  Report `i` is for `inputs[i]`.
pub fn check_crates(inputs: &[CrateInput], policy: &Policy) -> Vec<Report> {
    let mut prepared: Vec<Prepared> = inputs.iter().map(|i| prepare(i, policy)).collect();
    // Lex every document of every crate, grouped by edition.
    let mut lexed: Vec<Vec<Lexed>> = prepared.iter().map(|p| vec![None; p.docs.len()]).collect();
    for edition in [lexer::Edition::E2021, lexer::Edition::E2024] {
        let mut slots: Vec<(usize, usize)> = Vec::new();
        let mut texts: Vec<&str> = Vec::new();
        for (ci, p) in prepared.iter().enumerate() {
            if p.edition == edition {
                for (di, &si) in p.docs.iter().enumerate() {
                    slots.push((ci, di));
                    texts.push(inputs[ci].sources[si].text.as_str());
                }
            }
        }
        if texts.is_empty() {
            continue;
        }
        for ((ci, di), r) in slots.into_iter().zip(lexer::lex_documents(&texts, edition)) {
            lexed[ci][di] = Some(r);
        }
    }
    // One hostile crate must not fail the others.  A forged or swallowed
    // boundary, or a batch that ran out of attempts, marks every file of the
    // batch; so each crate that got such an error is lexed again ALONE (its own
    // lexer), and only it keeps the failure if the failure is really its own.
    for ci in 0..prepared.len() {
        let hit = lexed[ci].iter().any(|r| {
            matches!(r, Some(Err(e)) if e.message.starts_with("not lexed")
                || e.message.starts_with("the file-boundary check failed"))
        });
        if !hit {
            continue;
        }
        let texts: Vec<&str> = prepared[ci]
            .docs
            .iter()
            .map(|&si| inputs[ci].sources[si].text.as_str())
            .collect();
        for (di, r) in lexer::lex_documents(&texts, prepared[ci].edition)
            .into_iter()
            .enumerate()
        {
            lexed[ci][di] = Some(r);
        }
    }
    let mut reports = Vec::with_capacity(inputs.len());
    for (ci, input) in inputs.iter().enumerate() {
        let p = std::mem::replace(
            &mut prepared[ci],
            Prepared {
                early: Vec::new(),
                package: None,
                cargo: None,
                class: Class::K,
                class_source: ClassSource::Default,
                kind: None,
                declared: Vec::new(),
                edition: lexer::Edition::E2021,
                docs: Vec::new(),
            },
        );
        let results = std::mem::take(&mut lexed[ci]);
        reports.push(analyse(input, policy, p, results));
    }
    reports
}

fn analyse(input: &CrateInput, policy: &Policy, p: Prepared, results: Vec<Lexed>) -> Report {
    let Prepared {
        mut early,
        package,
        cargo,
        class,
        class_source,
        kind,
        declared,
        docs,
        ..
    } = p;
    let mut files: Vec<FileCtx> = Vec::new();
    for (&si, r) in docs.iter().zip(results) {
        let path = &input.sources[si].path;
        let outcome = match r {
            Some(Ok(toks)) => FileCtx::from_tokens(path, toks),
            Some(Err(e)) => Err(e),
            None => Err(lexer::LexError {
                line: 1,
                message: "not lexed".to_string(),
            }),
        };
        match outcome {
            Ok(f) => files.push(f),
            Err(e) => input_finding(
                &mut early,
                path,
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
    let (types, gaps) = items::collect_types(&files);
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
    for g in &gaps {
        let file = files[g.file].path.clone();
        cx.add(Rule::Vt03, &file, g.line, g.message.clone());
    }
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
