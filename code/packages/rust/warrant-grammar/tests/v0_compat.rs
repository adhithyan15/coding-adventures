//! VER04j section 1.3: a v0 document means exactly what it did before WX-B and
//! WX-C.  Three independent proofs:
//!
//! 1. the layout tokens, parsed declarations and checker output of the two
//!    repository specifications hash to the values the *pre-change* crate
//!    produced (printed from `origin/main` before the change and pinned here);
//! 2. every `.warrant` file under `code/specs` parses and type-checks, and the
//!    documents that existed before WX-B (`V0_DOCUMENTS`) are still v0;
//! 3. for every package that binds a specification (`evidence.bindings`),
//!    regenerating the evidence harness reproduces the checked-in file byte for
//!    byte (the same drift check as `gen_evidence --check`), so no v0 evidence
//!    changed -- examples, exhaustive, property, fuzz and negative-control
//!    suites, and the manifest code, are all in that text.
//!
//! The first v1 adopter (`wasm-leb128.warrant`, which carries the `v1` header)
//! is held to the same standard in its own lane: it must type-check, and its
//! harness must regenerate byte for byte with the bindings' host lines (exactly
//! what `gen_evidence --check` does).  It never relaxes the v0 assertions: the
//! v0 documents are pinned by name, so a v0 document cannot silently become v1.

use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits, Version};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .to_path_buf()
}

/// The documents that existed before WX-B and WX-C.  They must stay v0: that
/// is what "no existing document changes meaning" rests on.
const V0_DOCUMENTS: [&str; 2] = ["ct-compare", "gf256"];

fn warrant_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(repo_root().join("code/specs"))
        .expect("code/specs")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "warrant"))
        .collect();
    out.sort();
    out
}

#[test]
fn the_repository_specifications_hash_as_they_did_before_wx_b_and_wx_c() {
    // (name, layout tokens, declarations, checked module) from the pre-change
    // crate.  The `version` field of the new `WarrantFile` is deliberately not
    // part of the hashed text.
    let pinned = [
        (
            "ct-compare",
            "718188b45bfccaa865979f0f3f2ba36dfda3efc09e9309db76894ec395c69835",
            "98aa1c1642c7eb83a95898a46af040982b7f121e9205908625c3206d82bc666f",
            "58e250d5f141d4b96a8750ebf17de529df89cb489a4aa0df4ebd4e1300987de7",
        ),
        (
            "gf256",
            "d6d7e5819e5cfae9991c9911d4e0c4454b3c66bd7d5e0a110b7c57ae71ef624c",
            "91975bd51d0b7f793ce02fb931880c224ca44ca43dc98b80eea4249c5bf4aabc",
            "d956626c41b0fe75e81b38827deedd6ddae6739b4f222ac90bb1341446fb1580",
        ),
    ];
    for (name, tokens, declarations, checked) in pinned {
        let bytes = std::fs::read(repo_root().join(format!("code/specs/{name}.warrant"))).unwrap();
        let doc = SourceDocument::admit(&bytes, SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        assert_eq!(file.version, Version::V0, "{name}");
        let module = check_warrant(&file).unwrap();
        assert_eq!(
            sha256_hex(format!("{layout:?}").as_bytes()),
            tokens,
            "{name}: layout tokens"
        );
        assert_eq!(
            sha256_hex(format!("{:?}", file.declarations).as_bytes()),
            declarations,
            "{name}: parsed declarations"
        );
        assert_eq!(
            sha256_hex(format!("{module:?}").as_bytes()),
            checked,
            "{name}: checker output"
        );
    }
}

#[test]
fn every_warrant_file_in_the_repository_still_checks_and_the_v0_ones_stay_v0() {
    let files = warrant_files();
    assert!(
        files.len() >= 2,
        "ct-compare.warrant and gf256.warrant at least"
    );
    let mut v0_seen = Vec::new();
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let doc = SourceDocument::admit(&bytes, SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default())
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        // None of the documents that predate WX-B carries the `v1` header:
        // that is what "no existing document changes meaning" rests on.  A
        // newer document may be v1 (it opts in explicitly with the header).
        if V0_DOCUMENTS.contains(&stem.as_str()) {
            assert_eq!(file.version, Version::V0, "{}", path.display());
            v0_seen.push(stem);
        }
        check_warrant(&file).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
    }
    v0_seen.sort();
    assert_eq!(v0_seen, V0_DOCUMENTS, "every pinned v0 document is present");
}

#[test]
fn every_bound_package_regenerates_its_checked_in_evidence_byte_for_byte() {
    let root = repo_root();
    let mut checked_packages = 0;
    let mut v1_packages = 0;
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("code/packages/rust"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("evidence.bindings").is_file())
        .collect();
    dirs.sort();
    for dir in dirs {
        let package = dir.file_name().unwrap().to_string_lossy().into_owned();
        let b = parse_evidence_bindings(
            &std::fs::read_to_string(dir.join("evidence.bindings")).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{package}: {e}"));
        b.validate_paths(&package).unwrap();
        let spec = std::fs::read(root.join(&b.spec)).unwrap();
        let doc = SourceDocument::admit(&spec, SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        let stem = Path::new(&b.spec).file_stem().unwrap().to_string_lossy().into_owned();
        if V0_DOCUMENTS.contains(&stem.as_str()) {
            assert_eq!(file.version, Version::V0, "{package}'s document is v0");
        }
        let checked = check_warrant(&file).unwrap();
        if file.version == Version::V1 {
            // A v1 adopter: regenerate exactly as `gen_evidence` does, host
            // lines included, and require the checked-in file byte for byte.
            let options = EvidenceOptions {
                package_crate: b.manifest.package.clone(),
                spec_path: b.spec.clone(),
                spec_sha256: sha256_hex(&spec),
                seed: b.seed,
                property_cases: b.property_cases,
                fuzz_iterations: b.fuzz_iterations,
                negctl_fuzz_iterations: b.negctl_fuzz_iterations,
                exhaustive_budget: b.exhaustive_budget,
                arg_types: b.arg_types.clone(),
                hosts: b.hosts.clone(),
            };
            let text = compile_rust_evidence_harness(&file, &checked, &b.manifest, &options)
                .unwrap_or_else(|e| panic!("{package}: {e}"));
            let on_disk = std::fs::read(root.join(&b.output)).unwrap();
            assert!(
                on_disk == text.as_bytes(),
                "{package}: the checked-in v1 evidence drifted from the generator"
            );
            assert!(text.contains("warrant_version"), "{package}: a v1 harness records its version");
            v1_packages += 1;
            continue;
        }
        let options = EvidenceOptions {
            package_crate: b.manifest.package.clone(),
            spec_path: b.spec.clone(),
            spec_sha256: sha256_hex(&spec),
            seed: b.seed,
            property_cases: b.property_cases,
            fuzz_iterations: b.fuzz_iterations,
            negctl_fuzz_iterations: b.negctl_fuzz_iterations,
            exhaustive_budget: b.exhaustive_budget,
            arg_types: b.arg_types.clone(),
            hosts: Default::default(),
        };
        let text = compile_rust_evidence_harness(&file, &checked, &b.manifest, &options)
            .unwrap_or_else(|e| panic!("{package}: {e}"));
        let on_disk = std::fs::read(root.join(&b.output)).unwrap();
        assert!(
            on_disk == text.as_bytes(),
            "{package}: the checked-in evidence drifted from the generator"
        );
        // `gen_evidence` now passes the bindings' host lines through (gf256's
        // `args=u8,u32` is one).  For a v0 document that must not change a
        // byte: the v0 adapters read `arg_types`, never `hosts`.
        let with_hosts = EvidenceOptions {
            hosts: b.hosts.clone(),
            ..options
        };
        let driver_text = compile_rust_evidence_harness(&file, &checked, &b.manifest, &with_hosts)
            .unwrap_or_else(|e| panic!("{package}: {e}"));
        assert!(driver_text == text, "{package}: passing the host lines changed a v0 harness");
        // The v1 machinery must not leak into a v0 harness.
        for needle in [
            "F::U8",
            "V::Fixed",
            "V::Int",
            "V::Opt",
            "warrant_version",
            "fx_edge",
            "//@",
        ] {
            assert!(
                !text.contains(needle),
                "{package}: v0 harness contains `{needle}`"
            );
        }
        checked_packages += 1;
    }
    assert!(
        checked_packages >= 2,
        "ct-compare and gf256 bind specifications"
    );
    assert!(v1_packages >= 1, "wasm-leb128 binds a v1 specification");
}
