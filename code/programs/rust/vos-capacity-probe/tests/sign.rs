//! `sign` (VOS-028c3c-2c, VOS-D120), the banner it writes, and the overall re-run budget of
//! VOS-D134.  Everything runs on the pure functions over a joined file built from the committed
//! rows, so nothing needs a git checkout, a clock or a Unix host.

mod common;

use std::path::PathBuf;
use std::time::Duration;
use vos_capacity_probe::cli::{dispatch, merge_results, Failure};
use vos_capacity_probe::results::{
    budget_spent, rerun_within, validate_classes, Recording, Results, QUICK_RERUN_BUDGET_S,
    TREE_NAMES,
};
use vos_capacity_probe::sign::{sign, SignRequest};
use vos_capacity_probe::table::render_block;

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

fn committed_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json")
}

fn committed_text() -> String {
    std::fs::read_to_string(committed_path()).unwrap()
}

fn trees() -> Vec<(String, String)> {
    TREE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{:040x}", i + 1)))
        .collect()
}

/// A joined record-mode recording of run 123 attempt 1 at `HEAD`, from the committed rows.
fn joined() -> Results {
    let mut r = common::complete_plan(Results::from_text(&committed_text()).unwrap());
    r.meta.merges.clear();
    r.meta.regeneration = None;
    r.meta.trees = trees();
    for (k, v) in r.meta.revisions.iter_mut() {
        if k == "repo" {
            *v = HEAD.to_string();
        }
    }
    r.meta.recording = Some(Recording {
        mode: "record".into(),
        run_id: Some("123".into()),
        run_attempt: Some("1".into()),
        shard: None,
        shards: Some(64),
        jobs: 2,
    });
    r
}

fn req() -> SignRequest {
    SignRequest {
        reviewer: "A. Reviewer".into(),
        host: "ci-host 1".into(),
        run_id: "123".into(),
        run_attempt: "1".into(),
        head_sha: HEAD.into(),
        expect_sha256: None,
    }
}

/// The commit `HEAD` holds exactly the recorded trees: the honest reviewer's case (VOS-D137).
fn at_head(rev: &str) -> Result<Vec<(String, String)>, String> {
    assert_eq!(rev, HEAD, "sign asks about the typed commit and no other");
    Ok(trees())
}

/// A commit that is not in the reviewer's checkout.
fn missing(rev: &str) -> Result<Vec<(String, String)>, String> {
    Err(format!(
        "commit {rev} is not in this checkout (run `git fetch origin {rev}` and sign again)"
    ))
}

/// For the merge tests, which still take an ancestry function (`merge` keeps that rule).
fn yes(_: &str) -> Result<bool, String> {
    Ok(true)
}

/// Sign with `r`; `--expect-sha256` is required, so a request that names none gets the digest of
/// `text` (the honest reviewer's case).  The refusal of an absent digest is tested through
/// `sign` itself.
fn do_sign(text: &str, r: &SignRequest) -> Result<Results, String> {
    let with = SignRequest {
        reviewer: r.reviewer.clone(),
        host: r.host.clone(),
        run_id: r.run_id.clone(),
        run_attempt: r.run_attempt.clone(),
        head_sha: r.head_sha.clone(),
        expect_sha256: r.expect_sha256.clone().or_else(|| Some(digest(text))),
    };
    sign(text, &with, &trees(), &at_head)
}

fn refused(text: &str, r: &SignRequest, needle: &str) {
    let e = do_sign(text, r).expect_err("must be refused");
    assert!(e.contains(needle), "wanted {needle:?} in {e:?}");
}

fn digest(text: &str) -> String {
    coding_adventures_sha256::sha256_hex(text.as_bytes())
}

// ---------------------------------------------------------------------------------------
// The committed file is untouched
// ---------------------------------------------------------------------------------------

#[test]
fn the_committed_file_still_round_trips() {
    let text = committed_text();
    let res = Results::from_canonical_text(&text).expect("canonical");
    assert_eq!(res.to_text(), text, "the committed bytes are unchanged");
    assert!(res.meta.regeneration.is_none());
    assert!(
        text.contains("\"regeneration\": null"),
        "an unsigned file still writes null"
    );
    assert!(validate_classes(&res).structural.is_empty());
}

// ---------------------------------------------------------------------------------------
// A good signature
// ---------------------------------------------------------------------------------------

#[test]
fn signing_leaves_every_row_byte_identical() {
    let input = joined().to_text();
    let signed = do_sign(&input, &req()).expect("signs");
    let g = signed.meta.regeneration.clone().expect("banner");
    assert_eq!(g.unsigned_sha256, digest(&input));
    assert_eq!(
        (g.reviewer.as_str(), g.host.as_str(), g.head_sha.as_str()),
        ("A. Reviewer", "ci-host 1", HEAD)
    );
    // The signed text differs from the input in exactly the banner's line.
    let out = signed.to_text();
    let (a, b): (Vec<&str>, Vec<&str>) = (input.lines().collect(), out.lines().collect());
    assert_eq!(a.len(), b.len(), "one line replaces one line");
    let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
    assert_eq!(diff.len(), 1, "{diff:?}");
    assert!(a[diff[0]].contains("\"regeneration\": null"));
    assert!(b[diff[0]].contains("\"unsigned_sha256\""));
    // Taking the banner out again gives the input, byte for byte, and the signed file is clean.
    let mut bare = Results::from_canonical_text(&out).expect("canonical");
    assert_eq!(bare.rows, joined().rows);
    bare.meta.regeneration = None;
    assert_eq!(bare.to_text(), input);
    assert!(validate_classes(&signed).structural.is_empty());
}

#[test]
fn the_expected_digest_is_accepted_when_it_matches() {
    let input = joined().to_text();
    let mut r = req();
    r.expect_sha256 = Some(digest(&input));
    assert!(do_sign(&input, &r).is_ok());
    r.expect_sha256 = Some(digest(&input).to_uppercase());
    assert!(do_sign(&input, &r).is_ok(), "hex case is not significant");
}

// ---------------------------------------------------------------------------------------
// One test per refusal
// ---------------------------------------------------------------------------------------

#[test]
fn an_absent_expected_digest_is_refused() {
    // The sign-off is tied to the joined file the join job printed a digest for: no digest, no
    // signature, even for an otherwise perfect file (VOS30 9.12.2 item 7).
    let input = joined().to_text();
    let e = sign(&input, &req(), &trees(), &at_head).unwrap_err();
    assert!(e.contains("--expect-sha256 is required"), "{e}");
}

#[test]
fn a_file_with_merges_is_not_signed() {
    let mut r = joined();
    r.meta.merges.push(vos_capacity_probe::results::Merge {
        rev: HEAD.into(),
        binary_sha256: "0".repeat(64),
        dirty_tree: false,
        experiment: "A1".into(),
        where_: String::new(),
        kind: None,
        trees: Vec::new(),
        rows: Vec::new(),
        at: "2026-11-23T00:00:00Z".into(),
    });
    refused(&r.to_text(), &req(), "records merges");
}

#[test]
fn a_mismatched_expected_digest_is_refused() {
    let input = joined().to_text();
    let mut r = req();
    r.expect_sha256 = Some("0".repeat(64));
    refused(&input, &r, "not the expected");
    r.expect_sha256 = Some("xyz".into());
    refused(&input, &r, "--expect-sha256 must be 64 hex digits");
}

#[test]
fn a_file_that_is_not_canonical_is_refused() {
    let input = joined()
        .to_text()
        .replace("\"schema\": 2", "\"schema\":  2");
    refused(&input, &req(), "sign refuses:");
}

#[test]
fn a_file_that_already_has_a_banner_is_refused() {
    let signed = do_sign(&joined().to_text(), &req()).unwrap().to_text();
    refused(&signed, &req(), "already carries a regeneration banner");
}

#[test]
fn a_local_run_and_a_non_record_or_non_joined_recording_are_refused() {
    let mut r = joined();
    r.meta.recording = None;
    refused(&r.to_text(), &req(), "no recording block");

    let mut r = joined();
    r.meta.recording.as_mut().unwrap().mode = "verify".into();
    refused(&r.to_text(), &req(), "only a `record` file is signed");

    let mut r = joined();
    let rec = r.meta.recording.as_mut().unwrap();
    rec.shards = None;
    rec.shard = Some((3, 64));
    refused(&r.to_text(), &req(), "not a joined recording");

    // A local run records no run id: the flags cannot equal it.
    let mut r = joined();
    let rec = r.meta.recording.as_mut().unwrap();
    rec.run_id = None;
    rec.run_attempt = None;
    refused(&r.to_text(), &req(), "--run-id 123 is not the recording's");
}

#[test]
fn a_run_id_or_attempt_that_is_not_the_recordings_is_refused() {
    let input = joined().to_text();
    let mut r = req();
    r.run_id = "124".into();
    refused(&input, &r, "--run-id 124 is not the recording's run id");
    let mut r = req();
    r.run_attempt = "2".into();
    refused(
        &input,
        &r,
        "--run-attempt 2 is not the recording's run attempt",
    );
    for bad in ["", "12a", &"1".repeat(21)] {
        let mut r = req();
        r.run_id = bad.into();
        refused(&input, &r, "--run-id must be 1 to 20 ASCII digits");
    }
}

#[test]
fn a_head_sha_that_is_not_the_recorded_revision_is_refused() {
    let input = joined().to_text();
    let mut r = req();
    r.head_sha = "f".repeat(40);
    refused(&input, &r, "is not the recorded repo revision");
    for bad in ["abc", &"g".repeat(40), &"a".repeat(41)] {
        let mut r = req();
        r.head_sha = bad.into();
        refused(&input, &r, "--head-sha must be a 40-hex revision");
    }
}

#[test]
fn a_head_sha_whose_trees_differ_from_the_recorded_ones_is_refused() {
    // VOS-D137: the check is on content (tree ids at the commit), not on ancestry.  Each of the
    // five paths in turn differs at the typed commit.
    let input = joined().to_text();
    let mut r = req();
    r.expect_sha256 = Some(digest(&input));
    for i in 0..TREE_NAMES.len() {
        let moved = move |_: &str| -> Result<Vec<(String, String)>, String> {
            let mut t = trees();
            t[i].1 = "f".repeat(40);
            Ok(t)
        };
        let e = sign(&input, &r, &trees(), &moved).unwrap_err();
        assert!(
            e.contains("differ from the recorded `trees`") && e.contains(TREE_NAMES[i]),
            "{e}"
        );
    }
    // A commit that has fewer trees than the file records differs too.
    let short = |_: &str| -> Result<Vec<(String, String)>, String> {
        Ok(trees().into_iter().take(4).collect())
    };
    let e = sign(&input, &r, &trees(), &short).unwrap_err();
    assert!(e.contains("differ from the recorded `trees`"), "{e}");
}

#[test]
fn a_head_sha_that_is_not_in_the_checkout_is_refused_with_the_fetch_hint() {
    let input = joined().to_text();
    let mut r = req();
    r.expect_sha256 = Some(digest(&input));
    let e = sign(&input, &r, &trees(), &missing).unwrap_err();
    assert!(e.starts_with("sign refuses: "), "{e}");
    assert!(e.contains(&format!("git fetch origin {HEAD}")), "{e}");
}

#[test]
fn a_head_sha_whose_trees_equal_the_recorded_ones_signs_whatever_its_ancestry() {
    // Nothing about HEAD or the history is asked: only the trees at the typed commit.
    let signed = do_sign(&joined().to_text(), &req()).expect("signs");
    assert_eq!(signed.meta.regeneration.unwrap().head_sha, HEAD);
}

#[test]
fn trees_at_reads_real_commits_and_refuses_missing_ones() {
    // The real function, on this very checkout (every git checkout has HEAD; no Unix needed).
    use vos_capacity_probe::meta::{head_trees, trees_at};
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();
    if let Ok(o) = head {
        if o.status.success() {
            let sha = String::from_utf8_lossy(&o.stdout).trim().to_string();
            assert_eq!(trees_at(&sha).unwrap(), head_trees());
        }
    }
    // Not 40 hex: refused before git is run.
    for bad in ["", "abc", &"g".repeat(40), &"a".repeat(41), "--version"] {
        assert!(trees_at(bad).unwrap_err().contains("40-hex"), "{bad}");
    }
    // 40 hex that no object store holds.
    let e = trees_at(&"0".repeat(40)).unwrap_err();
    assert!(
        e.contains(&format!("git fetch origin {}", "0".repeat(40))),
        "{e}"
    );
}

#[test]
fn a_file_that_is_structurally_invalid_is_refused() {
    let mut r = joined();
    r.meta.dirty_tree = true;
    refused(&r.to_text(), &req(), "structurally invalid");
}

#[test]
fn a_file_that_does_not_describe_the_tree_at_hand_is_refused() {
    let input = joined().to_text();
    // The working tree's ids differ from the recorded ones.
    let mut now = trees();
    now[1].1 = "f".repeat(40);
    let mut r = req();
    r.expect_sha256 = Some(digest(&input));
    let e = sign(&input, &r, &now, &at_head).unwrap_err();
    assert!(e.contains("check --full refuses"), "{e}");
    // A file that records no tree ids at all.
    let mut r = joined();
    r.meta.trees.clear();
    refused(&r.to_text(), &req(), "records no tree ids");
}

#[test]
fn a_file_that_does_not_cover_the_full_plan_is_refused() {
    let mut r = joined();
    let first = r.rows[0].clone();
    // Drop every row of the first knob: structurally fine (no half-seeded group), but a plan gap.
    r.rows.retain(|x| {
        !(x.experiment == first.experiment && x.mode == first.mode && x.knob == first.knob)
    });
    refused(&r.to_text(), &req(), "the full plan is not covered");
}

#[test]
fn a_reviewer_or_host_outside_the_charset_is_refused() {
    let input = joined().to_text();
    let long = "a".repeat(65);
    for bad in [
        "",
        " lead",
        "trail ",
        "a|b",
        "a`b",
        "a<b",
        "a>b",
        "double  space",
        "[Dr X](https://evil.example)",
        "![a](x)",
        "https://x",
        "a*b",
        "#x",
        "a_b~c",
        "a\nb",
        "a\tb",
        "a\u{7f}b",
        "caf\u{e9}",
        &long,
    ] {
        let mut r = req();
        r.reviewer = bad.into();
        refused(&input, &r, "--reviewer must be 1 to 64 characters");
        let mut r = req();
        r.host = bad.into();
        refused(&input, &r, "--host must be 1 to 64 characters");
    }
    let mut r = req();
    r.reviewer = "a".repeat(64);
    assert!(do_sign(&input, &r).is_ok(), "64 characters is allowed");
    // Everything in the whitelist is allowed.
    let mut r = req();
    r.reviewer = "Dr. A_b-C, O'Neil @ 9".into();
    r.host = "ci.host-1_a".into();
    assert!(do_sign(&input, &r).is_ok());
}

// ---------------------------------------------------------------------------------------
// validate and the banner
// ---------------------------------------------------------------------------------------

#[test]
fn validate_catches_a_row_edit_after_signing() {
    let signed = do_sign(&joined().to_text(), &req()).unwrap();
    assert!(validate_classes(&signed).structural.is_empty());
    // Edit one row's recorded verdict after signing, and read the file back as `check` would.
    let mut edited = signed.clone();
    edited.rows[0].conflicts += 1;
    let back = Results::from_canonical_text(&edited.to_text()).expect("canonical");
    let found = validate_classes(&back).structural;
    assert!(
        found
            .iter()
            .any(|m| m.contains("edited after it was signed")),
        "{found:?}"
    );
    // So does the status of `validate` on the file, through the command.
    let dir = std::env::temp_dir().join(format!("sign-edit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("edited.json");
    std::fs::write(&p, edited.to_text()).unwrap();
    let args = vec!["validate".to_string(), p.to_string_lossy().into_owned()];
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let r = dispatch(&args, &mut o, &mut e);
    assert!(
        matches!(r, Err(Failure::Usage(_))),
        "structural is exit 2: {r:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn validate_checks_the_banner_against_the_file() {
    let signed = do_sign(&joined().to_text(), &req()).unwrap();
    let structural = |f: &dyn Fn(&mut Results)| {
        let mut x = signed.clone();
        f(&mut x);
        validate_classes(&x).structural.join("\n")
    };
    assert!(
        structural(&|x| x.meta.regeneration.as_mut().unwrap().reviewer = "a|b".into())
            .contains("reviewer is not 1 to 64")
    );
    assert!(
        structural(&|x| x.meta.regeneration.as_mut().unwrap().run_id = "9".into())
            .contains("not a joined record-mode recording of the banner's run")
    );
    assert!(
        structural(&|x| x.meta.regeneration.as_mut().unwrap().head_sha = "z".into())
            .contains("head_sha is not a 40-hex")
    );
    assert!(
        structural(&|x| x.meta.regeneration.as_mut().unwrap().head_sha = "a".repeat(40))
            .contains("head_sha is not the recorded repo revision")
    );
    assert!(
        structural(&|x| x.meta.regeneration.as_mut().unwrap().unsigned_sha256 = "q".into())
            .contains("unsigned_sha256 is not 64 hex")
    );
    // A banner on a file that records merges is structural.
    assert!(
        structural(&|x| x.meta.merges.push(vos_capacity_probe::results::Merge {
            rev: HEAD.into(),
            binary_sha256: "0".repeat(64),
            dirty_tree: false,
            experiment: "A1".into(),
            where_: String::new(),
            kind: None,
            trees: Vec::new(),
            rows: Vec::new(),
            at: "2026-11-23T00:00:00Z".into(),
        }))
        .contains("a file with merges cannot carry a banner")
    );
    // A banner on a file that is not a joined record (a shard here) is structural.
    assert!(structural(&|x| {
        let rec = x.meta.recording.as_mut().unwrap();
        rec.shards = None;
        rec.shard = Some((0, 64));
    })
    .contains("not a joined record-mode recording"));
    assert!(structural(&|x| x.meta.recording = None).contains("not a joined record-mode"));
}

#[test]
fn a_banner_with_a_missing_or_unknown_key_is_unreadable() {
    let text = do_sign(&joined().to_text(), &req()).unwrap().to_text();
    let no_host = text.replace("\"host\":\"ci-host 1\",", "");
    assert!(Results::from_text(&no_host)
        .unwrap_err()
        .contains("no host"));
    let extra = text.replace("\"reviewer\":", "\"extra\":\"x\",\"reviewer\":");
    assert!(Results::from_text(&extra)
        .unwrap_err()
        .contains("unknown key"));
    let non_string = text.replace("\"run_id\":\"123\"", "\"run_id\":123");
    assert!(Results::from_text(&non_string)
        .unwrap_err()
        .contains("not a string"));
}

// ---------------------------------------------------------------------------------------
// merge and table
// ---------------------------------------------------------------------------------------

#[test]
fn merge_clears_a_banner() {
    let signed = do_sign(&joined().to_text(), &req()).unwrap();
    assert!(signed.meta.regeneration.is_some());
    // Refresh the A1 rows from a fresh run (here: the same rows) into the signed base.
    let mut fresh = joined();
    fresh.meta.recording = None;
    fresh.rows.retain(|r| r.experiment == "A1");
    let merged = merge_results(
        &signed,
        &fresh,
        "A1",
        None,
        true,
        "2026-11-23T00:00:00Z",
        &yes,
    )
    .expect("merges");
    assert!(
        merged.meta.regeneration.is_none(),
        "a merged file is a new recording and needs a new signature"
    );
    assert_eq!(merged.meta.merges.len(), 1);
    assert!(validate_classes(&merged).structural.is_empty());
}

#[test]
fn table_renders_a_signed_banner() {
    let signed = do_sign(&joined().to_text(), &req()).unwrap();
    let block = render_block(&signed);
    assert!(
        block.contains(
            "Full tier regenerated by A. Reviewer on ci-host 1. Recording: run 123 attempt 1 at \
             revision 0123456789ab."
        ),
        "{}",
        block.lines().find(|l| l.contains("PROXY")).unwrap_or("")
    );
    assert!(!block.contains("not independently regenerated"));
    let unsigned = render_block(&joined());
    assert!(unsigned.contains("Full tier not independently regenerated."));
}

// ---------------------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------------------

fn run(args: &[&str]) -> Result<(), Failure> {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    dispatch(&a, &mut Vec::new(), &mut Vec::new())
}

#[test]
fn the_command_refuses_a_bad_command_line_and_writes_nothing() {
    let dir = std::env::temp_dir().join(format!("sign-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("in.json");
    std::fs::write(&input, joined().to_text()).unwrap();
    let out = dir.join("out.json");
    let (i, o) = (
        input.to_string_lossy().into_owned(),
        out.to_string_lossy().into_owned(),
    );
    let all = |extra: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = [
            "sign",
            &i,
            "--reviewer",
            "R",
            "--host",
            "H",
            "--run-id",
            "123",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        v.extend(["--run-attempt", "1", "--head-sha", HEAD].map(String::from));
        v.extend(["--expect-sha256".to_string(), digest(&joined().to_text())]);
        v.extend(extra.iter().map(|s| s.to_string()));
        v
    };
    let go = |a: Vec<String>| dispatch(&a, &mut Vec::new(), &mut Vec::new());
    // The digest is required on the command line too.
    let no_digest: Vec<String> = all(&["--out", &o])
        .into_iter()
        .take_while(|a| a != "--expect-sha256")
        .chain(["--out".to_string(), o.clone()])
        .collect();
    assert!(matches!(
        go(no_digest),
        Err(Failure::Usage(m)) if m.contains("sign needs --expect-sha256")
    ));
    // A missing --out, an unknown flag, a repeated flag, an --out that is the input.
    assert!(matches!(go(all(&[])), Err(Failure::Usage(m)) if m.contains("needs --out")));
    assert!(matches!(
        go(all(&["--out", &o, "--bogus", "1"])),
        Err(Failure::Usage(m)) if m.contains("unknown flag")
    ));
    assert!(matches!(
        go(all(&["--out", &o, "--reviewer", "S"])),
        Err(Failure::Usage(m)) if m.contains("more than once")
    ));
    assert!(matches!(
        go(all(&["--out", &i])),
        Err(Failure::Usage(m)) if m.contains("is the input file")
    ));
    // An --out that exists is never overwritten.
    std::fs::write(&out, "keep").unwrap();
    assert!(matches!(
        go(all(&["--out", &o])),
        Err(Failure::Usage(m)) if m.contains("already exists")
    ));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep");
    std::fs::remove_file(&out).unwrap();
    // A refusal from the pure function (the revision is fake, git cannot place it, or the working
    // tree's ids differ) leaves no file.
    assert!(matches!(go(all(&["--out", &o])), Err(Failure::Usage(_))));
    assert!(!out.exists());
    // No file and two files.
    assert!(run(&["sign", "--out", &o]).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_command_names_the_fetch_for_a_commit_that_is_not_in_the_checkout() {
    // VOS-D137, through the real command: the file records the working tree's own ids (so that
    // `full_refusals` is clean) and a typed revision no object store holds, so the refusal comes
    // from `meta::trees_at`.  Skipped when git cannot read the working tree (no checkout) or the
    // five crates have uncommitted changes (a `+uncommitted` id is not a valid recorded id).
    let now = vos_capacity_probe::meta::working_trees();
    if now.is_empty() || now != vos_capacity_probe::meta::head_trees() {
        return;
    }
    let mut r = joined();
    r.meta.trees = now;
    let text = r.to_text();
    let dir = std::env::temp_dir().join(format!("sign-fetch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (input, out) = (dir.join("in.json"), dir.join("out.json"));
    std::fs::write(&input, &text).unwrap();
    let a: Vec<String> = [
        "sign",
        input.to_str().unwrap(),
        "--reviewer",
        "R",
        "--host",
        "H",
        "--run-id",
        "123",
        "--run-attempt",
        "1",
        "--head-sha",
        HEAD,
        "--expect-sha256",
        &digest(&text),
        "--out",
        out.to_str().unwrap(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let e = dispatch(&a, &mut Vec::new(), &mut Vec::new());
    assert!(
        matches!(&e, Err(Failure::Usage(m)) if m.contains(&format!("git fetch origin {HEAD}"))),
        "{e:?}"
    );
    assert!(!out.exists());
    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------------------
// VOS-D134: the overall re-run budget
// ---------------------------------------------------------------------------------------

#[test]
fn the_budget_rule_is_pure() {
    let s = Duration::from_secs;
    assert!(!budget_spent(s(10_000), None), "no budget is unbounded");
    assert!(!budget_spent(s(0), Some(s(1))));
    assert!(!budget_spent(Duration::from_millis(999), Some(s(1))));
    assert!(budget_spent(s(1), Some(s(1))), "spent at the budget");
    assert!(budget_spent(s(2), Some(s(1))));
    assert!(
        budget_spent(s(0), Some(s(0))),
        "a zero budget is spent at once"
    );
}

#[test]
fn the_overall_budget_is_generous_and_at_least_one_long_row() {
    // The measured honest cost of the 85 quick rows recorded before VOS-028c3c commit A is about 10 s; the longest single row wall
    // is 1,800 s.  The budget is above both by a wide margin and below an hour.
    const { assert!(QUICK_RERUN_BUDGET_S >= vos_capacity_probe::exp::B5_ROW_WALL_S) };
    const { assert!(QUICK_RERUN_BUDGET_S <= 3_600) };
}

#[test]
fn a_spent_budget_stops_the_rerun_with_one_finding_and_runs_no_row() {
    let res = Results::from_text(&committed_text()).unwrap();
    let quick = vos_capacity_probe::results::quick_specs(&res).len();
    assert!(quick > 0);
    // Zero is spent before the first row starts, so nothing runs and the test is instant.
    let found = rerun_within(&res, 4, Duration::ZERO);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("exceeded its overall budget of 0 s")
            && found[0].contains(&format!("0 of {quick} rows")),
        "{}",
        found[0]
    );
}

// ---------------------------------------------------------------------------------------
// table refuses a banner that is not well formed
// ---------------------------------------------------------------------------------------

#[test]
fn table_refuses_a_hand_edited_banner() {
    let signed = do_sign(&joined().to_text(), &req()).unwrap();
    let dir = std::env::temp_dir().join(format!("sign-table-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let spec = dir.join("spec.md");
    let original = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS30-proof-route-and-capacity.md"),
    )
    .unwrap();
    std::fs::write(&spec, &original).unwrap();
    let go = |res: &std::path::Path, mode: &str| {
        let a: Vec<String> = [
            "table",
            mode,
            "--results",
            &res.to_string_lossy(),
            "--spec",
            &spec.to_string_lossy(),
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        dispatch(&a, &mut Vec::new(), &mut Vec::new())
    };
    for bad in ["a\nb", "a|b", "a<b", "[x](https://evil.example)"] {
        let mut x = signed.clone();
        x.meta.regeneration.as_mut().unwrap().reviewer = bad.into();
        let p = dir.join("bad.json");
        std::fs::write(&p, x.to_text()).unwrap();
        for mode in ["--check", "--write"] {
            let r = go(&p, mode);
            assert!(
                matches!(&r, Err(Failure::Usage(m)) if m.contains("table refuses")),
                "{bad:?} {mode}: {r:?}"
            );
        }
    }
    assert_eq!(
        std::fs::read_to_string(&spec).unwrap(),
        original,
        "a refused table writes nothing"
    );
    std::fs::remove_dir_all(&dir).ok();
}
