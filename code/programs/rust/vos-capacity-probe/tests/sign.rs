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
/// The commit the verify run ran at (VOS-D141: it may differ from the recording's).
const VHEAD: &str = "fedcba9876543210fedcba9876543210fedcba98";

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
        verifies_sha256: None,
    });
    r
}

/// The working tree after a shared crate moved: every input id differs from the recorded one.
fn moved_trees() -> Vec<(String, String)> {
    TREE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{:040x}", i + 1000)))
        .collect()
}

/// The joined verify file of run 456 that reproduced `rec` (the recorded file) at the working
/// tree `at`: the same rows, mode `verify`, 64 shards at `--jobs 2`, naming `rec` by its unsigned
/// digest (VOS-D141).
fn verify_file(rec: &Results, at: &[(String, String)]) -> Results {
    let mut v = rec.clone();
    v.meta.regeneration = None;
    v.meta.merges.clear();
    v.meta.trees = at.to_vec();
    for (k, x) in v.meta.revisions.iter_mut() {
        if k == "repo" {
            *x = VHEAD.to_string();
        }
    }
    v.meta.recording = Some(Recording {
        mode: "verify".into(),
        run_id: Some("456".into()),
        run_attempt: Some("1".into()),
        shard: None,
        shards: Some(64),
        jobs: 2,
        verifies_sha256: Some(rec.unsigned_sha256()),
    });
    v
}

fn req() -> SignRequest {
    SignRequest {
        reviewer: "A. Reviewer".into(),
        host: "ci-host 1".into(),
        run_id: "123".into(),
        run_attempt: "1".into(),
        head_sha: HEAD.into(),
        expect_sha256: None,
        verify_text: None,
        expect_verify_sha256: None,
    }
}

/// The commit `HEAD` holds exactly the recorded trees: the honest reviewer's case (VOS-D137).
fn at_head(rev: &str) -> Result<Vec<(String, String)>, String> {
    assert!(
        rev == HEAD || rev == VHEAD,
        "sign asks about the typed commit and the verify's commit and no other"
    );
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
    sign(text, &complete(text, r), &trees(), &at_head)
}

/// `r` with every digest and the verify file filled in as the honest reviewer's would be: the
/// digest of `text`, and the clean verify file of the unmodified `joined()` recording at the
/// recorded tree (the working tree of `do_sign`).  A field `r` already sets is kept.
fn complete(text: &str, r: &SignRequest) -> SignRequest {
    let j = verify_file(&joined(), &trees()).to_text();
    SignRequest {
        reviewer: r.reviewer.clone(),
        host: r.host.clone(),
        run_id: r.run_id.clone(),
        run_attempt: r.run_attempt.clone(),
        head_sha: r.head_sha.clone(),
        expect_sha256: r.expect_sha256.clone().or_else(|| Some(digest(text))),
        expect_verify_sha256: r.expect_verify_sha256.clone().or_else(|| Some(digest(&j))),
        verify_text: r.verify_text.clone().or(Some(j)),
    }
}

/// Sign `joined()` with the verify file `j` (digest filled in) at the working tree `now`; the tree
/// ids at the typed commit are the recorded ones.
fn sign_with(j: &Results, now: &[(String, String)]) -> Result<Results, String> {
    let input = joined().to_text();
    let jt = j.to_text();
    let r = SignRequest {
        expect_sha256: Some(digest(&input)),
        verify_text: Some(jt.clone()),
        expect_verify_sha256: Some(digest(&jt)),
        ..req()
    };
    // The recording's commit holds the recorded trees; the verify's commit holds the verify's.
    let jt_trees = j.meta.trees.clone();
    let at = move |rev: &str| -> Result<Vec<(String, String)>, String> {
        Ok(if rev == VHEAD {
            jt_trees.clone()
        } else {
            trees()
        })
    };
    sign(&input, &r, now, &at)
}

/// `sign_with` on the clean verify file at the recorded tree, after `edit` changed it.
fn refused_with(edit: &dyn Fn(&mut Results), needle: &str) {
    let mut j = verify_file(&joined(), &trees());
    edit(&mut j);
    let e = sign_with(&j, &trees()).expect_err("must be refused");
    assert!(e.starts_with("sign refuses: "), "{e}");
    assert!(e.contains(needle), "wanted {needle:?} in {e:?}");
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
    let r = complete(&input, &req());
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
    let r = complete(&input, &req());
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
fn a_file_that_does_not_describe_the_tree_at_hand_is_signable_only_with_a_verify_at_it() {
    // VOS-D141: the working tree's ids differ from the recorded ones.  That is no longer a refusal
    // by itself: the verify file decides.  Here the verify ran at the RECORDED tree, so it does not
    // vouch for the working tree, and `sign` names the first input that differs.
    let input = joined().to_text();
    let mut now = trees();
    now[1].1 = "f".repeat(40);
    let r = complete(&input, &req());
    let e = sign(&input, &r, &now, &at_head).unwrap_err();
    assert!(
        e.contains("ran at a different tree than the working tree") && e.contains(TREE_NAMES[1]),
        "{e}"
    );
    // A file that records no tree ids at all is still refused, whatever the verify says.
    let mut r = joined();
    r.meta.trees.clear();
    refused(&r.to_text(), &req(), "records no tree ids");
    // So is a checkout with an uncommitted input: a verify runs at a committed tree.
    let mut dirty = trees();
    dirty[2].1.push_str("+uncommitted");
    let r = complete(&input, &req());
    let e = sign(&input, &r, &dirty, &at_head).unwrap_err();
    assert!(e.contains("uncommitted changes"), "{e}");
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
    // The verify anchor is held to the same shapes (VOS-028c3c-3b-1b).
    for (edit, want) in [
        (
            (&|x: &mut Results| x.meta.regeneration.as_mut().unwrap().verified.run_id = "".into())
                as &dyn Fn(&mut Results),
            "verified run_id is not 1 to 20 ASCII digits",
        ),
        (
            &|x| x.meta.regeneration.as_mut().unwrap().verified.run_attempt = "1a".into(),
            "verified run_attempt is not 1 to 20 ASCII digits",
        ),
        (
            &|x| x.meta.regeneration.as_mut().unwrap().verified.head_sha = "abc".into(),
            "verified head_sha is not a 40-hex revision",
        ),
        (
            &|x| x.meta.regeneration.as_mut().unwrap().verified.verify_sha256 = "q".into(),
            "verified verify_sha256 is not 64 hex digits",
        ),
        (
            &|x| x.meta.regeneration.as_mut().unwrap().verified.trees.clear(),
            "verified trees: it records no tree ids",
        ),
        (
            &|x| {
                x.meta.regeneration.as_mut().unwrap().verified.trees[0]
                    .1
                    .push_str("+uncommitted")
            },
            "marked +uncommitted",
        ),
        (
            &|x| x.meta.regeneration.as_mut().unwrap().verified.trees[1].0 = "other".into(),
            "do not name exactly the recorded inputs",
        ),
    ] {
        assert!(structural(&edit).contains(want), "wanted {want:?}");
    }
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
    // `verified` is REQUIRED (no file was signed before it existed), strict, and typed.
    let start = text.find(",\"verified\":{").expect("verified is written");
    let end = start + text[start..].find("}}").expect("end of verified") + 2;
    let without = format!("{}{}", &text[..start], &text[end..]);
    assert!(Results::from_text(&without)
        .unwrap_err()
        .contains("regeneration has no verified"));
    let extra_v = text.replace("\"verified\":{", "\"verified\":{\"extra\":\"x\",");
    assert!(Results::from_text(&extra_v)
        .unwrap_err()
        .contains("verified has an unknown key"));
    let no_digest = text.replace("\"verify_sha256\":", "\"verify_sha257\":");
    assert!(Results::from_text(&no_digest).is_err());
    let int_trees = text.replace(
        "\"run_attempt\":\"1\",\"head_sha\":\"fedcba",
        "\"run_attempt\":1,\"head_sha\":\"fedcba",
    );
    assert!(Results::from_text(&int_trees)
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
    // Both anchors: the recording run and the verify run (VOS-028c3c-3b-1b).
    let digest16 = &digest(&verify_file(&joined(), &trees()).to_text())[..16];
    assert!(
        block.contains(&format!(
            "Recording: run 123 attempt 1 at revision 0123456789ab. Verified: run 456 attempt 1 \
             at revision fedcba987654, verify file SHA-256 {digest16}."
        )),
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
    let vfile = dir.join("verify.json");
    let vtext = verify_file(&joined(), &trees()).to_text();
    std::fs::write(&vfile, &vtext).unwrap();
    let vpath = vfile.to_string_lossy().into_owned();
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
        v.extend(["--verify-file".to_string(), vpath.clone()]);
        v.extend(["--expect-verify-sha256".to_string(), digest(&vtext)]);
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
    // Both verify flags are required too (VOS-028c3c-3b-1b): a usage error before anything is read.
    let strip = |flag: &str| -> Vec<String> {
        let a = all(&["--out", &o]);
        let at = a.iter().position(|x| x == flag).unwrap();
        a.iter()
            .enumerate()
            .filter(|(k, _)| *k != at && *k != at + 1)
            .map(|(_, x)| x.clone())
            .collect()
    };
    assert!(matches!(
        go(strip("--verify-file")),
        Err(Failure::Usage(m)) if m.contains("sign needs --verify-file")
    ));
    assert!(matches!(
        go(strip("--expect-verify-sha256")),
        Err(Failure::Usage(m)) if m.contains("sign needs --expect-verify-sha256")
    ));
    // A wrong verify digest is refused by the pure function, and writes nothing.
    let mut wrong = all(&["--out", &o]);
    let at = wrong
        .iter()
        .position(|x| x == "--expect-verify-sha256")
        .unwrap();
    wrong[at + 1] = "0".repeat(64);
    assert!(matches!(
        go(wrong),
        Err(Failure::Usage(m)) if m.contains("the verify file's SHA-256 is")
    ));
    assert!(!out.exists());
    // A verify file that does not exist is an unreadable-file usage error.
    let mut gone = all(&["--out", &o]);
    let at = gone.iter().position(|x| x == "--verify-file").unwrap();
    gone[at + 1] = dir.join("no-such.json").to_string_lossy().into_owned();
    assert!(matches!(go(gone), Err(Failure::Usage(_))));
    assert!(!out.exists());
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
    r.meta.trees = now.clone();
    let text = r.to_text();
    let vtext = verify_file(&r, &now).to_text();
    let dir = std::env::temp_dir().join(format!("sign-fetch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (input, out) = (dir.join("in.json"), dir.join("out.json"));
    let vfile = dir.join("verify.json");
    std::fs::write(&input, &text).unwrap();
    std::fs::write(&vfile, &vtext).unwrap();
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
        "--verify-file",
        vfile.to_str().unwrap(),
        "--expect-verify-sha256",
        &digest(&vtext),
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
// VOS-028c3c-3b-1b (VOS-D141): the verify evidence
// ---------------------------------------------------------------------------------------

#[test]
fn a_recording_at_another_tree_is_signed_on_a_clean_verify_at_the_working_tree() {
    // The case the whole amendment exists for: a shared crate moved since the recording, so the
    // working tree's ids differ from the recorded ones, and the verify file (run 456 at VHEAD)
    // ran at the working tree and reproduced every row.
    let now = moved_trees();
    assert_ne!(now, trees());
    let j = verify_file(&joined(), &now);
    let signed = sign_with(&j, &now).expect("signs");
    let g = signed.meta.regeneration.clone().expect("banner");
    let jt = j.to_text();
    assert_eq!(
        g.run_id, "123",
        "the recording's run is still the first anchor"
    );
    assert_eq!(g.verified.run_id, "456");
    assert_eq!(g.verified.run_attempt, "1");
    assert_eq!(g.verified.head_sha, VHEAD);
    assert_eq!(g.verified.verify_sha256, digest(&jt));
    assert_eq!(g.verified.trees, now, "the tree the verify ran at");
    // The file's own recorded trees never change: they record where it was recorded.
    assert_eq!(signed.meta.trees, trees());
    // The digest of the recording without its banner is the same before and after signing, and
    // the `verified` field does not move it.
    assert_eq!(signed.unsigned_sha256(), digest(&joined().to_text()));
    assert_eq!(g.unsigned_sha256, signed.unsigned_sha256());
    // It reads back, validates, and the banner is on one line with `verified` nested in it.
    let text = signed.to_text();
    let back = Results::from_canonical_text(&text).expect("canonical");
    assert_eq!(back, signed);
    assert!(
        validate_classes(&back).structural.is_empty(),
        "{:?}",
        validate_classes(&back).structural
    );
    let banner = text
        .lines()
        .find(|l| l.contains("\"regeneration\": {"))
        .expect("banner line");
    for want in [
        "\"verified\":{\"run_id\":\"456\",\"run_attempt\":\"1\"",
        &format!("\"head_sha\":\"{VHEAD}\""),
        &format!("\"verify_sha256\":\"{}\"", digest(&jt)),
        "\"trees\":{",
    ] {
        assert!(banner.contains(want), "{want}: {banner}");
    }
    // Only the banner line differs from the unsigned file.
    let input = joined().to_text();
    let diff = input
        .lines()
        .zip(text.lines())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(diff, 1);
}

#[test]
fn the_verify_files_digest_is_required_well_formed_and_equal() {
    let input = joined().to_text();
    let j = verify_file(&joined(), &trees());
    let jt = j.to_text();
    let base = SignRequest {
        expect_sha256: Some(digest(&input)),
        verify_text: Some(jt.clone()),
        expect_verify_sha256: Some(digest(&jt)),
        ..req()
    };
    let go = |r: &SignRequest| sign(&input, r, &trees(), &at_head);
    assert!(go(&base).is_ok());
    let mut upper = base.clone();
    upper.expect_verify_sha256 = Some(digest(&jt).to_uppercase());
    assert!(go(&upper).is_ok(), "hex case is not significant");
    // The banner records the digest as computed (lower case), whatever case was typed.
    assert_eq!(
        go(&upper)
            .unwrap()
            .meta
            .regeneration
            .unwrap()
            .verified
            .verify_sha256,
        digest(&jt)
    );
    let mut none = base.clone();
    none.verify_text = None;
    assert!(go(&none).unwrap_err().contains("--verify-file is required"));
    let mut none = base.clone();
    none.expect_verify_sha256 = None;
    assert!(go(&none)
        .unwrap_err()
        .contains("--expect-verify-sha256 is required"));
    let mut bad = base.clone();
    bad.expect_verify_sha256 = Some("xyz".into());
    assert!(go(&bad)
        .unwrap_err()
        .contains("--expect-verify-sha256 must be 64 hex digits"));
    let mut wrong = base.clone();
    wrong.expect_verify_sha256 = Some("0".repeat(64));
    let e = go(&wrong).unwrap_err();
    assert!(
        e.contains("the verify file's SHA-256 is") && e.contains("not the expected"),
        "{e}"
    );
    // The digest is of the bytes as read: one more byte at the end is a different file.
    let mut tail = base.clone();
    tail.verify_text = Some(format!("{jt}\n"));
    assert!(go(&tail)
        .unwrap_err()
        .contains("the verify file's SHA-256 is"));
    // A digest that matches a file that is not a results file is refused as such.
    let junk = "not json";
    let mut junk_req = base.clone();
    junk_req.verify_text = Some(junk.into());
    junk_req.expect_verify_sha256 = Some(digest(junk));
    assert!(go(&junk_req)
        .unwrap_err()
        .contains("sign refuses: --verify-file:"));
}

#[test]
fn a_verify_file_that_is_not_a_joined_verify_file_is_refused() {
    refused_with(
        &|j| j.meta.recording.as_mut().unwrap().mode = "record".into(),
        "verifies_sha256 is only for mode verify",
    );
    // (`verifies_sha256` is only for verify mode, so a record-mode file cannot carry it: the
    // parse refuses first; with the field cleared the core's own refusal is what speaks.)
    refused_with(
        &|j| {
            let r = j.meta.recording.as_mut().unwrap();
            r.mode = "record".into();
            r.verifies_sha256 = None;
        },
        "is not a verify file",
    );
    refused_with(&|j| j.meta.recording = None, "is not a verify file");
    refused_with(
        &|j| {
            let r = j.meta.recording.as_mut().unwrap();
            r.shards = None;
            r.shard = Some((3, 64));
        },
        "one verify shard, not a joined file",
    );
}

#[test]
fn a_verify_that_fanned_out_differently_is_refused() {
    refused_with(
        &|j| j.meta.recording.as_mut().unwrap().shards = Some(32),
        "--verify-file joins Some(32) shards but the recording joins Some(64)",
    );
    refused_with(
        &|j| j.meta.recording.as_mut().unwrap().jobs = 4,
        "--verify-file ran at --jobs 4 but the recording ran at --jobs 2",
    );
}

#[test]
fn a_verify_file_with_a_structural_finding_or_merges_is_refused() {
    refused_with(
        &|j| j.meta.dirty_tree = true,
        "--verify-file has 1 structural finding(s)",
    );
    refused_with(&|j| j.rows.clear(), "structural finding(s)");
    refused_with(
        &|j| {
            j.meta.merges.push(vos_capacity_probe::results::Merge {
                rev: VHEAD.into(),
                binary_sha256: "0".repeat(64),
                dirty_tree: false,
                experiment: "A1".into(),
                where_: String::new(),
                kind: None,
                trees: trees(),
                rows: Vec::new(),
                at: "2026-11-24T00:00:00Z".into(),
            })
        },
        "--verify-file",
    );
}

#[test]
fn a_verify_at_an_uncommitted_or_legacy_tree_is_refused() {
    refused_with(
        &|j| j.meta.trees[3].1.push_str("+uncommitted"),
        "--verify-file",
    );
    refused_with(
        &|j| {
            j.meta.trees = vos_capacity_probe::results::LEGACY_TREE_NAMES
                .iter()
                .map(|n| (n.to_string(), "a".repeat(40)))
                .collect()
        },
        "--verify-file",
    );
    refused_with(&|j| j.meta.trees.clear(), "a file records no tree ids");
}

#[test]
fn a_verify_at_another_tree_than_the_working_tree_is_refused() {
    // A clean verify at `moved_trees`, but the reviewer's checkout is at the recorded tree (and
    // the other way round): the verify does not vouch for the tree at hand.
    let j = verify_file(&joined(), &moved_trees());
    let e = sign_with(&j, &trees()).unwrap_err();
    assert!(
        e.contains("ran at a different tree than the working tree") && e.contains(TREE_NAMES[0]),
        "{e}"
    );
    let j = verify_file(&joined(), &trees());
    let e = sign_with(&j, &moved_trees()).unwrap_err();
    assert!(
        e.contains("ran at a different tree than the working tree"),
        "{e}"
    );
    // One input only.
    let mut one = trees();
    one[5].1 = "e".repeat(40);
    let e = sign_with(&verify_file(&joined(), &one), &trees()).unwrap_err();
    assert!(e.contains(TREE_NAMES[5]), "{e}");
}

#[test]
fn a_verify_revision_whose_trees_differ_or_that_is_missing_is_refused() {
    let now = moved_trees();
    let j = verify_file(&joined(), &now);
    let input = joined().to_text();
    let jt = j.to_text();
    let r = SignRequest {
        expect_sha256: Some(digest(&input)),
        verify_text: Some(jt.clone()),
        expect_verify_sha256: Some(digest(&jt)),
        ..req()
    };
    // The commit named in J's `revisions.repo` holds other trees than J says it ran at.
    let other = |rev: &str| -> Result<Vec<(String, String)>, String> {
        Ok(if rev == VHEAD {
            let mut t = moved_trees();
            t[4].1 = "e".repeat(40);
            t
        } else {
            trees()
        })
    };
    let e = sign(&input, &r, &now, &other).unwrap_err();
    assert!(
        e.contains("differ from the verify file's `trees`") && e.contains(TREE_NAMES[4]),
        "{e}"
    );
    // A different number of trees differs too.
    let short = |rev: &str| -> Result<Vec<(String, String)>, String> {
        Ok(if rev == VHEAD {
            moved_trees().into_iter().take(3).collect()
        } else {
            trees()
        })
    };
    let e = sign(&input, &r, &now, &short).unwrap_err();
    assert!(e.contains("differ from the verify file's `trees`"), "{e}");
    // The commit is not in the checkout: the fetch hint is named.
    let gone = |rev: &str| -> Result<Vec<(String, String)>, String> {
        if rev == VHEAD {
            missing(rev)
        } else {
            Ok(trees())
        }
    };
    let e = sign(&input, &r, &now, &gone).unwrap_err();
    assert!(
        e.starts_with("sign refuses: ") && e.contains(&format!("git fetch origin {VHEAD}")),
        "{e}"
    );
}

#[test]
fn a_verify_of_another_file_is_refused() {
    refused_with(
        &|j| j.meta.recording.as_mut().unwrap().verifies_sha256 = Some("0".repeat(64)),
        "verified a different file",
    );
    refused_with(
        &|j| j.meta.recording.as_mut().unwrap().verifies_sha256 = None,
        "names no recorded file",
    );
}

#[test]
fn a_verify_whose_rows_are_not_the_recordings_is_refused() {
    // A whole knob group missing from the verify: a different identity set.
    refused_with(
        &|j| {
            let first = j.rows[0].clone();
            j.rows.retain(|x| {
                !(x.experiment == first.experiment && x.mode == first.mode && x.knob == first.knob)
            });
        },
        "the files hold different rows",
    );
}

#[test]
fn a_verify_with_a_row_difference_is_refused_and_names_it() {
    refused_with(
        &|j| {
            j.rows
                .iter_mut()
                .find(|r| r.experiment == "A1")
                .expect("an A1 row")
                .propagations += 1;
        },
        "does not reproduce the recording: 1 row difference(s) and 0 decision difference(s); \
         first: ",
    );
    refused_with(
        &|j| {
            j.rows
                .iter_mut()
                .find(|r| r.experiment == "A1")
                .unwrap()
                .propagations += 1;
        },
        "propagations differs",
    );
}

#[test]
fn a_verify_with_a_decision_difference_is_refused() {
    // The decision-driving D3 rows move: row differences and a decision difference both.
    let mut j = verify_file(&joined(), &trees());
    for r in j.rows.iter_mut().filter(|r| r.experiment == "D3") {
        r.worst_step_conflicts = r.worst_step_conflicts.map(|c| c + 1_000_000);
    }
    let e = sign_with(&j, &trees()).unwrap_err();
    assert!(e.contains("does not reproduce the recording"), "{e}");
    assert!(!e.contains("and 0 decision difference(s)"), "{e}");
    assert!(e.contains("run `decision-diff` for the full list"), "{e}");
    // The same comparison, called directly, is what both commands share.
    let rec = joined();
    let found = vos_capacity_probe::cli::compare_rerun(
        &rec,
        &j,
        Some(64),
        &vos_capacity_probe::cli::RerunLabels {
            recorded: "FILE",
            rerun: "--verify-file",
        },
    )
    .expect("authentic");
    assert!(!found.decision_diffs.is_empty());
    assert!(!found.row_diffs.is_empty());
    assert!(!found.is_clean());
    let e = vos_capacity_probe::cli::compare_rerun(
        &rec,
        &j,
        Some(32),
        &vos_capacity_probe::cli::RerunLabels {
            recorded: "FILE",
            rerun: "--verify-file",
        },
    )
    .unwrap_err();
    assert!(
        e.contains("--verify-file joins 64 shards but --expect-shards is 32"),
        "{e}"
    );
}

#[test]
fn a_verify_without_its_own_run_or_revision_cannot_be_cited() {
    refused_with(
        &|j| {
            let r = j.meta.recording.as_mut().unwrap();
            r.run_id = None;
            r.run_attempt = None;
        },
        "carries no run id and attempt",
    );
    refused_with(
        &|j| {
            for (k, v) in j.meta.revisions.iter_mut() {
                if k == "repo" {
                    *v = "unknown".into();
                }
            }
        },
        "--verify-file",
    );
}

#[test]
fn every_existing_check_still_refuses_before_the_verify_is_read() {
    // The verify evidence is added to the old checks, not substituted for them: a recording that
    // fails one is refused with the old message even though the verify file is clean.
    let input = joined().to_text();
    let mut r = req();
    r.head_sha = "f".repeat(40);
    refused(&input, &r, "is not the recorded repo revision");
    let mut r = req();
    r.run_id = "124".into();
    refused(&input, &r, "--run-id 124 is not the recording's run id");
    refused(
        &input,
        &SignRequest {
            expect_sha256: Some("0".repeat(64)),
            ..req()
        },
        "not the expected",
    );
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
