//! VER-P047: source-equivalence is a refusal-only report anchor, never a pass.

mod common;

use std::fs;

use coding_adventures_ca_assurance_report::evidence::{derive_claims, Evidence};
use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::manifest::source_tree_framing;
use common::Sandbox;

const FIRST_ID: &str = "warrant/fixed_point_q16/source-equivalence/q16_checked_add";

fn fixed_point(tag: &str) -> Sandbox {
    Sandbox::for_package(tag, "fixed-point")
}

fn edit_proof_entry(sb: &Sandbox, id: &str, from: &str, to: &str) {
    let path = sb.pkg.join("evidence/proof-manifest.json");
    let text = fs::read_to_string(&path).unwrap();
    let marker = format!("\"id\": \"{id}\"");
    let id_at = text
        .find(&marker)
        .unwrap_or_else(|| panic!("missing entry {id}"));
    let start = text[..id_at]
        .rfind("\n    {")
        .map(|at| at + 1)
        .unwrap_or_else(|| panic!("missing object start for {id}"));
    let end = text[id_at..]
        .find("\n    }")
        .map(|at| id_at + at)
        .unwrap_or_else(|| panic!("missing object end for {id}"));
    let rel = text[start..end]
        .find(from)
        .unwrap_or_else(|| panic!("missing {from:?} in {id}"));
    let at = start + rel;
    let mut changed = text.clone();
    changed.replace_range(at..at + from.len(), to);
    fs::write(path, changed).unwrap();
}

fn replace_manifest_pin(sb: &Sandbox, field: &str, old: &str, new: &str) {
    let dir = sb.pkg.join("evidence");
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let text = fs::read_to_string(&path).unwrap();
            if text.contains(field) && text.contains(old) {
                fs::write(&path, text.replace(old, new)).unwrap();
            }
        }
    }
}

fn update_source_pins(sb: &Sandbox, old: &str) {
    let framed = source_tree_framing(&sb.pkg.join("src")).unwrap();
    let new = sha256_hex(&framed);
    replace_manifest_pin(sb, "source_sha256", old, &new);
}

fn update_spec_pins(sb: &Sandbox, old: &str) {
    let path = sb.root.join("code/specs/fixed-point.warrant");
    let bytes = fs::read(&path).unwrap();
    replace_manifest_pin(sb, "spec_sha256", old, &sha256_hex(&bytes));
}

fn add_step_bound(sb: &Sandbox) {
    let path = sb.root.join("code/specs/fixed-point.warrant");
    let old = fs::read_to_string(&path).unwrap();
    let old_hash = sha256_hex(old.as_bytes());
    let changed = old.replace(
        "  assume platform:\n",
        "  bound q16_checked_add:\n    steps <= 1024\n\n  assume platform:\n",
    );
    assert_ne!(changed, old, "step-bound insertion point is present");
    fs::write(path, changed).unwrap();
    update_spec_pins(sb, &old_hash);
}

#[test]
fn actual_fixed_point_frontend_refusals_are_claim_anchors_but_not_proofs() {
    let sb = fixed_point("p047-positive");
    let ev = Evidence::load(&sb.pkg).expect("actual frontend refusals reproduce");
    let source: Vec<_> = ev
        .proofs
        .iter()
        .filter(|p| p.kind == "source-equivalence")
        .collect();
    assert_eq!(source.len(), 10);
    assert!(source
        .iter()
        .all(|p| p.verdict == "not-live" && p.label == "not-live"));
    assert!(source
        .iter()
        .all(|p| p.certificates.is_empty() && p.max_len == 0));
    assert!(source
        .iter()
        .all(|p| p.note.contains("actual") || p.note.contains("isolated export attempt")));

    let claims = derive_claims(&ev).expect("fallback anchors every package operation");
    assert_eq!(claims.len(), 10);
    assert!(
        claims.iter().all(|claim| claim.proof.is_none()),
        "refusal never becomes a proof block"
    );
    assert_eq!(ev.rungs().e5, "not-live", "fallback cannot make E5 pass");
    let ctx = ev.audit_context("rust/fixed-point");
    assert!(ctx.evidence.ct_proof_ids.is_empty());
    assert!(ctx.evidence.step_bound_ids.is_empty());
}

#[test]
fn fallback_rejects_wrong_export_id_and_non_refusal_status() {
    let sb = fixed_point("p047-shape");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"function\": \"checked_add\"",
        "\"function\": \"checked_sub\"",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("does not match bound export"));

    let sb = fixed_point("p047-id");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        FIRST_ID,
        "warrant/fixed_point_q16/source-equivalence/wrong",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("fallback id must be"));

    let sb = fixed_point("p047-status");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"label\": \"not-live\"",
        "\"label\": \"unchecked\"",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("must carry the label `not-live`"));

    let sb = fixed_point("p047-unchecked");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"label\": \"not-live\"",
        "\"label\": \"unchecked\"",
    );
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"verdict\": \"not-live\"",
        "\"verdict\": \"unchecked\"",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("must be not-live"));

    let sb = fixed_point("p047-pass");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"label\": \"not-live\"",
        "\"label\": \"pass\"",
    );
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"verdict\": \"not-live\"",
        "\"verdict\": \"pass\"",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("must be not-live"));

    let sb = fixed_point("p047-unknown-kind");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"kind\": \"source-equivalence\"",
        "\"kind\": \"negative-control\"",
    );
    let ev = Evidence::load(&sb.pkg).expect("ordinary evidence loader can parse the unrelated row");
    assert!(derive_claims(&ev)
        .unwrap_err()
        .0
        .contains("has no proof obligations"));
}

#[test]
fn fallback_rejects_certificates_counters_and_missing_reviewed_opt_in() {
    let sb = fixed_point("p047-cert");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"certificates\": []",
        "\"certificates\": [{\"id\": \"forged\", \"kind\": \"miter\"}]",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("certificates must be an empty array"));

    let sb = fixed_point("p047-counter");
    edit_proof_entry(
        &sb,
        FIRST_ID,
        "\"max_len\": 0,",
        "\"max_len\": 0,\n      \"runs\": 0,",
    );
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("unknown source-equivalence field `runs`"));

    let sb = fixed_point("p047-opt-in");
    let path = sb.pkg.join("evidence.bindings");
    let bindings = fs::read_to_string(&path).unwrap();
    let option = " proofs_not_live=source-frontend-refuses-const-item-or-q16-custom-type";
    let start = bindings.find("op q16_checked_add ").unwrap();
    assert!(bindings[start..].contains(option));
    let mut changed = bindings.clone();
    let at = start + bindings[start..].find(option).unwrap();
    changed.replace_range(at..at + option.len(), "");
    fs::write(path, changed).unwrap();
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("not declared `proofs_not_live=<reason>`"));
}

#[test]
fn fallback_cannot_replace_cost_proof_obligations_or_unlabelled_inputs() {
    let sb = fixed_point("p047-step-bound");
    add_step_bound(&sb);
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("no steps bound"));

    let sb = fixed_point("p047-secret");
    let path = sb.root.join("code/specs/fixed-point.warrant");
    let old = fs::read_to_string(&path).unwrap();
    let old_hash = sha256_hex(old.as_bytes());
    let changed = old.replacen(
        "  labels q16_checked_add:\n    a public",
        "  labels q16_checked_add:\n    a secret",
        1,
    );
    assert_ne!(changed, old, "public label is present");
    fs::write(path, changed).unwrap();
    update_spec_pins(&sb, &old_hash);
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("every parameter explicitly public"));
}

#[test]
fn stale_source_pin_and_newly_accepted_export_fail_closed() {
    let sb = fixed_point("p047-stale-pin");
    let path = sb.pkg.join("evidence/proof-manifest.json");
    let text = fs::read_to_string(&path).unwrap();
    let marker = "\"source_sha256\": \"";
    let at = text.find(marker).unwrap() + marker.len();
    let end = text[at..].find('"').unwrap() + at;
    let mut changed = text;
    changed.replace_range(at..end, &"0".repeat(64));
    fs::write(path, changed).unwrap();
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("stale Rust source tree"));

    let sb = fixed_point("p047-accepted");
    let old_source_pin = sb.src_digest();
    let path = sb.pkg.join("src/lib.rs");
    let original = fs::read_to_string(&path).unwrap();
    let start = original.find("pub fn checked_add(").unwrap();
    let open = original[start..].find('{').unwrap() + start;
    let mut depth = 0usize;
    let mut end = None;
    for (offset, ch) in original[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + offset + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.unwrap();
    let replacement = "pub fn checked_add(a: i32, b: i32) -> i32 { a + b }";
    let mut changed = original.clone();
    changed.replace_range(start..end, replacement);
    fs::write(path, changed).unwrap();
    update_source_pins(&sb, &old_source_pin);
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("frontend now accepts direct crate-root export `checked_add`"));
}

#[test]
fn only_a_unique_direct_crate_root_export_is_supported() {
    let sb = fixed_point("p047-private-child");
    let old_pin = sb.src_digest();
    let path = sb.pkg.join("src/lib.rs");
    let original = fs::read_to_string(&path).unwrap();
    let changed = original.replacen("pub fn checked_add(", "fn checked_add(", 1)
        + "\nmod private_child { pub fn checked_add(a: Q16, b: Q16) -> Result<Q16, Q16Error> { a } }\n";
    assert_ne!(changed, original);
    fs::write(path, changed).unwrap();
    update_source_pins(&sb, &old_pin);
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("no unconditional direct public export `checked_add`"));

    let sb = fixed_point("p047-duplicate-root");
    let old_pin = sb.src_digest();
    let path = sb.pkg.join("src/lib.rs");
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str("\npub fn checked_add(a: Q16, b: Q16) -> Result<Q16, Q16Error> { a }\n");
    fs::write(path, source).unwrap();
    update_source_pins(&sb, &old_pin);
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("ambiguous direct public export `checked_add`"));
}

#[test]
fn cfg_macro_and_custom_cargo_target_candidates_fail_closed() {
    for (tag, prefix) in [
        ("p047-cfg", "#[cfg(any())]\n"),
        ("p047-cfg-attr", "#[cfg_attr(unix, allow(dead_code))]\n"),
    ] {
        let sb = fixed_point(tag);
        let old_pin = sb.src_digest();
        let path = sb.pkg.join("src/lib.rs");
        let original = fs::read_to_string(&path).unwrap();
        let changed = original.replacen(
            "pub fn checked_add(",
            &format!("{prefix}pub fn checked_add("),
            1,
        );
        fs::write(path, changed).unwrap();
        update_source_pins(&sb, &old_pin);
        assert!(Evidence::load(&sb.pkg)
            .unwrap_err()
            .0
            .contains("attribute is unsupported"));
    }

    for (tag, cargo) in [
        ("p047-lib-table", "[\"lib\"]\npath = \"elsewhere.rs\"\n"),
        ("p047-autolib", "[package]\nautolib = false\n"),
        ("p047-dotted-lib", "lib.path = \"elsewhere.rs\"\n"),
        ("p047-package-autolib", "package.autolib = false\n"),
    ] {
        let sb = fixed_point(tag);
        fs::write(sb.pkg.join("Cargo.toml"), cargo).unwrap();
        assert!(Evidence::load(&sb.pkg).unwrap_err().0.contains("Cargo"));
    }
}

#[test]
fn root_macro_inputs_and_symlinked_src_are_not_export_authority() {
    let sb = fixed_point("p047-macro-input");
    let old_pin = sb.src_digest();
    let path = sb.pkg.join("src/lib.rs");
    let mut source = fs::read_to_string(&path).unwrap();
    source.insert_str(0, "generated!((pub fn checked_add() {}));\n");
    fs::write(path, source).unwrap();
    update_source_pins(&sb, &old_pin);
    assert!(Evidence::load(&sb.pkg)
        .unwrap_err()
        .0
        .contains("macro invocation is unsupported"));

    #[cfg(unix)]
    {
        let sb = fixed_point("p047-src-root-symlink");
        let src = sb.pkg.join("src");
        let saved = sb.pkg.join("src.real");
        fs::rename(&src, &saved).unwrap();
        std::os::unix::fs::symlink(&saved, &src).unwrap();
        assert!(Evidence::load(&sb.pkg)
            .unwrap_err()
            .0
            .contains("source directory"));
        fs::remove_file(&src).unwrap();
        fs::rename(saved, src).unwrap();
    }
}
