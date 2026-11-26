//! VOS-D142 (VOS-028c3c-3b-1c): the 8.6 re-plan label is machine-generated and machine-checked.
//! `decide --replan` prints `decide_replan`'s answer, `table --write` renders it into the replan
//! block of VOS30 8.6, and `table --check` fails on a hand-edited block or label.  Every test that
//! writes works on a temporary copy of the spec; the committed spec is only ever read.

use std::path::PathBuf;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::results::{decide_replan, Results};
use vos_capacity_probe::table::{
    check_spec, extract_replan_block, render_replan_block, replan_lines, REPLAN_BEGIN, REPLAN_END,
};

fn specs(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs")
        .join(name)
}

fn results_path() -> PathBuf {
    specs("data/vos30-capacity-results.json")
}

fn committed() -> Results {
    Results::from_canonical_text(&std::fs::read_to_string(results_path()).unwrap()).unwrap()
}

fn spec_text() -> String {
    std::fs::read_to_string(specs("VOS30-proof-route-and-capacity.md")).unwrap()
}

fn run(args: &[&str]) -> (String, String, Result<(), Failure>) {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = dispatch(&a, &mut out, &mut err);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        r,
    )
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-replan-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The byte range of the replan block, markers and trailing newline included.
fn block_range(spec: &str) -> (usize, usize) {
    let a = spec.find(REPLAN_BEGIN).unwrap();
    let e = spec.find(REPLAN_END).unwrap() + REPLAN_END.len();
    (a, e)
}

#[test]
fn decide_replan_prints_the_recorded_decision() {
    let rp = results_path();
    let (out, _, r) = run(&["decide", "--replan", "--results", rp.to_str().unwrap()]);
    assert!(r.is_ok());
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "row R3: Compiled lane, reduced");
    assert_eq!(lines[1], "cut density K = 2");
    assert!(
        lines[2].starts_with("memory: shipped T (D5 split=index route-green at T = 64 and T = 256"),
        "{}",
        lines[2]
    );
    // The conditions are the B5 reds the spec's 8.6 names: 1,000 instructions and fuse = 4.
    let because = &lines[3..];
    assert_eq!(because.len(), 12, "{because:?}");
    assert!(because.iter().all(|l| l.starts_with("because: B5 at ")));
    assert!(because
        .iter()
        .any(|l| l.contains("1,000 instructions, div = 25%, fuse = 2")));
    assert!(because
        .iter()
        .any(|l| l.contains("200 instructions, div = 100%, fuse = 4")));
    // It is exactly the library's answer, deterministically, and `decide` without the flag is the
    // 8.3 decision as before.
    assert_eq!(
        out,
        replan_lines(&decide_replan(&committed().rows)).join("\n") + "\n"
    );
    let (plain, _, _) = run(&["decide", "--results", rp.to_str().unwrap()]);
    assert!(plain.starts_with("row "), "{plain}");
    assert!(!plain.contains("cut density"));
}

#[test]
fn the_committed_spec_passes_check_and_its_block_is_what_the_rows_give() {
    let (spec, res) = (spec_text(), committed());
    assert_eq!(check_spec(&spec, Some(&res)), Vec::<String>::new());
    assert!(spec.contains("**Re-plan decision (VOS-028c): R3, compiled lane, reduced**"));
    assert!(spec.contains("### 8.6 "));
    let (_, block) = extract_replan_block(&spec).unwrap().unwrap();
    assert_eq!(block, render_replan_block(&res));
    assert!(block.starts_with(REPLAN_BEGIN) && block.trim_end().ends_with(REPLAN_END));
}

#[test]
fn table_check_fails_when_the_marked_block_is_edited() {
    let d = scratch("block");
    let spec = spec_text();
    let sp = d.join("spec.md");
    let rp = results_path();
    let go = |text: &str| {
        std::fs::write(&sp, text).unwrap();
        run(&[
            "table",
            "--check",
            "--results",
            rp.to_str().unwrap(),
            "--spec",
            sp.to_str().unwrap(),
        ])
    };
    // Control: the unedited copy passes through the real command.
    assert!(go(&spec).2.is_ok());
    // A hand-edited line and an added line each fail as drift.
    for (name, edited) in [
        (
            "edited",
            spec.replace("* cut density K = 2\n", "* cut density K = 4\n"),
        ),
        (
            "added",
            spec.replace(REPLAN_END, &format!("* because: invented\n{REPLAN_END}")),
        ),
    ] {
        assert_ne!(edited, spec, "{name}: the edit did not apply");
        let (_, err, r) = go(&edited);
        assert!(matches!(r, Err(Failure::Violations(_))), "{name}");
        assert!(err.contains("re-plan block"), "{name}: {err}");
        assert!(err.contains("drift"), "{name}: {err}");
    }
    // A deleted block fails while the label stays.
    let (a, e) = block_range(&spec);
    let gone = format!("{}{}", &spec[..a], &spec[e..]);
    let (_, err, r) = go(&gone);
    assert!(matches!(r, Err(Failure::Violations(_))));
    assert!(err.contains("no generated re-plan block"), "{err}");
    // A single marker is a broken pair, not a skip.
    let (_, err, r) = go(&spec.replace(REPLAN_END, ""));
    assert!(matches!(r, Err(Failure::Violations(_))));
    assert!(err.contains("replan markers"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn table_check_fails_when_the_hand_written_label_is_edited() {
    let (spec, res) = (spec_text(), committed());
    let label = "R3, compiled lane, reduced";
    assert!(spec.contains(label));
    for (what, edited, needle) in [
        (
            "row",
            spec.replace(label, "R4, compiled lane"),
            "re-plan label",
        ),
        (
            "title",
            spec.replace(label, "R3, compiled lane"),
            "re-plan label",
        ),
        (
            "k",
            spec.replace("**cut density K = 2**", "**cut density K = 4**"),
            "cut density K = 2",
        ),
        (
            "memory",
            spec.replace("**at their shipped T**", "**at T at most 16**"),
            "memory verdict",
        ),
    ] {
        assert_ne!(edited, spec, "{what}: the edit did not apply");
        let bad = check_spec(&edited, Some(&res));
        assert!(bad.iter().any(|b| b.contains(needle)), "{what}: {bad:?}");
    }
    // A label with no block is its own failure.
    let (a, e) = block_range(&spec);
    let no_block = format!("{}{}", &spec[..a], &spec[e..]);
    assert!(check_spec(&no_block, Some(&res))
        .iter()
        .any(|b| b.contains("no generated re-plan block")));
    let neither = no_block.replace("**Re-plan decision (VOS-028c):", "**Re-plan sentence:");
    // Fail closed: with no block AND no label, `check` still fails, once for each.
    let bad = check_spec(&neither, Some(&res));
    assert!(
        bad.iter().any(|b| b.contains("no generated re-plan block")),
        "{bad:?}"
    );
    assert!(bad.iter().any(|b| b.contains("exactly once")), "{bad:?}");
    // A block outside 8.6 is refused.
    let moved = format!("{}\n{}", no_block, render_replan_block(&res));
    assert!(check_spec(&moved, Some(&res))
        .iter()
        .any(|b| b.contains("not inside section 8.6")));
}

#[test]
fn table_write_regenerates_only_the_marked_block() {
    let d = scratch("write");
    let spec = spec_text();
    let (a, e) = block_range(&spec);
    // Corrupt the block in a copy, write, and the copy equals the committed spec again.
    let sp = d.join("spec.md");
    std::fs::write(
        &sp,
        spec.replace("* cut density K = 2\n", "* cut density K = 9\n"),
    )
    .unwrap();
    let rp = results_path();
    let write = || {
        run(&[
            "table",
            "--write",
            "--results",
            rp.to_str().unwrap(),
            "--spec",
            sp.to_str().unwrap(),
        ])
    };
    assert!(write().2.is_ok());
    let after = std::fs::read_to_string(&sp).unwrap();
    assert_eq!(after, spec);
    // The hand-written prose around the block is untouched by a write.
    assert_eq!(&after[..a], &spec[..a]);
    assert_eq!(&after[e..], &spec[e..]);
    // A broken marker pair is a usage error and writes nothing.
    std::fs::write(&sp, spec.replace(REPLAN_END, "")).unwrap();
    let before = std::fs::read_to_string(&sp).unwrap();
    assert!(matches!(write().2, Err(Failure::Usage(_))));
    assert_eq!(std::fs::read_to_string(&sp).unwrap(), before);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_different_decision_changes_the_block_and_the_label_check() {
    // Make B5 at 200 instructions, 25%, fuse = 2 not green: the rows now give R2, so the
    // committed spec's block and label (both R3) fail against them.
    let mut res = committed();
    for r in res.rows.iter_mut().filter(|r| {
        r.experiment == "B5"
            && r.knob.key().contains("\"length\":200")
            && r.knob.key().contains("\"fuse\":2")
            && r.knob.key().contains("0.25")
    }) {
        r.green = false;
        r.result = "unknown".into();
    }
    assert_eq!(decide_replan(&res.rows).row, 2);
    let bad = check_spec(&spec_text(), Some(&res));
    assert!(bad.iter().any(|b| b.contains("drift")), "{bad:?}");
    assert!(
        bad.iter()
            .any(|b| b.contains("R2, owned-compile lane mandatory")),
        "{bad:?}"
    );
}

#[test]
fn a_missing_section_is_a_finding_and_so_is_every_strictness_violation() {
    let (spec, res) = (spec_text(), committed());
    let finds = |text: &str, needle: &str| {
        let bad = check_spec(text, Some(&res));
        assert!(bad.iter().any(|b| b.contains(needle)), "{needle}: {bad:?}");
    };
    // The heading renamed: 8.6 cannot be found, so the label and block are not in it.
    let renamed = spec.replace("### 8.6 The re-plan", "### Eight point six The re-plan");
    assert_ne!(renamed, spec);
    finds(&renamed, "no section 8.6");
    // K = 25 is not K = 2.
    let k25 = spec.replace("**cut density K = 2**", "**cut density K = 25**");
    assert_ne!(k25, spec);
    finds(&k25, "cut density K = 2");
    // A second label is refused, wherever it is.
    let label = "**Re-plan decision (VOS-028c): R3, compiled lane, reduced**";
    assert!(spec.contains(label));
    let twice = spec.replace(
        "\n<!-- vos30-replan:begin -->",
        &format!("\n{label}, cut density K = 2, at their shipped T\n\n<!-- vos30-replan:begin -->"),
    );
    assert_ne!(twice, spec);
    finds(&twice, "exactly once");
    // A marker that shares its line with other text is refused (and not silently skipped).
    let shared = spec.replace(REPLAN_END, &format!("text {REPLAN_END}"));
    assert_ne!(shared, spec);
    finds(&shared, "shares its line");
    let shared = spec.replace(REPLAN_BEGIN, &format!("{REPLAN_BEGIN} trailing"));
    assert_ne!(shared, spec);
    finds(&shared, "shares its line");
    // Whitespace around an alone marker is not "sharing a line" (the padded block is drift, since
    // the comparison is byte for byte, but the marker is accepted).
    let padded = spec.replace(REPLAN_BEGIN, &format!("  {REPLAN_BEGIN}  "));
    let bad = check_spec(&padded, Some(&res));
    assert!(
        !bad.iter().any(|b| b.contains("shares its line")),
        "{bad:?}"
    );
    assert!(bad.iter().any(|b| b.contains("drift")), "{bad:?}");
}

#[test]
fn a_crlf_copy_of_the_spec_passes_when_unedited_and_fails_when_edited() {
    let (spec, res) = (spec_text(), committed());
    let crlf = spec.replace("\r\n", "\n").replace('\n', "\r\n");
    assert!(crlf.contains("\r\n"));
    assert_eq!(check_spec(&crlf, Some(&res)), Vec::<String>::new());
    let edited = crlf.replace("* cut density K = 2\r\n", "* cut density K = 4\r\n");
    assert_ne!(edited, crlf);
    assert!(check_spec(&edited, Some(&res))
        .iter()
        .any(|b| b.contains("drift")));
    let label = crlf.replace("R3, compiled lane, reduced", "R4, compiled lane");
    assert_ne!(label, crlf);
    assert!(check_spec(&label, Some(&res))
        .iter()
        .any(|b| b.contains("re-plan label")));
}
