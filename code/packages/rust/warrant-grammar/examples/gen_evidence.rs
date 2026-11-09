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
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn generate_one(root: &std::path::Path, package: &str, check: bool) -> Result<(), Box<dyn std::error::Error>> {
    // The package name is joined into filesystem paths: refuse anything but a
    // plain directory name before it is used.
    if !is_package_dir_name(package) {
        return Err(format!("package `{package}` must match [A-Za-z0-9][A-Za-z0-9_-]*").into());
    }
    let bindings_path = root.join("code/packages/rust").join(package).join("evidence.bindings");
    let b = parse_evidence_bindings(&fs::read_to_string(&bindings_path)?)
        .map_err(|e| format!("{}: {e}", bindings_path.display()))?;
    // spec and output are pinned to this package: the file cannot redirect the
    // generator to another document or make it write elsewhere.
    b.validate_paths(package)
        .map_err(|e| format!("{}: {e}", bindings_path.display()))?;
    let spec = fs::read(root.join(&b.spec))?;
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
    };
    let text = compile_rust_evidence_harness(&file, &checked, &b.manifest, &options)
        .map_err(|e| format!("{e}"))?;
    let target = root.join(&b.output);
    if check {
        if fs::read(&target)? != text.as_bytes() {
            return Err(format!(
                "generated evidence drift: {} is stale; run gen_evidence {package}",
                b.output
            )
            .into());
        }
        println!("Warrant evidence harness for {package} is current");
    } else {
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
