//! WX-F slice 1: binding-file `driver` records parse and schema-validate.
//!
//! Covers VER04k section 3.4 rules 1-3 and 7 plus every schema negative of
//! the section 3.11 test plan. Typing against the document (W0711),
//! obligations, evaluation and sabotage arrive in later slices; generation
//! refuses driven operations explicitly until then.

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::driver::driver_named_methods;

const BASE: &str = "package p\nrevision 1\nspec code/specs/p.warrant\noutput code/packages/rust/p/tests/generated/e.rs\nseed 1\nproperty_cases 1\nfuzz_iterations 1\nnegctl_fuzz_iterations 1\n";

const SHA1_OP: &str = "op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step\n";
const SHA1_DRIVER: &str = "driver digest_chunks state=coding_adventures_sha1::Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish lengths=55,56,63,64,65,119,120,127,128,129\n";

fn parse_ok(body: &str) -> coding_adventures_warrant_grammar::binding::EvidenceBindingFile {
    parse_evidence_bindings(&format!("{BASE}{body}")).expect("must parse")
}

fn parse_err(body: &str) -> String {
    parse_evidence_bindings(&format!("{BASE}{body}")).expect_err("must fail")
}

#[test]
fn parses_the_sha1_spec_example() {
    let file = parse_ok(&format!("{SHA1_OP}{SHA1_DRIVER}"));
    let binding = &file.manifest.bindings["digest_chunks"];
    assert!(binding.export.is_none());
    let driver = binding.driver.as_ref().expect("driver record attached");
    assert_eq!(driver.state, "coding_adventures_sha1::Digest");
    assert_eq!(driver.new.func, "new");
    assert!(driver.new.args.is_empty());
    assert_eq!(driver.drive, "chunks");
    assert_eq!(driver.step.method, "update");
    assert_eq!(driver.step.args.len(), 1);
    assert!(driver.finish.is_some());
    assert_eq!(driver.lengths.as_ref().unwrap().len(), 10);
}

#[test]
fn parses_the_range_coder_and_fallible_examples() {
    let body = "op encode_bits driver faults=drop-last-chunk\n\
        driver encode_bits state=range_coder::BoolEncoder new=new() drive=bits step=write_bit(mut,elem.0:bool,elem.1:u8) finish=finish(own)->bytes result=finish\n\
        op digest_try driver faults=drop-last-chunk\n\
        driver digest_try state=coding_adventures_sha256::Sha256Hasher new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=coding_adventures_sha256::Sha256Error/LengthOverflow=TOO_LONG\n";
    let file = parse_ok(body);
    assert_eq!(file.manifest.bindings.len(), 2);
    let enc = file.manifest.bindings["encode_bits"]
        .driver
        .as_ref()
        .unwrap();
    assert_eq!(enc.step.args.len(), 2);
    let sha = file.manifest.bindings["digest_try"]
        .driver
        .as_ref()
        .unwrap();
    assert!(sha.errors.is_some());
}

#[test]
fn rejects_export_and_driver_together_and_neither() {
    let both = "op o export=f driver faults=a\n";
    assert!(parse_err(both).contains("W0501"));
    let neither = "op o faults=a\n";
    assert!(parse_err(neither).contains("W0501"));
}

#[test]
fn rejects_driver_line_problems() {
    // Driver line for an op without the flag.
    let no_flag =
        "op o export=f faults=a\ndriver o state=s::T new=new() drive=x step=s(mut) result=finish\n";
    assert!(parse_err(no_flag).contains("W0501"));
    // Driver line for an unknown op.
    let unknown = "op o export=f faults=a\ndriver ghost state=s::T new=new() drive=x step=s(mut) result=finish\n";
    assert!(parse_err(unknown).contains("W0501"));
    // Two driver lines.
    let twice = format!("{SHA1_OP}{SHA1_DRIVER}{SHA1_DRIVER}");
    assert!(parse_err(&twice).contains("W0501"));
    // Flagged op with no driver line.
    assert!(parse_err(SHA1_OP).contains("W0501"));
}

#[test]
fn rejects_bad_paths_identifiers_and_lengths() {
    let template = |driver: &str| format!("{SHA1_OP}driver digest_chunks {driver}\n");
    // Path segment with `;`, `{`, `..`.
    for bad in ["s;x::T", "s::{T", "s::..::T"] {
        let err = parse_err(&template(&format!(
            "state={bad} new=new() drive=x step=s(mut) result=finish"
        )));
        assert!(err.contains("W0501"), "{bad}");
    }
    // 9-segment path.
    let long_path = (0..9)
        .map(|i| format!("s{i}"))
        .collect::<Vec<_>>()
        .join("::");
    assert!(parse_err(&template(&format!(
        "state={long_path} new=new() drive=x step=s(mut) result=finish"
    )))
    .contains("W0501"));
    // 129-byte identifier.
    let long_ident = "a".repeat(129);
    assert!(parse_err(&template(&format!(
        "state=s::T new={long_ident}() drive=x step=s(mut) result=finish"
    )))
    .contains("W0501"));
    // 128-byte identifier is accepted.
    let ok_ident = "a".repeat(128);
    parse_ok(&template(&format!(
        "state=s::T new={ok_ident}() drive=x step=s(mut) result=finish"
    )));
    // Closed vocabularies.
    assert!(parse_err(&template(
        "state=s::T new=new() drive=x step=s(mutable) result=finish"
    ))
    .contains("W0501"));
    assert!(parse_err(&template(
        "state=s::T new=new() drive=x step=s(mut) result=stream"
    ))
    .contains("W0501"));
    // Lengths: unsorted, 33 values, value of 4097.
    assert!(parse_err(&template(
        "state=s::T new=new() drive=x step=s(mut) result=finish lengths=2,1"
    ))
    .contains("W0501"));
    let many = (1..=33)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert!(parse_err(&template(&format!(
        "state=s::T new=new() drive=x step=s(mut) result=finish lengths={many}"
    )))
    .contains("W0501"));
    assert!(parse_err(&template(
        "state=s::T new=new() drive=x step=s(mut) result=finish lengths=4097"
    ))
    .contains("W0501"));
    // Repeated and unknown keys.
    assert!(parse_err(&template(
        "state=s::T state=t::U new=new() drive=x step=s(mut) result=finish"
    ))
    .contains("W0501"));
    assert!(parse_err(&template(
        "state=s::T new=new() drive=x step=s(mut) result=finish frobnicate=1"
    ))
    .contains("W0501"));
    // Missing required keys.
    for missing in ["state", "new", "drive", "step", "result"] {
        let mut words = vec![
            "state=s::T",
            "new=new()",
            "drive=x",
            "step=s(mut)",
            "result=finish",
        ];
        words.retain(|w| !w.starts_with(missing));
        assert!(
            parse_err(&template(&words.join(" "))).contains("W0501"),
            "{missing}"
        );
    }
}

#[test]
fn rejects_uncapped_drive_param_and_error_paths() {
    let template = |driver: &str| format!("{SHA1_OP}driver digest_chunks {driver}\n");
    // 129-byte drive parameter name.
    let long_drive = "d".repeat(129);
    assert!(parse_err(&template(&format!(
        "state=s::T new=new() drive={long_drive} step=s(mut) result=finish"
    )))
    .contains("W0501"));
    // 129-byte param.NAME source.
    let long_param = "p".repeat(129);
    assert!(parse_err(&template(&format!(
        "state=s::T new=new() drive=x step=s(mut,param.{long_param}:u8) result=finish"
    )))
    .contains("W0501"));
    // errors= path: 9 segments and a 129-byte segment.
    let long_path = (0..9).map(|i| format!("e{i}")).collect::<Vec<_>>().join("::");
    assert!(parse_err(&template(&format!(
        "state=s::T new=new() drive=x step=s(mut) result=finish errors={long_path}/V=C"
    )))
    .contains("W0501"));
    let long_seg = "e".repeat(129);
    assert!(parse_err(&template(&format!(
        "state=s::T new=new() drive=x step=s(mut) result=finish errors=crate::{long_seg}/V=C"
    )))
    .contains("W0501"));
    // A capped errors path still parses.
    parse_ok(&template(
        "state=s::T new=new() drive=x step=s(mut) result=finish errors=crate::E/V=C",
    ));
}

#[test]
fn named_methods_lists_constructor_step_finish_and_ctx() {
    // VER04k 3.4 rule 6: each counts as bound for the export analyzer.
    let file = parse_ok(&format!("{SHA1_OP}{SHA1_DRIVER}"));
    let driver = file.manifest.bindings["digest_chunks"]
        .driver
        .as_ref()
        .expect("record");
    assert_eq!(driver_named_methods(driver), vec!["new", "update", "sum1"]);
    // A context function joins first; a collect-mode record has no finish.
    let file = parse_ok(&format!(
        "{SHA1_OP}driver digest_chunks state=S ctx=cx(param.seed:u8) new=new() drive=chunks step=update(mut,elem:bytes)->u8 result=collect\n"
    ));
    let driver = file.manifest.bindings["digest_chunks"]
        .driver
        .as_ref()
        .expect("record");
    assert_eq!(driver_named_methods(driver), vec!["cx", "new", "update"]);
}
