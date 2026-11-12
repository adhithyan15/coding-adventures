//! Regenerate (or check) the Warrant-derived evidence harness of a package.
//!
//! ```text
//! cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- [--check] [<package> ...]
//! ```
//!
//! `<package>` is a directory name under `code/packages/rust` (for example
//! `ct-compare` or `gf256`); with no package every directory that holds an
//! `evidence.bindings` file is processed.  For each package the example reads
//! `<package>/evidence.bindings` (the only per-package input: which public
//! export implements which Warrant operation, which faults attack it, and the
//! generation parameters), compiles the Warrant document named there, and
//! writes the `output` file.  `--check` compares bytes instead and fails when
//! the checked-in file has drifted from the Warrant document, the bindings or
//! the generator (VER04h section 1).
use std::{env, fs, path::PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{is_package_dir_name, parse_evidence_bindings};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{
    compile_rust_evidence_harness_with_tables, load_document_tables, EvidenceOptions, TableSet,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::vectors::{ensure_no_symlinks, read_fixture, FIXTURES_ROOT, MAX_TABLE_FILE_BYTES};

/// Every file this driver reads is read through the loader's capped, symlink-safe
/// reader: at most 64 MiB (a generated harness is a few MiB), no symlinked
/// component, a regular file.
const MAX_DRIVER_READ: u64 = 4 * MAX_TABLE_FILE_BYTES;

/// Generate (or, with `check`, compare) the harness of one package under the
/// repository root `root`.  `pub(crate)` only so that
/// `tests/gen_evidence_driver.rs` can drive this exact function on a scratch
/// repository; the example's own `main` is its only other caller.
pub(crate) fn generate_one(root: &std::path::Path, package: &str, check: bool) -> Result<(), Box<dyn std::error::Error>> {
    // The package name is joined into filesystem paths: refuse anything but a
    // plain directory name before it is used.
    if !is_package_dir_name(package) {
        return Err(format!("package `{package}` must match [A-Za-z0-9][A-Za-z0-9_-]*").into());
    }
    let bindings_path = root.join("code/packages/rust").join(package).join("evidence.bindings");
    let bindings_rel = format!("code/packages/rust/{package}/evidence.bindings");
    let bindings_text = read_fixture(root, &bindings_rel, MAX_DRIVER_READ)?;
    let b = parse_evidence_bindings(std::str::from_utf8(&bindings_text)?)
        .map_err(|e| format!("{}: {e}", bindings_path.display()))?;
    // spec and output are pinned to this package: the file cannot redirect the
    // generator to another document or make it write elsewhere.
    b.validate_paths(package)
        .map_err(|e| format!("{}: {e}", bindings_path.display()))?;
    let spec = read_fixture(root, &b.spec, MAX_DRIVER_READ)?;
    let doc = SourceDocument::admit(&spec, SourceLimits::default()).map_err(|e| format!("{e:?}"))?;
    let layout = capture_layout(&doc, LayoutLimits::default()).map_err(|e| format!("{e:?}"))?;
    let file = parse_document(&layout, &doc, ParserLimits::default()).map_err(|e| format!("{e:?}"))?;
    let checked = check_warrant(&file).map_err(|e| format!("{e:?}"))?;
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
        // The WX-A host lines (`struct`, `enum`, and `args=`/`ret=`/`errors=` on
        // an `op` line) are part of the bindings and must reach the generator.
        // This used to be `Default::default()`, which silently dropped them:
        // every host-composite operation then failed with W0701/W0704 here while
        // the in-test drivers, which pass them, succeeded
        // (tests/gen_evidence_driver.rs).  A bindings file with no host lines
        // parses to the default, so ct-compare and gf256 generate byte-identically.
        hosts: b.hosts.clone(),
    };
    // WX-E: verify every pinned table (digest before parse, size and row caps,
    // no symlinks, every row decoded) and let the harness embed its copy.  A
    // package's tables always live next to its harness (`validate_paths` pins
    // the output to `tests/generated/`).
    let tables = load_document_tables(&file, &root.join(FIXTURES_ROOT)).map_err(|e| format!("{e}"))?;
    let set = TableSet {
        dir: "tests/generated/tables",
        tables: &tables,
    };
    let text = compile_rust_evidence_harness_with_tables(&file, &checked, &b.manifest, &options, &set)
        .map_err(|e| format!("{e}"))?;
    let package_dir = root.join("code/packages/rust").join(package);
    for (rel, bytes) in set.files() {
        let copy = package_dir.join(&rel);
        if check {
            if read_fixture(&package_dir, &rel, MAX_DRIVER_READ).ok().as_deref() != Some(bytes) {
                return Err(format!(
                    "generated table copy drift: {rel} of {package} is stale; run gen_evidence {package}"
                )
                .into());
            }
        } else {
            let dir = copy.parent().ok_or("table copy has no directory")?;
            // Never write through a linked file or directory, at any level.
            ensure_no_symlinks(root, &format!("code/packages/rust/{package}/{rel}"))?;
            fs::create_dir_all(dir)?;
            fs::write(&copy, bytes)?;
            println!("wrote {}", copy.display());
        }
    }
    let target = root.join(&b.output);
    if check {
        if read_fixture(root, &b.output, MAX_DRIVER_READ)? != text.as_bytes() {
            return Err(format!(
                "generated evidence drift: {} is stale; run gen_evidence {package}",
                b.output
            )
            .into());
        }
        println!("Warrant evidence harness for {package} is current");
    } else {
        ensure_no_symlinks(root, &b.output)?;
        fs::write(&target, text.as_bytes())?;
        println!("wrote {}", target.display());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut check = false;
    let mut packages: Vec<String> = Vec::new();
    for a in env::args().skip(1) {
        if a == "--check" {
            check = true;
        } else if a.starts_with('-') {
            return Err("usage: gen_evidence [--check] [<package> ...]".into());
        } else {
            packages.push(a);
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .ok_or("missing repository root")?
        .to_path_buf();
    if packages.is_empty() {
        for entry in fs::read_dir(root.join("code/packages/rust"))? {
            let entry = entry?;
            if entry.path().join("evidence.bindings").is_file() {
                packages.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        packages.sort();
    }
    if packages.is_empty() {
        return Err("no package has an evidence.bindings file".into());
    }
    for p in &packages {
        generate_one(&root, p, check)?;
    }
    Ok(())
}
