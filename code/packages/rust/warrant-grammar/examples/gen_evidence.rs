//! Regenerate (or check) the Warrant-derived evidence harness of ct-compare.
//!
//! `cargo run -p coding_adventures_warrant_grammar --example gen_evidence`
//! writes `ct-compare/tests/generated/warrant_evidence.rs` from
//! `code/specs/ct-compare.warrant`; `-- --check` compares bytes and fails when
//! the checked-in file has drifted from the Warrant document or the generator
//! (VER04h section 1).  The binding table below is the one hand-written input:
//! it names which public export implements which Warrant operation.
use std::collections::BTreeMap;
use std::{env, fs, path::PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::binding::{BindingManifest, ErrorNormalization, OpBinding};
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::evidence::{compile_rust_evidence_harness, EvidenceOptions};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn binding(export: &str, widths: &[&str], fresh: bool, norm: Option<&str>, ctl: &[&str]) -> OpBinding {
    OpBinding {
        export: export.into(),
        static_params: widths.iter().map(|s| s.to_string()).collect(),
        fresh,
        error_normalization: norm.map(|c| ErrorNormalization {
            kind: "panic_to_error".into(),
            error_case: c.into(),
        }),
        negative_controls: ctl.iter().map(|s| s.to_string()).collect(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let check = match args.as_slice() {
        [] => false,
        [a] if a == "--check" => true,
        _ => return Err("usage: gen_evidence [--check]".into()),
    };
    let code = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .ok_or("missing code directory")?
        .to_path_buf();
    let spec_path = code.join("specs/ct-compare.warrant");
    let spec = fs::read(&spec_path)?;
    let doc = SourceDocument::admit(&spec, SourceLimits::default()).map_err(|e| format!("{e:?}"))?;
    let layout = capture_layout(&doc, LayoutLimits::default()).map_err(|e| format!("{e:?}"))?;
    let file = parse_document(&layout, &doc, ParserLimits::default()).map_err(|e| format!("{e:?}"))?;
    let checked = check_warrant(&file).map_err(|e| format!("{e:?}"))?;

    let mut bindings = BTreeMap::new();
    bindings.insert("eq".into(), binding("ct_eq", &[], false, None, &["invert", "ignore-last-byte"]));
    bindings.insert("eq_fixed".into(), binding("ct_eq_fixed", &["N"], false, None, &["invert", "ignore-last-byte"]));
    bindings.insert(
        "select_bytes".into(),
        binding("ct_select_bytes", &[], true, Some("CT_SELECT_LENGTH_MISMATCH"), &["swap-choice", "missing-error"]),
    );
    bindings.insert("eq_u64".into(), binding("ct_eq_u64", &[], false, None, &["invert", "ignore-high-bit"]));
    let manifest = BindingManifest {
        package: "coding_adventures_ct_compare".into(),
        revision: "0.1.0".into(),
        bindings,
    };
    let options = EvidenceOptions {
        package_crate: "coding_adventures_ct_compare".into(),
        spec_path: "code/specs/ct-compare.warrant".into(),
        spec_sha256: sha256_hex(&spec),
        seed: 0x5741_5252_414e_5421,
        property_cases: 4096,
        fuzz_iterations: 5000,
        negctl_fuzz_iterations: 1000,
    };
    let text = compile_rust_evidence_harness(&file, &checked, &manifest, &options)
        .map_err(|e| format!("{e}"))?;
    let target = code.join("packages/rust/ct-compare/tests/generated/warrant_evidence.rs");
    if check {
        if fs::read(&target)? != text.as_bytes() {
            return Err("generated evidence drift: tests/generated/warrant_evidence.rs is stale; run gen_evidence".into());
        }
        println!("Warrant evidence harness is current");
    } else {
        fs::write(&target, text.as_bytes())?;
        println!("wrote {}", target.display());
    }
    Ok(())
}
