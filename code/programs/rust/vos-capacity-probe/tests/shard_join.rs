//! VOS-028c3c-2b-i: sharding, the `recording` block and `join`.
//!
//! Everything here is pure over `Results` (or reads a temp file through the real dispatcher), so
//! it runs on every platform.  The shards are made from the committed results file completed
//! with fixture rows for the plan rows it does not yet hold (`common::complete_plan`; VOS-028c3c
//! commit A registered B3 every2/every4, D5 and B5 before the recording), so slicing it by
//! `plan_shard` gives structurally valid shard files without solving anything.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::exp::{
    b3_c3_knobs, b5_knobs, d5_knobs, full_knob_list, parse_shard, plan, plan_shard, shard_of, Tier,
    MAX_SHARDS,
};
use vos_capacity_probe::join::{join, Expect};
use vos_capacity_probe::results::{Recording, Regeneration, Results, MIN_SEEDS, TREE_NAMES};

fn committed_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json")
}

fn committed() -> Results {
    Results::from_text(&std::fs::read_to_string(committed_path()).unwrap()).unwrap()
}

type Id = (String, String, String, u64);

fn id_of(r: &vos_capacity_probe::row::Row) -> Id {
    (r.experiment.clone(), r.mode.clone(), r.knob.key(), r.seed)
}

// ---------------------------------------------------------------------------------------
// The committed file is untouched by the new key
// ---------------------------------------------------------------------------------------

#[test]
fn the_committed_file_still_round_trips() {
    let text = std::fs::read_to_string(committed_path()).unwrap();
    let res = Results::from_canonical_text(&text).expect("canonical");
    assert_eq!(res.to_text(), text, "the committed bytes are unchanged");
    // The key is omitted when absent, whether or not the committed file has a recording block.
    let mut bare = res.clone();
    bare.meta.recording = None;
    assert!(
        !bare.to_text().contains("\"recording\""),
        "the key is omitted when absent"
    );
}

// ---------------------------------------------------------------------------------------
// Shard arithmetic
// ---------------------------------------------------------------------------------------

#[test]
fn parse_shard_accepts_in_range_and_refuses_the_rest() {
    assert_eq!(parse_shard("0/64"), Ok((0, 64)));
    assert_eq!(parse_shard("63/64"), Ok((63, 64)));
    assert_eq!(parse_shard("0/1"), Ok((0, 1)));
    assert_eq!(parse_shard("1023/1024"), Ok((1023, 1024)));
    for bad in [
        "64/64",
        "0/0",
        "1/1025",
        "0/1025",
        "a/b",
        "-1/4",
        "+1/4",
        "1/+4",
        "1/4/5",
        "1",
        "",
        "/",
        "1/",
        "/4",
        " 1/4",
        "1/4 ",
        "0x1/4",
        "99999999999/4",
    ] {
        assert!(parse_shard(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn shard_of_is_round_robin() {
    assert_eq!(shard_of(0, 64), 0);
    assert_eq!(shard_of(63, 64), 63);
    assert_eq!(shard_of(64, 64), 0);
    assert_eq!(shard_of(130, 64), 2);
    assert_eq!(shard_of(5, 1), 0);
}

#[test]
fn the_shards_partition_the_plan() {
    let whole: Vec<Id> = plan(Tier::Full, &[], MIN_SEEDS)
        .into_iter()
        .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
        .collect();
    let distinct: BTreeSet<&Id> = whole.iter().collect();
    assert_eq!(
        distinct.len(),
        whole.len(),
        "the plan has no repeated identity"
    );
    for n in [1u32, 2, 7, 64, 128] {
        let mut seen: Vec<Id> = Vec::new();
        for k in 0..n {
            let part: Vec<Id> = plan_shard(MIN_SEEDS, k, n)
                .into_iter()
                .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
                .collect();
            seen.extend(part);
        }
        let got: BTreeSet<&Id> = seen.iter().collect();
        assert_eq!(seen.len(), whole.len(), "N = {n}: every row exactly once");
        assert_eq!(got, distinct, "N = {n}: the union is the plan");
    }
    // One shard of one is the plan itself, in plan order.
    let one: Vec<Id> = plan_shard(MIN_SEEDS, 0, 1)
        .into_iter()
        .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
        .collect();
    assert_eq!(one, whole);
}

#[test]
fn all_seeds_of_a_knob_stay_in_one_shard() {
    for k in 0..7 {
        let part = plan_shard(MIN_SEEDS, k, 7);
        for s in &part {
            let same = part
                .iter()
                .filter(|t| {
                    (&t.experiment, &t.mode, t.knob.key()) == (&s.experiment, &s.mode, s.knob.key())
                })
                .count();
            assert_eq!(same as u32, MIN_SEEDS, "{} {}", s.experiment, s.knob.key());
        }
    }
}

/// Per-shard counts of each experiment label, for a list of knob owners in plan order.
fn per_shard(labels: &[&str], n: u32, of: &str) -> Vec<usize> {
    let mut counts = vec![0usize; n as usize];
    for (i, l) in labels.iter().enumerate() {
        if *l == of {
            counts[shard_of(i, n) as usize] += 1;
        }
    }
    counts
}

#[test]
fn no_shard_holds_more_than_one_b5_knob() {
    // The plan as VOS-028c3c will register it: the current knobs, the B3 every2/every4 block,
    // D5 and B5.  B5 is inserted at every offset, so the property holds wherever the real
    // registration puts it, provided it stays one contiguous block.  Two layouts: the extra B3
    // knobs appended after everything (the worst case for B5: nothing keeps them near B3), and
    // spliced in next to the existing B3 knobs (how `knobs("B3")` will register them).
    // The plan as it stood before VOS-028c3c commit A, rebuilt from the real list: B5 and D5
    // dropped, and the 72 B3 cut knobs (the last B3 labels) dropped.
    let (c3, d5, b5) = (b3_c3_knobs().len(), d5_knobs().len(), b5_knobs().len());
    assert_eq!((c3, d5, b5), (72, 24, 24));
    let mut current: Vec<&str> = full_knob_list()
        .into_iter()
        .map(|(e, _)| e)
        .filter(|e| *e != "B5" && *e != "D5")
        .collect();
    let end_b3 = current.iter().rposition(|l| *l == "B3").unwrap() + 1;
    current.drain(end_b3 - c3..end_b3);
    assert_eq!(current.len(), 445 - 24 - 24 - 72);
    let mut appended = current.clone();
    appended.extend(std::iter::repeat_n("B3", c3));
    appended.extend(std::iter::repeat_n("D5", d5));
    let last_b3 = current.iter().rposition(|l| *l == "B3").unwrap();
    let first_b3 = current.iter().position(|l| *l == "B3").unwrap();
    let mut spliced = current.clone();
    for _ in 0..c3 {
        spliced.insert(last_b3 + 1, "B3");
    }
    spliced.extend(std::iter::repeat_n("D5", d5));
    for (layout, base) in [("appended", &appended), ("spliced", &spliced)] {
        for off in 0..=base.len() {
            let mut labels = base.clone();
            for j in 0..b5 {
                labels.insert(off + j, "B5");
            }
            assert_eq!(labels.len(), 445, "VOS30 9.2: 445 knobs after activation");
            for n in [64u32, 128] {
                let b5s = per_shard(&labels, n, "B5");
                assert!(
                    b5s.iter().all(|&c| c <= 1),
                    "{layout} offset {off}, N = {n}: B5 {b5s:?}"
                );
                assert_eq!(b5s.iter().sum::<usize>(), 24);
                let d5s = per_shard(&labels, n, "D5");
                assert!(
                    d5s.iter().all(|&c| c <= 1),
                    "{layout} offset {off}, N = {n}: D5"
                );
            }
        }
    }
    // "At most three B3 knobs" (VOS30 9.2) holds for the layout where the B3 block is one
    // contiguous run: it is 180 knobs, so at most ceil(180/64) = 3 per shard at N = 64.  A B5
    // block cannot land inside a registered block, so offsets inside B3's run are not asserted.
    let (lo, hi) = (first_b3, last_b3 + 1 + c3);
    for off in (0..=spliced.len()).filter(|o| *o <= lo || *o >= hi) {
        let mut labels = spliced.clone();
        for j in 0..b5 {
            labels.insert(off + j, "B5");
        }
        for n in [64u32, 128] {
            let b3s = per_shard(&labels, n, "B3");
            assert!(
                b3s.iter().all(|&c| c <= 3),
                "offset {off}, N = {n}: B3 {b3s:?}"
            );
        }
    }
    // The appended layout is why the spec's wording needs the contiguity: B3 can reach 4 there.
    assert!(per_shard(&appended, 64, "B3").contains(&4));
}

/// Self-arming since VOS-028c3c commit A: the REAL plan keeps B5 and D5 each one contiguous
/// block of 24 knobs with at most one per shard, and the B3 knobs one contiguous run of 180 with at
/// most three per shard at N = 64.  A registration that splits a block fails here, not on a runner.
#[test]
fn the_real_plan_keeps_b5_one_per_shard() {
    let labels: Vec<&str> = full_knob_list().into_iter().map(|(e, _)| e).collect();
    assert_eq!(labels.len(), 445, "VOS30 9.2: 445 knobs after activation");
    let at = |exp: &str| -> Vec<usize> {
        labels
            .iter()
            .enumerate()
            .filter(|(_, l)| **l == exp)
            .map(|(i, _)| i)
            .collect()
    };
    // VOS30 9.2: 445 knobs at 5 seeds is 2,225 rows; the quick tier is 105 rows (85 before, plus
    // the smallest B5 and D5 knobs, 2 knobs x 5 seeds each).
    assert_eq!(plan(Tier::Full, &[], MIN_SEEDS).len(), 2_225);
    assert_eq!(plan(Tier::Quick, &[], MIN_SEEDS).len(), 105);
    for (exp, count, max) in [("B5", 24usize, 1usize), ("D5", 24, 1), ("B3", 180, 3)] {
        let a = at(exp);
        assert_eq!(a.len(), count, "{exp} knob count");
        assert_eq!(
            a[count - 1] - a[0],
            count - 1,
            "{exp} is one contiguous block"
        );
        let counts = per_shard(&labels, 64, exp);
        assert!(
            counts.iter().all(|&c| c <= max),
            "{exp} at N = 64: {counts:?}"
        );
    }
    for n in [64u32, 128] {
        for exp in ["B5", "D5"] {
            assert!(
                per_shard(&labels, n, exp).iter().all(|&c| c <= 1),
                "{exp} N = {n}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// The recording block
// ---------------------------------------------------------------------------------------

fn trees() -> Vec<(String, String)> {
    TREE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{:040x}", i + 1)))
        .collect()
}

fn rec(k: u32, n: u32) -> Recording {
    Recording {
        mode: "record".into(),
        run_id: Some("1234567890".into()),
        run_attempt: Some("1".into()),
        shard: Some((k, n)),
        shards: None,
        jobs: 2,
        verifies_sha256: None,
    }
}

/// A small, valid-shaped file for the recording tests (the committed rows, one meta).
fn base() -> Results {
    let mut r = common::complete_plan(committed());
    r.meta.merges.clear();
    r.meta.regeneration = None;
    r.meta.trees = trees();
    r
}

#[test]
fn the_recording_block_round_trips() {
    for rc in [
        rec(3, 64),
        Recording {
            run_id: None,
            run_attempt: None,
            ..rec(0, 1)
        },
        Recording {
            mode: "verify".into(),
            shard: None,
            shards: Some(64),
            ..rec(0, 1)
        },
    ] {
        let mut r = base();
        r.meta.recording = Some(rc.clone());
        let text = r.to_text();
        assert!(text.contains("\n  \"recording\": {"), "{text:.400}");
        let back = Results::from_canonical_text(&text).expect("canonical");
        assert_eq!(back.meta.recording, Some(rc));
        assert_eq!(back.to_text(), text);
    }
}

#[test]
fn a_bad_run_id_or_shape_is_refused_on_read() {
    let mut r = base();
    r.meta.recording = Some(rec(3, 64));
    let text = r.to_text();
    let with = |from: &str, to: &str| {
        assert!(text.contains(from), "{from}");
        Results::from_text(&text.replacen(from, to, 1))
    };
    assert!(with("\"run_id\":\"1234567890\"", "\"run_id\":\"1234567890\"").is_ok());
    for bad in [
        "12a4",
        "",
        "+12",
        "-1",
        " 1",
        "123456789012345678901",
        "1e3",
        "١٢٣",
    ] {
        let e = with(
            "\"run_id\":\"1234567890\"",
            &format!("\"run_id\":\"{bad}\""),
        );
        assert!(e.is_err(), "run_id {bad:?} must be refused");
        let e = with(
            "\"run_attempt\":\"1\"",
            &format!("\"run_attempt\":\"{bad}\""),
        );
        assert!(e.is_err(), "run_attempt {bad:?} must be refused");
    }
    // 20 digits is the longest accepted.
    assert!(with(
        "\"run_id\":\"1234567890\"",
        "\"run_id\":\"12345678901234567890\""
    )
    .is_ok());
    // A number where a string belongs, an unknown mode, an unknown key, both or neither of
    // shard and shards, one of run_id and run_attempt, a bad shard string.
    assert!(with("\"run_id\":\"1234567890\"", "\"run_id\":5").is_err());
    assert!(with("\"mode\":\"record\"", "\"mode\":\"fast\"").is_err());
    assert!(with("\"mode\":\"record\"", "\"mode\":\"record\",\"extra\":1").is_err());
    assert!(with("\"shard\":\"3/64\"", "\"shard\":\"3/64\",\"shards\":64").is_err());
    assert!(with("\"shard\":\"3/64\",", "").is_err());
    assert!(with("\"shard\":\"3/64\"", "\"shard\":\"64/64\"").is_err());
    assert!(with("\"run_attempt\":\"1\",", "").is_err());
}

// ---------------------------------------------------------------------------------------
// join
// ---------------------------------------------------------------------------------------

/// The committed rows cut into `n` shard files, each with its own recording.
fn shards(n: u32) -> Vec<(String, Results)> {
    let all = base();
    (0..n)
        .map(|k| {
            let want: BTreeSet<Id> = plan_shard(MIN_SEEDS, k, n)
                .into_iter()
                .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
                .collect();
            let mut r = all.clone();
            r.rows.retain(|row| want.contains(&id_of(row)));
            assert_eq!(
                r.rows.len(),
                want.len(),
                "the committed file covers the plan"
            );
            r.meta.recording = Some(rec(k, n));
            (format!("shard-{k}.json"), r)
        })
        .collect()
}

fn no_expect() -> Expect {
    Expect::default()
}

fn refused(inputs: &[(String, Results)], expect: &Expect, needle: &str) {
    match join(inputs, expect) {
        Ok(_) => panic!("join must refuse ({needle})"),
        Err(e) => assert!(e.contains(needle), "wanted {needle:?} in {e:?}"),
    }
}

#[test]
fn a_set_of_shards_joins_into_the_plan() {
    let all = base();
    for n in [1u32, 5, 64] {
        let j = join(&shards(n), &no_expect()).expect("joins");
        let plan_ids: Vec<Id> = plan(Tier::Full, &[], MIN_SEEDS)
            .into_iter()
            .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
            .collect();
        let got: Vec<Id> = j.results.rows.iter().map(id_of).collect();
        assert_eq!(got, plan_ids, "N = {n}: rows in plan order");
        for r in &j.results.rows {
            let orig = all.rows.iter().find(|o| id_of(o) == id_of(r)).unwrap();
            assert_eq!(r, orig, "a row's bytes are the shard's");
        }
        let rc = j.results.meta.recording.as_ref().unwrap();
        assert_eq!(
            (rc.mode.as_str(), rc.shard, rc.shards, rc.jobs),
            ("record", None, Some(n), 2)
        );
        assert_eq!(rc.run_id.as_deref(), Some("1234567890"));
        assert_eq!(rc.run_attempt.as_deref(), Some("1"));
        assert!(j.findings.is_empty(), "{:?}", j.findings);
        assert!(j.results.meta.merges.is_empty() && j.results.meta.regeneration.is_none());
        // And the joined file is one the tool accepts back.
        Results::from_canonical_text(&j.results.to_text()).expect("canonical");
    }
}

#[test]
fn join_is_independent_of_argument_order() {
    let ins = shards(5);
    let a = join(&ins, &no_expect()).unwrap().results.to_text();
    let mut rev = ins.clone();
    rev.reverse();
    let mut rot = ins.clone();
    rot.rotate_left(2);
    for other in [rev, rot] {
        assert_eq!(join(&other, &no_expect()).unwrap().results.to_text(), a);
    }
    // Row order inside a file does not matter either.
    let mut shuffled = ins.clone();
    shuffled[1].1.rows.reverse();
    assert_eq!(join(&shuffled, &no_expect()).unwrap().results.to_text(), a);
}

#[test]
fn row_findings_are_reported_and_do_not_refuse() {
    let mut ins = shards(5);
    // A red `green` is a row finding (a kept row outcome), not a structural one.
    ins[2].1.rows[0].green = !ins[2].1.rows[0].green;
    let j = join(&ins, &no_expect()).expect("a row finding does not refuse");
    assert!(!j.findings.is_empty());
}

#[test]
fn an_empty_input_list_is_refused() {
    refused(&[], &no_expect(), "at least one shard");
}

#[test]
fn a_structurally_invalid_shard_is_refused() {
    let mut ins = shards(5);
    ins[1].1.meta.dirty_tree = true;
    refused(&ins, &no_expect(), "structurally invalid");
    let mut ins = shards(5);
    ins[3].1.rows[0].tier = "bogus".into();
    refused(&ins, &no_expect(), "shard-3.json: structurally invalid");
}

#[test]
fn a_shard_without_a_recording_or_a_shard_key_is_refused() {
    let mut ins = shards(5);
    ins[0].1.meta.recording = None;
    refused(&ins, &no_expect(), "no recording block");
    let mut ins = shards(5);
    ins[0].1.meta.recording = Some(Recording {
        shard: None,
        shards: Some(5),
        ..rec(0, 5)
    });
    refused(&ins, &no_expect(), "not a single shard's");
}

#[test]
fn a_missing_duplicate_or_extra_shard_is_refused() {
    let mut ins = shards(5);
    ins.remove(2);
    refused(&ins, &no_expect(), "shard 2/5 is missing");
    let mut ins = shards(5);
    ins.push(ins[1].clone());
    refused(&ins, &no_expect(), "shard 1/5 is given twice");
    let mut ins = shards(5);
    ins.pop();
    refused(&ins, &no_expect(), "shard 4/5 is missing");
}

#[test]
fn shards_of_different_n_are_refused() {
    let mut ins = shards(5);
    ins[1].1.meta.recording = Some(rec(1, 6));
    refused(
        &ins,
        &no_expect(),
        "is a shard of 6 but another input is a shard of 5",
    );
}

#[test]
fn every_provenance_disagreement_is_refused() {
    type Change = Box<dyn Fn(&mut Results)>;
    let cases: Vec<(&str, Change)> = vec![
        ("tool name", Box::new(|r| r.meta.name = "other".into())),
        (
            "tool version",
            Box::new(|r| r.meta.version = "9.9.9".into()),
        ),
        (
            "binary_sha256",
            Box::new(|r| r.meta.binary_sha256 = "ab".repeat(32)),
        ),
        (
            "tree ids",
            Box::new(|r| r.meta.trees[0].1 = format!("{:040x}", 99)),
        ),
        (
            "revisions",
            Box::new(|r| r.meta.revisions[0].1 = "f".repeat(40)),
        ),
        (
            "host triple",
            Box::new(|r| r.meta.triple = "x86_64-apple-darwin".into()),
        ),
        (
            "recording mode",
            Box::new(|r| r.meta.recording.as_mut().unwrap().mode = "verify".into()),
        ),
        (
            "jobs",
            Box::new(|r| r.meta.recording.as_mut().unwrap().jobs = 3),
        ),
        (
            "run id",
            Box::new(|r| r.meta.recording.as_mut().unwrap().run_id = Some("999".into())),
        ),
        (
            "run attempt",
            Box::new(|r| r.meta.recording.as_mut().unwrap().run_attempt = Some("2".into())),
        ),
    ];
    for (what, change) in &cases {
        let mut ins = shards(5);
        change(&mut ins[3].1);
        refused(&ins, &no_expect(), &format!("{what} of shard 3/5"));
    }
    // A run id present in one shard and absent in another is a disagreement too.
    let mut ins = shards(5);
    let rc = ins[2].1.meta.recording.as_mut().unwrap();
    rc.run_id = None;
    rc.run_attempt = None;
    refused(&ins, &no_expect(), "run id of shard 2/5");
}

#[test]
fn host_os_may_differ_between_shards() {
    // VOS-D133: `uname -sr` is a property of the runner's kernel, the triple is the binary's.
    let mut ins = shards(5);
    ins[2].1.meta.os = "Linux 6.8.0-1017-azure".into();
    ins[3].1.meta.rustc = "rustc 1.99.0".into();
    join(&ins, &no_expect()).expect("os and rustc are not compared");
}

#[test]
fn a_bad_binary_digest_or_missing_provenance_is_refused_even_if_equal() {
    let mut ins = shards(5);
    for (_, r) in &mut ins {
        r.meta.binary_sha256 = "unavailable".into();
    }
    refused(&ins, &no_expect(), "not 64 hex digits");
    let mut ins = shards(5);
    for (_, r) in &mut ins {
        r.meta.trees.clear();
    }
    refused(&ins, &no_expect(), "no tree ids");
    let mut ins = shards(5);
    for (_, r) in &mut ins {
        r.meta.revisions.clear();
    }
    refused(&ins, &no_expect(), "no revisions");
    let mut ins = shards(5);
    for (_, r) in &mut ins {
        r.meta.triple = "unknown".into();
    }
    refused(&ins, &no_expect(), "no host triple");
}

#[test]
fn merges_and_a_banner_are_refused() {
    let mut ins = shards(5);
    ins[1].1.meta.regeneration = Some(Regeneration {
        reviewer: "A".into(),
        host: "h".into(),
        run_id: "1".into(),
        run_attempt: "1".into(),
        head_sha: "a".repeat(40),
        unsigned_sha256: "b".repeat(64),
        verified: vos_capacity_probe::results::Verified {
            run_id: "2".into(),
            run_attempt: "1".into(),
            head_sha: "c".repeat(40),
            verify_sha256: "d".repeat(64),
            trees: Vec::new(),
        },
    });
    refused(
        &ins,
        &no_expect(),
        "shard-1.json: it has a regeneration banner",
    );
    let mut ins = shards(5);
    // A merge record built here: the committed recording has none (a CI recording replaces the
    // merged file), and the refusal must not depend on the file having one.
    ins[4].1.meta.merges = vec![vos_capacity_probe::results::Merge {
        rev: "a".repeat(40),
        binary_sha256: "b".repeat(64),
        dirty_tree: false,
        experiment: "A1".into(),
        where_: String::new(),
        rows: vec![],
        at: "2026-11-19T00:00:00Z".into(),
        kind: None,
        trees: vec![],
    }];
    refused(&ins, &no_expect(), "shard-4.json: it has merges");
}

/// Remove, and return, all the rows of the first knob of `r` (five seeds: the file stays
/// structurally valid, which is the point of the tests that use it).
fn take_a_knob(r: &mut Results) -> Vec<vos_capacity_probe::row::Row> {
    let key = (
        r.rows[0].experiment.clone(),
        r.rows[0].mode.clone(),
        r.rows[0].knob.key(),
    );
    let (taken, kept): (Vec<_>, Vec<_>) = r
        .rows
        .drain(..)
        .partition(|x| (&x.experiment, &x.mode, x.knob.key()) == (&key.0, &key.1, key.2.clone()));
    r.rows = kept;
    taken
}

#[test]
fn rows_that_are_not_the_shards_slice_are_refused() {
    // Without a plan comparison every one of these would join: the shard set is complete, the
    // provenance agrees and each file is structurally valid.
    // (1) Another shard's knob smuggled in.
    let mut ins = shards(5);
    let knob = take_a_knob(&mut ins[0].1);
    ins[1].1.rows.extend(knob);
    refused(
        &ins,
        &no_expect(),
        "shard-0.json: its rows are not exactly the plan's shard 0/5",
    );
    // (2) A knob missing from its own shard.
    let mut ins = shards(5);
    take_a_knob(&mut ins[2].1);
    refused(
        &ins,
        &no_expect(),
        "shard-2.json: its rows are not exactly the plan's shard 2/5",
    );
    // (3) A knob doubled inside one shard (a repeated identity).
    let mut ins = shards(5);
    let again = ins[3].1.rows[..5].to_vec();
    ins[3].1.rows.extend(again);
    assert!(join(&ins, &no_expect()).is_err());
    // (4) A joined file's recording (`shards`) where a shard's is expected.
    let mut ins = shards(5);
    for (_, r) in &mut ins {
        r.meta.recording.as_mut().unwrap().shard = None;
        r.meta.recording.as_mut().unwrap().shards = Some(5);
    }
    refused(&ins, &no_expect(), "not a single shard's");
}

#[test]
fn a_swapped_shard_file_is_refused() {
    // Shard 1's rows under shard 2's recording and the other way round: every provenance field
    // agrees, the set is exactly 0..N, and still neither file holds its own slice.
    let mut ins = shards(5);
    let (a, b) = (ins[1].1.rows.clone(), ins[2].1.rows.clone());
    ins[1].1.rows = b;
    ins[2].1.rows = a;
    refused(&ins, &no_expect(), "not exactly the plan's shard");
    // Two recordings swapped over their own rows is the same swap.
    let mut ins = shards(5);
    ins[3].1.meta.recording = Some(rec(4, 5));
    ins[4].1.meta.recording = Some(rec(3, 5));
    refused(&ins, &no_expect(), "not exactly the plan's shard");
}

#[test]
fn the_expect_flags_refuse_on_mismatch_and_pass_on_a_match() {
    let ins = shards(5);
    let good = Expect {
        binary_sha256: Some(ins[0].1.meta.binary_sha256.clone()),
        run_id: Some("1234567890".into()),
        run_attempt: Some("1".into()),
        shards: Some(5),
        jobs: Some(2),
    };
    join(&ins, &good).expect("every expectation met");
    refused(
        &ins,
        &Expect {
            binary_sha256: Some("cd".repeat(32)),
            ..good.clone()
        },
        "--expect-binary-sha256",
    );
    refused(
        &ins,
        &Expect {
            binary_sha256: Some("nothex".into()),
            ..good.clone()
        },
        "--expect-binary-sha256 is not 64 hex",
    );
    refused(
        &ins,
        &Expect {
            run_id: Some("4321".into()),
            ..good.clone()
        },
        "--expect-run-id",
    );
    refused(
        &ins,
        &Expect {
            run_attempt: Some("2".into()),
            ..good.clone()
        },
        "--expect-run-attempt",
    );
    refused(
        &ins,
        &Expect {
            run_id: Some("12x".into()),
            ..good.clone()
        },
        "--expect-run-id is not 1 to 20 ASCII digits",
    );
    refused(
        &ins,
        &Expect {
            shards: Some(6),
            ..good.clone()
        },
        "--expect-shards",
    );
    refused(
        &ins,
        &Expect {
            jobs: Some(4),
            ..good
        },
        "--expect-jobs",
    );
}

#[test]
fn join_of_local_shards_refuses_expect_run_id() {
    let mut ins = shards(5);
    for (_, r) in &mut ins {
        let rc = r.meta.recording.as_mut().unwrap();
        rc.run_id = None;
        rc.run_attempt = None;
    }
    let j = join(&ins, &no_expect()).expect("local shards join among themselves");
    let rc = j.results.meta.recording.unwrap();
    assert!(rc.run_id.is_none() && rc.run_attempt.is_none());
    refused(
        &ins,
        &Expect {
            run_id: Some("1".into()),
            ..no_expect()
        },
        "a local run",
    );
    refused(
        &ins,
        &Expect {
            run_attempt: Some("1".into()),
            ..no_expect()
        },
        "a local run",
    );
}

// ---------------------------------------------------------------------------------------
// The commands, through the real dispatcher
// ---------------------------------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-join-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(args: &[&str]) -> (Result<(), Failure>, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = dispatch(&args, &mut out, &mut err);
    (
        r,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn write_shards(dir: &std::path::Path, n: u32) -> Vec<String> {
    shards(n)
        .into_iter()
        .map(|(name, r)| {
            let p = dir.join(name);
            std::fs::write(&p, r.to_text()).unwrap();
            p.to_string_lossy().into_owned()
        })
        .collect()
}

#[test]
fn the_join_command_writes_a_canonical_file_that_validates() {
    let dir = scratch("happy");
    let files = write_shards(&dir, 5);
    let out = dir.join("joined.json").to_string_lossy().into_owned();
    let mut args = vec!["join", "--out", &out, "--expect-run-id", "1234567890"];
    args.extend([
        "--expect-run-attempt",
        "1",
        "--expect-shards",
        "5",
        "--expect-jobs",
        "2",
    ]);
    let mut rev: Vec<&str> = files.iter().rev().map(String::as_str).collect();
    args.append(&mut rev);
    let (r, o, _) = run(&args);
    assert!(r.is_ok(), "{r:?}");
    assert!(o.starts_with("joined 5 shards into "), "{o}");
    let text = std::fs::read_to_string(&out).unwrap();
    let back = Results::from_canonical_text(&text).unwrap();
    assert_eq!(back.meta.recording.as_ref().unwrap().shards, Some(5));
    let (r, _, _) = run(&["validate", &out]);
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn a_refused_join_exits_2_and_writes_nothing() {
    let dir = scratch("refuse");
    let mut files = write_shards(&dir, 5);
    let out = dir.join("joined.json");
    let out_s = out.to_string_lossy().into_owned();
    let call = |files: &[String], extra: &[&str]| {
        let mut args = vec!["join", "--out", out_s.as_str()];
        args.extend(extra.iter().copied());
        args.extend(files.iter().map(String::as_str));
        run(&args).0
    };
    // A missing shard.
    let r = call(&files[1..], &[]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    // A wrong expectation.
    let r = call(&files, &["--expect-run-id", "5"]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    // A typo'd flag is not silently ignored (it would skip the gate it names).
    let r = call(&files, &["--expect-run-ids", "1234567890"]);
    assert!(
        matches!(&r, Err(Failure::Usage(m)) if m.contains("unknown flag")),
        "{r:?}"
    );
    // A flag with no value, and a non-numeric --expect-shards.
    let r = call(&files, &["--expect-shards"]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    let r = call(&files, &["--expect-shards", "five"]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    // A shard file that is not in canonical form.
    let text = std::fs::read_to_string(&files[0]).unwrap();
    std::fs::write(&files[0], text.replacen("{\n", "{ \n", 1)).unwrap();
    let r = call(&files, &[]);
    assert!(
        matches!(&r, Err(Failure::Usage(m)) if m.contains("canonical")),
        "{r:?}"
    );
    std::fs::write(&files[0], text).unwrap();
    // An unreadable input.
    files.push(dir.join("absent.json").to_string_lossy().into_owned());
    let r = call(&files, &[]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    // No --out, no inputs.
    let (r, _, _) = run(&["join", files[0].as_str()]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    let (r, _, _) = run(&["join", "--out", out_s.as_str()]);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    assert!(!out.exists(), "no refusal wrote the output");
}

#[test]
fn run_shard_refuses_what_it_should_before_running_anything() {
    let usage = |args: &[&str], needle: &str| {
        let (r, _, _) = run(args);
        match r {
            Err(Failure::Usage(m)) => assert!(m.contains(needle), "{args:?}: {m}"),
            other => panic!("{args:?}: {other:?}"),
        }
    };
    usage(
        &["run", "--tier", "quick", "--shard", "0/4"],
        "--shard needs --tier full",
    );
    usage(
        &["run", "--tier", "full", "--shard", "0/4", "--only", "A1"],
        "--only",
    );
    for bad in ["4/4", "0/0", "1/1025", "a/b", "-1/4"] {
        usage(
            &["run", "--tier", "full", "--shard", bad],
            "--shard wants K/N",
        );
    }
    usage(
        &["run", "--tier", "full", "--shard"],
        "--shard needs a value",
    );
    usage(
        &[
            "run",
            "--tier",
            "full",
            "--run-id",
            "5",
            "--run-attempt",
            "1",
        ],
        "need --shard",
    );
    usage(
        &["run", "--tier", "full", "--shard", "0/4", "--run-id", "5"],
        "go together",
    );
    for bad in ["x", "", "+5", "123456789012345678901"] {
        usage(
            &[
                "run",
                "--tier",
                "full",
                "--shard",
                "0/4",
                "--run-id",
                bad,
                "--run-attempt",
                "1",
            ],
            "ASCII digits",
        );
    }
}

#[test]
fn a_row_that_records_another_seed_count_is_refused() {
    // The plan is for MIN_SEEDS seeds per knob.  A row with the right identity but another
    // `seeds` would otherwise be joined as if it were the plan's.
    let mut ins = shards(5);
    ins[1].1.rows[0].seeds = MIN_SEEDS + 1;
    refused(&ins, &no_expect(), "records seeds");
    let mut ins = shards(5);
    ins[4].1.rows[0].seeds = MIN_SEEDS - 1;
    // (Fewer than MIN_SEEDS is caught even earlier, by structural validation.)
    assert!(join(&ins, &no_expect()).is_err());
}

#[test]
fn join_caps_its_inputs_and_refuses_repeated_flags_and_overwrites() {
    let dir = scratch("hardening");
    let files = write_shards(&dir, 5);
    let out = dir.join("joined.json");
    let out_s = out.to_string_lossy().into_owned();
    let usage = |args: &[&str], needle: &str| {
        let (r, _, _) = run(args);
        match r {
            Err(Failure::Usage(m)) => assert!(m.contains(needle), "{args:?}: {m}"),
            other => panic!("{args:?}: {other:?}"),
        }
    };
    // More input files than MAX_SHARDS: refused before any is read (none of these exist).
    let many: Vec<String> = (0..=MAX_SHARDS)
        .map(|i| {
            dir.join(format!("absent-{i}.json"))
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let mut args = vec!["join", "--out", out_s.as_str()];
    args.extend(many.iter().map(String::as_str));
    usage(&args, "input files");
    // A repeated flag is a usage error, never "first wins".
    let mut args = vec!["join", "--out", out_s.as_str(), "--out", "other.json"];
    args.extend(files.iter().map(String::as_str));
    usage(&args, "--out given more than once");
    let mut args = vec![
        "join",
        "--out",
        out_s.as_str(),
        "--expect-run-id",
        "1",
        "--expect-run-id",
        "2",
    ];
    args.extend(files.iter().map(String::as_str));
    usage(&args, "--expect-run-id given more than once");
    usage(
        &["run", "--tier", "full", "--shard", "0/4", "--shard", "1/4"],
        "--shard given more than once",
    );
    // `--out` naming an input is refused, spelled any way that reaches the same file.
    let mut args = vec!["join", "--out", files[0].as_str()];
    args.extend(files.iter().map(String::as_str));
    usage(&args, "one of the input files");
    // An existing `--out` is never overwritten.
    std::fs::write(&out, "precious").unwrap();
    let mut args = vec!["join", "--out", out_s.as_str()];
    args.extend(files.iter().map(String::as_str));
    usage(&args, "already exists");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "precious");
}
