//! VER04j WX-E evidence generation, tested the way VER04h tests the v0 generator:
//! by *compiling and running* what it emits.  The generated harness
//! (`tests/generated/wxv_evidence.rs`) embeds the checked-in table copies
//! (`tests/generated/tables/*.tfv`, drift-tested by `tests/wxv_regen.rs`) and is
//! run against a tiny subject (`wxv_subject`): a SHA-256 export and a
//! first-byte-or-EMPTY export.
//!
//! Sections: the whole suite and every negative control (including the `table`
//! kind); the manifest entry of a table; sabotage (every fault fails a row of the
//! shipped table); the runner's integrity checks (a flipped byte, a row-count
//! mismatch, odd hex, a duplicate id, a missing or extra field, an out-of-range
//! value, a wrong-width `Bytes<N>`); and the differential run against Python's
//! `hashlib` over the same rows.

use std::collections::BTreeSet;
use std::io::Write;
use std::process::{Command, Stdio};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::tfv;
use coding_adventures_test_bootstrap::Verdict;

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    /// The subject under test.
    pub mod wxv_subject {
        pub fn digest(msg: &[u8]) -> [u8; 32] {
            coding_adventures_sha256::sha256(msg)
        }
        #[derive(Debug)]
        pub enum EmptyErr {
            Empty,
        }
        pub fn first_byte(msg: &[u8]) -> Result<u8, EmptyErr> {
            msg.first().copied().ok_or(EmptyErr::Empty)
        }
    }
    include!("generated/wxv_evidence.rs");

    pub fn digests_copy() -> &'static [u8] {
        TABLE_SYNTHETIC_DIGESTS_BYTES
    }
    pub fn digests_pin() -> &'static str {
        TABLE_SYNTHETIC_DIGESTS_SHA256
    }
    pub fn digests_columns() -> &'static [(&'static str, Ty)] {
        TABLE_SYNTHETIC_DIGESTS_COLUMNS
    }
    pub fn results_columns() -> &'static [(&'static str, Ty)] {
        TABLE_SYNTHETIC_RESULTS_COLUMNS
    }
    pub fn digests_holds(env: &[V], fault: Fault) -> bool {
        table_synthetic_digests_holds(env, fault)
    }
    pub fn results_holds(env: &[V], fault: Fault) -> bool {
        table_synthetic_results_holds(env, fault)
    }
}
use harness::{Fault, V};

include!("support/wxv_runner.rs");

fn is_fail(v: &Verdict, id: &str) -> bool {
    matches!(v, Verdict::Fail { failure_id, .. } if *failure_id == id)
}

fn message_of(v: &Verdict) -> String {
    match v {
        Verdict::Fail { message, .. } => message.clone(),
        Verdict::Pass => String::new(),
    }
}

/// Run the digests table over `bytes` as if it were the checked-in copy, with a
/// pin computed from `bytes` (so only the *content* checks can object) unless
/// `pin` says otherwise.
fn run_digests(bytes: &[u8], pin: Option<&str>, rows: usize) -> Verdict {
    let own = sha256_hex(bytes);
    harness::run_table(
        "synthetic_digests",
        bytes,
        pin.unwrap_or(&own),
        rows,
        harness::digests_columns(),
        harness::digests_holds,
        Fault::None,
    )
    .0
}

const DIGESTS_ROWS: usize = 23;

// ---------------------------------------------------------------------------
// The whole generated suite, including every negative control
// ---------------------------------------------------------------------------

#[test]
fn the_full_generated_suite_passes_and_every_fault_is_caught_by_the_tables() {
    let run = run_once();
    let failures: Vec<String> = run
        .outcomes
        .iter()
        .filter(|o| !o.passed())
        .map(|o| format!("{}: {}", o.id, message(o)))
        .collect();
    assert!(failures.is_empty(), "failures:\n{}", failures.join("\n"));
    let ids: BTreeSet<&str> = run.outcomes.iter().map(|o| o.id.as_str()).collect();
    // One test per table, one per (kind, op, fault) control.
    for id in [
        "warrant/wxv/table/synthetic_digests",
        "warrant/wxv/table/synthetic_results",
        "warrant/wxv/negctl/table/digest/flip-result-bit",
        "warrant/wxv/negctl/table/digest/drop-last-result-byte",
        "warrant/wxv/negctl/table/first_byte/missing-error",
        "warrant/wxv/negctl/example/digest/flip-result-bit",
        "warrant/wxv/negctl/example/digest/drop-last-result-byte",
        "warrant/wxv/negctl/exhaustive/first_byte/missing-error",
    ] {
        assert!(ids.contains(id), "no test {id}");
    }
}

#[test]
fn a_table_row_is_a_case_and_the_manifest_has_one_entry_per_table() {
    let compact = run_once().manifest.replace("\": ", "\":");
    let m = &compact;
    assert!(m.contains("\"kind\":\"table\""), "{m}");
    let entry = |id: &str| -> String {
        let at = m.find(id).unwrap_or_else(|| panic!("no entry {id}"));
        let start = m[..at].rfind('{').unwrap();
        let end = at + m[at..].find('}').unwrap();
        m[start..=end].to_string()
    };
    let d = entry("\"warrant/wxv/table/synthetic_digests\"");
    assert!(d.contains("\"kind\":\"table\""), "{d}");
    assert!(d.contains("\"cases\":23"), "{d}");
    assert!(d.contains("\"rows_pinned\":23"), "{d}");
    assert!(d.contains(&format!("\"table_sha256\":\"{}\"", harness::digests_pin())), "{d}");
    assert!(d.contains("\"verdict\":\"pass\""), "{d}");
    assert!(d.contains("\"op\":\"digest\"") && d.contains("\"claim\":\"synthetic_digests\""), "{d}");
    assert!(!d.contains("failed_ids"), "a passing table lists no failures: {d}");
    let r = entry("\"warrant/wxv/table/synthetic_results\"");
    assert!(r.contains("\"cases\":8") && r.contains("\"rows_pinned\":8"), "{r}");
    // The table's own pin is the SHA-256 of the checked-in copy.
    assert_eq!(sha256_hex(harness::digests_copy()), harness::digests_pin());
    // One entry per table, not one per row.
    assert_eq!(m.matches("\"kind\":\"table\"").count(), 2, "{m}");
    assert!(!m.contains("vectors/synthetic_digests/len-"), "{m}");
}

// ---------------------------------------------------------------------------
// Sabotage: every fault fails at least one row of the shipped table
// ---------------------------------------------------------------------------

#[test]
fn every_fault_fails_rows_of_the_shipped_tables() {
    // The generated runner deliberately keeps NO failed row ids when the `CI`
    // environment variable is set (the same log-size and data-exposure choice
    // that prints `repro-fnv` hashes instead of counterexamples there). The
    // counts and verdicts are the same either way; only the ids differ, so the
    // test checks whichever behaviour applies to the environment it runs in.
    // It reads `CI` rather than setting it: `std::env::set_var` races with the
    // other tests of this binary, which run on parallel threads.
    let in_ci = std::env::var("CI").is_ok();
    for (fault, failing) in [(Fault::FlipResultBit, 23), (Fault::DropLastResultByte, 23)] {
        let (v, stats) = harness::tab_synthetic_digests(fault);
        assert!(is_fail(&v, "not-deep-equal"), "{fault:?}: {v:?}");
        assert_eq!(stats.failed, failing, "{fault:?}");
        let msg = message_of(&v);
        assert!(msg.contains("23 of 23 rows failed"), "{msg}");
        if in_ci {
            assert!(stats.failed_ids.is_empty(), "CI keeps no row ids: {:?}", stats.failed_ids);
            assert!(!msg.contains("vectors/synthetic_digests/"), "CI names no rows: {msg}");
        } else {
            assert_eq!(stats.failed_ids.len(), 16, "the first 16 ids are kept");
            assert_eq!(stats.failed_ids[0], "vectors/synthetic_digests/len-0");
            assert!(msg.contains("vectors/synthetic_digests/len-0"), "{msg}");
        }
    }
    // `missing-error` fails the one row whose answer is an error.
    let (v, stats) = harness::tab_synthetic_results(Fault::MissingError);
    assert!(is_fail(&v, "not-deep-equal"), "{v:?}");
    assert_eq!(stats.failed, 1);
    if in_ci {
        assert!(stats.failed_ids.is_empty(), "{:?}", stats.failed_ids);
    } else {
        assert_eq!(stats.failed_ids, vec!["vectors/synthetic_results/case-0".to_string()]);
    }
    // No fault, no failure; and the honest run is what the manifest records.
    assert!(matches!(harness::tab_synthetic_digests(Fault::None).0, Verdict::Pass));
    assert!(matches!(harness::tab_synthetic_results(Fault::None).0, Verdict::Pass));
}

// ---------------------------------------------------------------------------
// The runner's integrity checks (the section 5.7 negatives)
// ---------------------------------------------------------------------------

#[test]
fn the_shipped_copy_runs_and_a_flipped_byte_is_corpus_malformed() {
    let copy = harness::digests_copy();
    assert!(matches!(run_digests(copy, Some(harness::digests_pin()), DIGESTS_ROWS), Verdict::Pass));
    // One flipped byte in the data: the pin no longer matches, no row is used.
    let mut bad = copy.to_vec();
    let at = bad.iter().position(|b| *b == b'e').unwrap();
    bad[at] = b'f';
    let v = run_digests(&bad, Some(harness::digests_pin()), DIGESTS_ROWS);
    assert!(is_fail(&v, "corpus-malformed"), "{v:?}");
    assert!(message_of(&v).contains("sha256 pin"), "{v:?}");
}

#[test]
fn a_row_count_off_by_one_in_either_direction_is_corpus_malformed() {
    let copy = harness::digests_copy();
    for rows in [DIGESTS_ROWS - 1, DIGESTS_ROWS + 1] {
        let v = run_digests(copy, None, rows);
        assert!(is_fail(&v, "corpus-malformed"), "{rows}: {v:?}");
        assert!(message_of(&v).contains("pinned"), "{v:?}");
    }
}

fn digests_text() -> String {
    String::from_utf8(harness::digests_copy().to_vec()).unwrap()
}

#[test]
fn malformed_tfv_is_corpus_malformed_before_any_row_runs() {
    let text = digests_text();
    let first_row = text.lines().nth(2).unwrap().to_string();
    let cases: Vec<(&str, String)> = vec![
        // odd-length hex
        ("odd hex", text.replacen("Kmsg X01 ", "Kmsg X012 ", 1)),
        // a duplicate row id
        ("duplicate id", format!("{text}{first_row}\n")),
        // no header
        ("no header", text.replacen("tfv 1\n", "", 1)),
        // trailing garbage on a record
        ("trailing", text.replacen("\nlen-1 ", " x\nlen-1 ", 1)),
        // uppercase hex
        ("uppercase", text.replacen("Kmsg X01 ", "Kmsg X0A ", 1)),
    ];
    for (what, bad) in cases {
        let rows = bad.lines().filter(|l| l.contains(" R2 ")).count();
        let v = run_digests(bad.as_bytes(), None, rows);
        assert!(is_fail(&v, "corpus-malformed"), "{what}: {v:?}");
    }
}

#[test]
fn columns_must_be_exactly_the_declared_ones_and_fit_their_types() {
    let text = digests_text();
    let md1 = "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a";
    let cases: Vec<(&str, String, &str)> = vec![
        // a missing field (the record has one field, the table two)
        (
            "missing field",
            text.replacen(&format!("len-1 R2 Kmsg X01 Kmd X{md1}"), "len-1 R1 Kmsg X01", 1),
            "declared columns",
        ),
        // an extra field
        (
            "extra field",
            text.replacen("len-1 R2 Kmsg X01 Kmd", "len-1 R3 Kmsg X01 Kextra I1 Kmd", 1),
            "declared columns",
        ),
        // a misnamed field
        (
            "wrong field name",
            text.replacen("len-1 R2 Kmsg X01 Kmd", "len-1 R2 Kmsg X01 Kdigest", 1),
            "missing a column",
        ),
        // a value of the wrong kind
        (
            "wrong kind",
            text.replacen("Kmsg X01 Kmd", "Kmsg I1 Kmd", 1),
            "does not fit",
        ),
        // Bytes<32> of the wrong width
        (
            "wrong width",
            text.replacen(&format!("Kmd X{md1}"), "Kmd X4b", 1),
            "does not fit",
        ),
    ];
    for (what, bad, reason) in cases {
        assert_ne!(bad, text, "{what}: the edit must change the table");
        let v = run_digests(bad.as_bytes(), None, DIGESTS_ROWS);
        assert!(is_fail(&v, "corpus-malformed"), "{what}: {v:?}");
        assert!(message_of(&v).contains(reason), "{what}: {v:?}");
    }
}

#[test]
fn a_value_outside_a_columns_range_is_refused_not_narrowed() {
    // The results table's `out` is a Result<U8, _>: 256 does not fit a U8.
    let ok256 = format!("tfv 1\nr R2 Kmsg X01 Kout M1 S{} I256\n", hex(b"ok"));
    let v = harness::run_table(
        "synthetic_results",
        ok256.as_bytes(),
        &sha256_hex(ok256.as_bytes()),
        1,
        harness::results_columns(),
        harness::results_holds,
        Fault::None,
    )
    .0;
    assert!(is_fail(&v, "corpus-malformed"), "{v:?}");
    assert!(message_of(&v).contains("out of range"), "{v:?}");
    // An unknown error case, an unknown Result key and a two-entry map.
    for bad in [
        format!("tfv 1\nr R2 Kmsg X Kout M1 S{} S{}\n", hex(b"err"), hex(b"NOPE")),
        format!("tfv 1\nr R2 Kmsg X Kout M1 S{} I1\n", hex(b"maybe")),
        "tfv 1\nr R2 Kmsg X Kout I1\n".to_string(),
    ] {
        let v = harness::run_table(
            "synthetic_results",
            bad.as_bytes(),
            &sha256_hex(bad.as_bytes()),
            1,
            harness::results_columns(),
            harness::results_holds,
            Fault::None,
        )
        .0;
        assert!(is_fail(&v, "corpus-malformed"), "{bad}: {v:?}");
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn a_non_utf8_copy_is_corpus_malformed_and_never_parsed() {
    let bytes = [0xffu8, 0xfe, 0x00];
    let v = run_digests(&bytes, None, 1);
    assert!(is_fail(&v, "corpus-malformed"), "{v:?}");
    assert!(message_of(&v).contains("UTF-8"), "{v:?}");
}

#[test]
fn a_panicking_row_fails_instead_of_aborting_the_table() {
    // A subject that panics on a row (it is not trusted to be total) fails that
    // row; the table still runs the others and reports them all.
    let text: String = digests_text().lines().take(4).collect::<Vec<_>>().join("\n") + "\n";
    let v = harness::run_table(
        "synthetic_digests",
        text.as_bytes(),
        &sha256_hex(text.as_bytes()),
        2,
        harness::digests_columns(),
        |_env: &[V], _f: Fault| -> bool { panic!("the subject panicked") },
        Fault::None,
    )
    .0;
    assert!(is_fail(&v, "not-deep-equal"), "{v:?}");
    assert!(message_of(&v).contains("2 of 2 rows failed"), "{v:?}");
}

#[test]
fn rows_decode_to_the_values_the_holds_expression_sees() {
    // The first row of the shipped table, decoded by the runner's own decoder
    // (via `holds`): the digest of the empty message.
    let records = tfv::parse_file(&digests_text()).unwrap();
    assert_eq!(records.len(), DIGESTS_ROWS);
    assert_eq!(records[0].id, "len-0");
    let msg = records[0].field("msg").unwrap().as_bytes().unwrap().to_vec();
    let md = records[0].field("md").unwrap().as_bytes().unwrap().to_vec();
    assert!(harness::digests_holds(&[V::Bytes(msg), V::Bytes(md.clone())], Fault::None));
    assert!(!harness::digests_holds(&[V::Bytes(vec![1]), V::Bytes(md)], Fault::None));
}

// ---------------------------------------------------------------------------
// Differential: the table's data against Python's hashlib, and the runner's
// verdict against the same rows
// ---------------------------------------------------------------------------

#[test]
fn the_pinned_digests_agree_with_python_hashlib_over_the_same_messages() {
    let records = tfv::parse_file(&digests_text()).unwrap();
    let messages: Vec<Vec<u8>> = records
        .iter()
        .map(|r| r.field("msg").unwrap().as_bytes().unwrap().to_vec())
        .collect();
    // One Python process over every message (stdin: hex lines, stdout: digests).
    let mut child = Command::new("python3")
        .args(["-I", "-c", "import hashlib,sys\nfor line in sys.stdin:\n    print(hashlib.sha256(bytes.fromhex(line.strip())).hexdigest())"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("python3 is required for the differential test");
    let mut stdin = child.stdin.take().unwrap();
    for m in &messages {
        writeln!(stdin, "{}", hex(m)).unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let python: Vec<String> = String::from_utf8(out.stdout).unwrap().lines().map(String::from).collect();
    assert_eq!(python.len(), messages.len());
    for ((rec, m), py) in records.iter().zip(&messages).zip(&python) {
        // The data is hashlib's, and the subject (an independent Rust
        // implementation) says the same: three-way agreement, row by row.
        let md = hex(rec.field("md").unwrap().as_bytes().unwrap());
        assert_eq!(&md, py, "row {} disagrees with hashlib", rec.id);
        assert_eq!(hex(&harness::wxv_subject::digest(m)), *py, "row {}", rec.id);
    }
    // ... and so the runner's verdict on the shipped rows is `pass`.
    assert!(matches!(harness::tab_synthetic_digests(Fault::None).0, Verdict::Pass));
}
