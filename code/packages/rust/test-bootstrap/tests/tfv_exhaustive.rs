//! # TFV v1: bounded-exhaustive documents, checked three independent ways
//!
//! The strictness of TFV is its whole point ("there is exactly one way to
//! write each value"), so the parser is tested here against *every* document
//! of a small grammar rather than against hand-picked examples.
//!
//! ## The domains (sizes are asserted, so the report can quote them)
//!
//! | domain | what is enumerated                                              | docs   |
//! |--------|-----------------------------------------------------------------|--------|
//! | A1     | every sequence of 1..=3 tokens from a 28-token alphabet of good and bad value forms, as the value of `r R1 Kv ...` | 22 764 |
//! | A2     | every sequence of exactly 4 tokens from a 12-token alphabet     | 20 736 |
//! | A3     | `r R2 Kv <tok> Kw <tok>` for every token pair of the 28-alphabet | 784    |
//! | A4     | `M2` maps over 14 keys x 14 keys (duplicate-key detection)      | 196    |
//! | B      | 8 headers x every sequence of 0..=3 lines from a 21-line pool x 3 terminators | 233 376 |
//! | C      | `S`/`X` payloads: all byte strings of length <= 2, boundary-byte strings of length 3 and 4, uppercase and odd-length hex | 81 892 |
//! | D      | counts and integers: every string of length <= 4 over `-01 9+a` in three contexts | 4 665 |
//! | E      | deep equality over a 470-value universe: 220 900 ordered pairs  | 220 900 |
//!
//! ## The three checks
//!
//! 1. **Self-consistency (always on).** Whatever the parser accepts must be
//!    *canonical*: re-encoding the parsed records reproduces, byte for byte,
//!    the record lines of the input. That is the spec's "exactly one way to
//!    write each value" turned into an executable property. The parser must
//!    also never panic, and domain D is checked against a hand-written
//!    oracle.
//! 2. **Algebraic laws (always on).** Deep equality is reflexive, symmetric
//!    and transitive over the whole domain E universe, and NaN/-0/map-order
//!    behave as TF01 §3.1 says.
//! 3. **Differential (when `TFV_DIFFERENTIAL_OUT` is set).** The verdict for
//!    every document and the whole domain E matrix are written to that file;
//!    `code/scripts/tfv_differential.py` replays them through the
//!    independent Python reference (`code/scripts/tfv.py`) and fails on any
//!    disagreement. The BUILD file runs both steps.

use std::collections::HashMap;
use std::io::Write;

use coding_adventures_test_bootstrap::prng::fnv1a64;
use coding_adventures_test_bootstrap::tfv::{encode, parse_file, values_equal, Record, Value};

// ---------------------------------------------------------------------------
// Dump file (differential input)
// ---------------------------------------------------------------------------

struct Dump {
    out: Option<std::io::BufWriter<std::fs::File>>,
}

impl Dump {
    fn open() -> Dump {
        let out = std::env::var_os("TFV_DIFFERENTIAL_OUT").map(|path| {
            let file = std::fs::File::create(&path)
                .unwrap_or_else(|e| panic!("cannot create {path:?}: {e}"));
            std::io::BufWriter::new(file)
        });
        Dump { out }
    }

    fn line(&mut self, text: &str) {
        if let Some(out) = self.out.as_mut() {
            writeln!(out, "{text}").expect("dump write");
        }
    }

    fn finish(mut self) {
        if let Some(out) = self.out.as_mut() {
            out.flush().expect("dump flush");
        }
    }
}

fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Canonical re-encoding of accepted records (the strictness property)
// ---------------------------------------------------------------------------

fn canonical_lines(records: &[Record]) -> Vec<String> {
    records
        .iter()
        .map(|record| {
            let mut line = format!("{} R{}", record.id, record.fields.len());
            for (name, value) in &record.fields {
                line.push_str(&format!(" K{name} {}", encode(value)));
            }
            line
        })
        .collect()
}

/// Parse `text`; if accepted, assert canonicality and return its digest.
fn classify(text: &str) -> Option<u64> {
    let records = parse_file(text).ok()?;
    let lines = canonical_lines(&records);
    // Every record line of the input appears verbatim, in order, among the
    // document's lines: there is no second spelling of an accepted record.
    let mut remaining = lines.iter();
    let mut want = remaining.next();
    for line in text.split('\n') {
        if let Some(expected) = want {
            if line == expected {
                want = remaining.next();
            }
        }
    }
    assert!(
        want.is_none(),
        "accepted a non-canonical document {text:?}; canonical lines {lines:?}"
    );
    // Re-parsing the canonical form gives the same canonical form.
    let again = format!("tfv 1\n{}\n", lines.join("\n"));
    let reparsed = parse_file(&again).unwrap_or_else(|e| panic!("canonical form rejected: {e}"));
    assert_eq!(canonical_lines(&reparsed), lines);
    Some(fnv1a64(lines.join("\n").as_bytes()))
}

fn check_docs(dump: &mut Dump, docs: &[String]) -> usize {
    let mut accepted = 0;
    for text in docs {
        match classify(text) {
            Some(digest) => {
                accepted += 1;
                dump.line(&format!("DOC\t{}\tok\t{digest:016x}", hex(text)));
            }
            None => dump.line(&format!("DOC\t{}\terr\t-", hex(text))),
        }
    }
    accepted
}

fn doc(body: &str) -> String {
    format!("tfv 1\n{body}\n")
}

// ---------------------------------------------------------------------------
// Domain A: value-token sequences
// ---------------------------------------------------------------------------

const ALPHABET_28: [&str; 28] = [
    "N",
    "T",
    "F",
    "I0",
    "I-0",
    "I7",
    "I-7",
    "I07",
    "I-",
    "I",
    "D8000000000000000",
    "D80000000000000",
    "D7FF8000000000000",
    "X",
    "X00",
    "X0",
    "XAB",
    "S",
    "S41",
    "S80",
    "Sc3a9",
    "L0",
    "L1",
    "L2",
    "L01",
    "M0",
    "M1",
    "M2",
];

const ALPHABET_12: [&str; 12] = [
    "N", "T", "I7", "I-0", "X00", "S41", "Sc3a9", "L0", "L1", "L2", "M1", "M2",
];

fn sequences(alphabet: &[&str], min_len: usize, max_len: usize) -> Vec<String> {
    let mut all = Vec::new();
    let mut frontier: Vec<Vec<&str>> = vec![Vec::new()];
    for len in 1..=max_len {
        let mut next = Vec::new();
        for prefix in &frontier {
            for token in alphabet {
                let mut seq = prefix.clone();
                seq.push(token);
                next.push(seq);
            }
        }
        if len >= min_len {
            all.extend(next.iter().map(|seq| seq.join(" ")));
        }
        frontier = next;
    }
    all
}

/// An oracle for domain A that shares no code with the parser: a value is a
/// good atom, or `L<n>` / `M<n>` (n in 0..=2) followed by n (or 2n) values,
/// and the whole token sequence must be exactly one value.
fn oracle_value_sequence(sequence: &str) -> bool {
    fn value(tokens: &[&str], at: usize) -> Option<usize> {
        const ATOMS: [&str; 12] = [
            "N",
            "T",
            "F",
            "I0",
            "I7",
            "I-7",
            "D8000000000000000",
            "X",
            "X00",
            "S",
            "S41",
            "Sc3a9",
        ];
        let token = *tokens.get(at)?;
        if ATOMS.contains(&token) {
            return Some(at + 1);
        }
        let (items_per_entry, count) = match token {
            "L0" | "M0" => return Some(at + 1),
            "L1" => (1, 1),
            "L2" => (1, 2),
            "M1" => (2, 1),
            "M2" => (2, 2),
            _ => return None,
        };
        let mut position = at + 1;
        for _ in 0..count * items_per_entry {
            position = value(tokens, position)?;
        }
        Some(position)
    }
    let tokens: Vec<&str> = sequence.split(' ').collect();
    // Maps with two deep-equal keys are rejected; with this alphabet that
    // means M2 whose two keys are the same token.
    let mut duplicate_keys = false;
    for (i, token) in tokens.iter().enumerate() {
        if *token == "M2" {
            let mut p = i + 1;
            let first = value(&tokens, p);
            if let Some(after_first) = first {
                if let Some(after_value) = value(&tokens, after_first) {
                    p = after_value;
                    if let Some(after_key2) = value(&tokens, p) {
                        if tokens[i + 1..after_first] == tokens[after_value..after_key2] {
                            duplicate_keys = true;
                        }
                    }
                }
            }
        }
    }
    value(&tokens, 0) == Some(tokens.len()) && !duplicate_keys
}

// ---------------------------------------------------------------------------
// Domain B: document structure
// ---------------------------------------------------------------------------

const HEADERS: [&str; 8] = [
    "tfv 1",
    "tfv 1 ",
    "tfv  1",
    "TFV 1",
    "tfv 2",
    "\u{feff}tfv 1",
    "",
    "tfv 1\r",
];

const LINE_POOL: [&str; 21] = [
    "",
    "# c",
    "#",
    "a R0",
    "a R1 Kx T",
    "b R0",
    "a R0 ",
    " a R0",
    "a  R0",
    "a R0\r",
    "a R2 Kx T Kx F",
    "a R2 Kx T Ky F",
    "a R1 KX T",
    "a R1 Kx",
    "a R1 x T",
    "A.b/c-d_e R0",
    "a$ R0",
    "a R-1",
    "a R01",
    "a R1 Kx T extra",
    "# c\r",
];

fn structure_docs() -> Vec<String> {
    let mut bodies: Vec<Vec<&str>> = vec![Vec::new()];
    let mut frontier = bodies.clone();
    for _ in 0..3 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for line in LINE_POOL {
                let mut body = prefix.clone();
                body.push(line);
                next.push(body);
            }
        }
        bodies.extend(next.iter().cloned());
        frontier = next;
    }
    let mut docs = Vec::new();
    for header in HEADERS {
        for body in &bodies {
            let mut lines = vec![header];
            lines.extend(body.iter().copied());
            let lf = lines.join("\n");
            docs.push(format!("{lf}\n")); // final LF
            docs.push(lf.clone()); // no final newline
            docs.push(lines.join("\r\n") + "\r\n"); // CRLF throughout
        }
    }
    docs
}

// ---------------------------------------------------------------------------
// Domain C: text and bytes payloads
// ---------------------------------------------------------------------------

fn payload_docs() -> Vec<(String, bool)> {
    let mut docs: Vec<(String, bool)> = Vec::new();
    let hex_of = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    // The oracle for an `S` payload is the standard library's UTF-8 check
    // (the parser uses `String::from_utf8`; Python's decoder is the third
    // opinion in the differential run). The oracle for an `X` payload is
    // "even length and no uppercase hex letter".
    let s_doc = |bytes: &[u8]| {
        (
            doc(&format!("r R1 Kv S{}", hex_of(bytes))),
            std::str::from_utf8(bytes).is_ok(),
        )
    };
    docs.push(s_doc(&[]));
    for a in 0..=255u8 {
        docs.push(s_doc(&[a]));
        for b in 0..=255u8 {
            docs.push(s_doc(&[a, b]));
        }
    }
    docs.push((doc("r R1 Kv X"), true));
    for a in 0..=255u8 {
        docs.push((doc(&format!("r R1 Kv X{}", hex_of(&[a]))), true));
    }
    // Boundary bytes: lengths 3 and 4, where the multi-byte UTF-8 rules live.
    let b3 = [
        0x00u8, 0x41, 0x7f, 0x80, 0x8f, 0x90, 0x9f, 0xa0, 0xbf, 0xc0, 0xe0, 0xed, 0xef, 0xf0, 0xf4,
        0xf5, 0xff,
    ];
    for a in b3 {
        for b in b3 {
            for c in b3 {
                docs.push(s_doc(&[a, b, c]));
            }
        }
    }
    let b4 = [0x80u8, 0x8f, 0x90, 0xbf, 0xc0, 0xf0, 0xf4, 0xf5, 0x41];
    for a in b4 {
        for b in b4 {
            for c in b4 {
                for d in b4 {
                    docs.push(s_doc(&[a, b, c, d]));
                }
            }
        }
    }
    // Uppercase hex for every byte: accepted only when it has no letters.
    for a in 0..=255u8 {
        let digits = format!("{a:02X}");
        let has_letter = digits.bytes().any(|c| c.is_ascii_uppercase());
        docs.push((doc(&format!("r R1 Kv X{digits}")), !has_letter));
    }
    // Odd-length hex: every 1- and 3-digit string. Always rejected.
    for n in 0..16u32 {
        docs.push((doc(&format!("r R1 Kv X{n:x}")), false));
    }
    for n in 0..4096u32 {
        docs.push((doc(&format!("r R1 Kv X{n:03x}")), false));
    }
    docs
}

// ---------------------------------------------------------------------------
// Domain D: counts and integers
// ---------------------------------------------------------------------------

/// Hand-written oracle for `-?(0|[1-9][0-9]*)` except `-0`, within the digit limit.
fn oracle_canonical_int(s: &str) -> bool {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if s == "-0" || digits.is_empty() || digits.len() > 1000 {
        return false;
    }
    if digits == "0" {
        return true;
    }
    let mut chars = digits.chars();
    matches!(chars.next(), Some('1'..='9')) && chars.all(|c| c.is_ascii_digit())
}

fn oracle_count(s: &str) -> bool {
    oracle_canonical_int(s) && !s.starts_with('-')
}

fn digit_strings() -> Vec<String> {
    let alphabet = ['-', '0', '1', '9', '+', 'a'];
    let mut all = vec![String::new()];
    let mut frontier = vec![String::new()];
    for _ in 0..4 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for c in alphabet {
                next.push(format!("{prefix}{c}"));
            }
        }
        all.extend(next.iter().cloned());
        frontier = next;
    }
    all
}

// ---------------------------------------------------------------------------
// Domain E: deep-equality universe
// ---------------------------------------------------------------------------

const ATOMS: [&str; 14] = [
    "N",
    "T",
    "F",
    "I0",
    "I1",
    "D0000000000000000",
    "D8000000000000000",
    "D7ff8000000000000",
    "D7ff0000000000001",
    "Dfff8000000000000",
    "D7ff0000000000000",
    "X",
    "X00",
    "S",
];

fn universe() -> Vec<String> {
    let mut all: Vec<String> = ATOMS.iter().map(|s| (*s).to_owned()).collect();
    all.push("L0".into());
    for a in ATOMS {
        all.push(format!("L1 {a}"));
    }
    for a in ATOMS {
        for b in ATOMS {
            all.push(format!("L2 {a} {b}"));
        }
    }
    all.push("M0".into());
    for k in ATOMS {
        for v in ATOMS {
            all.push(format!("M1 {k} {v}"));
        }
    }
    let keys = ["N", "T", "D7ff8000000000000", "X"];
    let vals = ["N", "I0"];
    for k1 in keys {
        for k2 in keys {
            if k1 == k2 {
                continue;
            }
            for v1 in vals {
                for v2 in vals {
                    all.push(format!("M2 {k1} {v1} {k2} {v2}"));
                }
            }
        }
    }
    all
}

fn parse_value_text(tokens: &str) -> Value {
    let records = parse_file(&format!("tfv 1\nr R1 Kv {tokens}\n"))
        .unwrap_or_else(|e| panic!("universe member {tokens:?} rejected: {e}"));
    records
        .into_iter()
        .next()
        .and_then(|r| r.fields.into_iter().next())
        .map(|(_, v)| v)
        .expect("one field")
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

#[test]
fn bounded_exhaustive_documents_parse_canonically() {
    let mut dump = Dump::open();
    let mut total = 0usize;
    let mut accepted = 0usize;

    // A1
    let a1: Vec<String> = sequences(&ALPHABET_28, 1, 3)
        .iter()
        .map(|seq| doc(&format!("r R1 Kv {seq}")))
        .collect();
    assert_eq!(a1.len(), 22_764);
    let a1_ok = check_docs(&mut dump, &a1);
    for (text, seq) in a1.iter().zip(sequences(&ALPHABET_28, 1, 3)) {
        assert_eq!(
            parse_file(text).is_ok(),
            oracle_value_sequence(&seq),
            "{seq}"
        );
    }
    // A2
    let a2: Vec<String> = sequences(&ALPHABET_12, 4, 4)
        .iter()
        .map(|seq| doc(&format!("r R1 Kv {seq}")))
        .collect();
    assert_eq!(a2.len(), 20_736);
    let a2_ok = check_docs(&mut dump, &a2);
    for (text, seq) in a2.iter().zip(sequences(&ALPHABET_12, 4, 4)) {
        assert_eq!(
            parse_file(text).is_ok(),
            oracle_value_sequence(&seq),
            "{seq}"
        );
    }

    // A3
    let mut a3 = Vec::new();
    for x in ALPHABET_28 {
        for y in ALPHABET_28 {
            a3.push(doc(&format!("r R2 Kv {x} Kw {y}")));
        }
    }
    assert_eq!(a3.len(), 784);
    let a3_ok = check_docs(&mut dump, &a3);
    for x in ALPHABET_28 {
        for y in ALPHABET_28 {
            // Each field is a single token, so each must be a complete value.
            let want = oracle_value_sequence(x) && oracle_value_sequence(y);
            assert_eq!(parse_file(&doc(&format!("r R2 Kv {x} Kw {y}"))).is_ok(), want);
        }
    }

    // A4: M2 over 14 x 14 keys.
    let keys = [
        "N",
        "T",
        "F",
        "I0",
        "I7",
        "X",
        "S",
        "L0",
        "L1 N",
        "L1 T",
        "D0000000000000000",
        "D8000000000000000",
        "D7ff8000000000000",
        "D7ff0000000000001",
    ];
    let mut a4 = Vec::new();
    for k1 in keys {
        for k2 in keys {
            a4.push(doc(&format!("r R1 Kv M2 {k1} N {k2} N")));
        }
    }
    assert_eq!(a4.len(), 196);
    let a4_ok = check_docs(&mut dump, &a4);
    // Hand count: a map is accepted iff its two keys are NOT deep-equal.
    // Equal-key pairs: 14 identical + {qNaN, sNaN} both orders = 16 rejected.
    assert_eq!(a4_ok, 196 - 16, "duplicate-key detection");

    // B
    let b = structure_docs();
    assert_eq!(b.len(), 8 * (1 + 21 + 441 + 9261) * 3);
    let b_ok = check_docs(&mut dump, &b);

    // C
    let c = payload_docs();
    assert_eq!(c.len(), 81_892);
    let c_texts: Vec<String> = c.iter().map(|(text, _)| text.clone()).collect();
    let c_ok = check_docs(&mut dump, &c_texts);
    let c_want = c.iter().filter(|(_, ok)| *ok).count();
    assert_eq!(c_ok, c_want, "payload oracle disagrees with the parser");
    for (text, want) in &c {
        assert_eq!(parse_file(text).is_ok(), *want, "{text:?}");
    }

    // D
    let strings = digit_strings();
    assert_eq!(strings.len(), 1 + 6 + 36 + 216 + 1296);
    let mut d_docs = Vec::new();
    let mut d_oracle = 0;
    for s in &strings {
        d_docs.push(doc(&format!("r R{s} Kv N")));
        d_docs.push(doc(&format!("r R1 Kv I{s}")));
        d_docs.push(doc(&format!("r R1 Kv L{s} N N N")));
        // Oracle, written independently of the parser:
        //   r R<s> Kv N      accepted iff s is exactly "1" (one field, present)
        //   r R1 Kv I<s>     accepted iff <s> is a canonical integer
        //   r R1 Kv L<s> N N N  accepted iff s is exactly "3"; the alphabet
        //                    -,0,1,9,+,a never spells "3", so never accepted
        if s == "1" {
            d_oracle += 1;
        }
        if oracle_canonical_int(s) {
            d_oracle += 1;
        }
    }
    assert_eq!(d_docs.len(), 4_665);
    let d_ok = check_docs(&mut dump, &d_docs);
    assert_eq!(d_ok, d_oracle, "digit-string oracle disagrees with the parser");
    for s in &strings {
        assert_eq!(
            parse_file(&doc(&format!("r R1 Kv I{s}"))).is_ok(),
            oracle_canonical_int(s),
            "I{s}"
        );
        assert_eq!(
            parse_file(&doc(&format!("r R{s} Kv N"))).is_ok(),
            oracle_count(s) && s == "1",
            "R{s}"
        );
    }

    total += a1.len() + a2.len() + a3.len() + a4.len() + b.len() + c_texts.len() + d_docs.len();
    accepted += a1_ok + a2_ok + a3_ok + a4_ok + b_ok + c_ok + d_ok;
    dump.finish();
    // Reported in the audit; asserted so a silent shrink of the domain fails.
    assert_eq!(total, 22_764 + 20_736 + 784 + 196 + 233_376 + 81_892 + 4_665);
    eprintln!("tfv exhaustive: {total} documents, {accepted} accepted");
}

#[test]
fn deep_equality_is_an_equivalence_over_the_whole_universe() {
    let tokens = universe();
    assert_eq!(tokens.len(), 470);
    let values: Vec<Value> = tokens.iter().map(|t| parse_value_text(t)).collect();
    let n = values.len();
    let mut matrix = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = values_equal(&values[i], &values[j]);
        }
    }
    for i in 0..n {
        assert!(matrix[i][i], "not reflexive: {}", tokens[i]);
        for j in 0..n {
            assert_eq!(matrix[i][j], matrix[j][i], "{} vs {}", tokens[i], tokens[j]);
            if matrix[i][j] {
                // Equal values are interchangeable: identical rows. This
                // gives transitivity over the whole universe.
                assert_eq!(matrix[i], matrix[j], "{} ~ {}", tokens[i], tokens[j]);
            }
        }
    }
    // Specific TF01 §3.1 facts, spelled out.
    let at = |t: &str| tokens.iter().position(|x| x == t).expect(t);
    let eq = |a: &str, b: &str| matrix[at(a)][at(b)];
    assert!(eq("D7ff8000000000000", "D7ff0000000000001"), "NaN = NaN");
    assert!(eq("D7ff8000000000000", "Dfff8000000000000"), "any NaN = any NaN");
    assert!(!eq("D0000000000000000", "D8000000000000000"), "+0 != -0");
    assert!(!eq("D7ff8000000000000", "D7ff0000000000000"), "NaN != inf");
    assert!(!eq("X", "S"), "bytes are not text");
    assert!(!eq("I0", "D0000000000000000"), "int is not float");
    assert!(!eq("N", "F"), "null is not false");
    assert!(!eq("L0", "M0"), "empty list is not empty map");
    assert!(eq("M2 N N T I0", "M2 T I0 N N"), "map order is irrelevant");
    assert!(!eq("L2 N T", "L2 T N"), "list order matters");
    assert!(eq("M1 D7ff8000000000000 N", "M1 D7ff0000000000001 N"), "NaN keys");

    // Differential: emit the matrix for the Python reference.
    let mut dump = Dump::open_append();
    for token in &tokens {
        dump.line(&format!("UNI\t{}", hex(token)));
    }
    for row in &matrix {
        let bits: String = row.iter().map(|&b| if b { '1' } else { '0' }).collect();
        dump.line(&format!("ROW\t{bits}"));
    }
    dump.finish();
    assert_eq!(n * n, 220_900);
}

impl Dump {
    /// The equality matrix goes to a sibling file `<path>.matrix`: libtest runs
    /// the two differential tests in parallel, so they must not share a file.
    fn open_append() -> Dump {
        let out = std::env::var_os("TFV_DIFFERENTIAL_OUT").map(|path| {
            let mut path = path;
            path.push(".matrix");
            let file = std::fs::File::create(&path)
                .unwrap_or_else(|e| panic!("cannot create {path:?}: {e}"));
            std::io::BufWriter::new(file)
        });
        Dump { out }
    }
}

// ---------------------------------------------------------------------------
// A malformed-input corpus: each line is a (reason, text) that must be rejected
// ---------------------------------------------------------------------------

#[test]
fn malformed_corpus_is_rejected() {
    let big_count = "99999999999999999999999999";
    let nested_65 = format!("tfv 1\nr R1 Kv {}N\n", "L1 ".repeat(65));
    let int_1001 = format!("tfv 1\nr R1 Kv I1{}\n", "0".repeat(1000));
    let corpus: Vec<(&str, String)> = vec![
        ("empty file", String::new()),
        ("only a newline", "\n".into()),
        ("no header", "r R0\n".into()),
        ("header with trailing space", "tfv 1 \nr R0\n".into()),
        ("header uppercase", "TFV 1\n".into()),
        ("header version 2", "tfv 2\n".into()),
        ("header version 01", "tfv 01\n".into()),
        ("BOM before header", "\u{feff}tfv 1\n".into()),
        ("CRLF header", "tfv 1\r\nr R0\r\n".into()),
        ("CR in record", "tfv 1\nr R0\r\n".into()),
        ("lone CR line", "tfv 1\n\r\n".into()),
        ("blank line between header and nothing, then junk", "tfv 1\n\njunk\n".into()),
        ("leading space", "tfv 1\n r R0\n".into()),
        ("trailing space", "tfv 1\nr R0 \n".into()),
        ("double space", "tfv 1\nr  R0\n".into()),
        ("tab separator", "tfv 1\nr\tR0\n".into()),
        ("non-breaking space separator", "tfv 1\nr\u{a0}R0\n".into()),
        ("bad id char", "tfv 1\nr$ R0\n".into()),
        ("non-ASCII id", "tfv 1\nr\u{e9} R0\n".into()),
        ("empty-looking id", "tfv 1\n R0\n".into()),
        ("missing count", "tfv 1\nr\n".into()),
        ("count without R", "tfv 1\nr 0\n".into()),
        ("lowercase r count", "tfv 1\nr r0\n".into()),
        ("leading-zero count", "tfv 1\nr R01 Kv N\n".into()),
        ("negative count", "tfv 1\nr R-1\n".into()),
        ("plus count", "tfv 1\nr R+1 Kv N\n".into()),
        ("count 1 with 0 fields", "tfv 1\nr R1\n".into()),
        ("count 0 with field", "tfv 1\nr R0 Kv N\n".into()),
        ("count 2 with 1 field", "tfv 1\nr R2 Kv N\n".into()),
        ("huge count", format!("tfv 1\nr R{big_count} Kv N\n")),
        ("field without K", "tfv 1\nr R1 v N\n".into()),
        ("uppercase field", "tfv 1\nr R1 KV N\n".into()),
        ("digit-led field", "tfv 1\nr R1 K1 N\n".into()),
        ("empty field name", "tfv 1\nr R1 K N\n".into()),
        ("duplicate field", "tfv 1\nr R2 Kv N Kv T\n".into()),
        ("duplicate record id", "tfv 1\nr R0\nr R0\n".into()),
        ("duplicate id across a comment", "tfv 1\nr R0\n# x\nr R0\n".into()),
        ("missing value", "tfv 1\nr R1 Kv\n".into()),
        ("trailing tokens", "tfv 1\nr R1 Kv N N\n".into()),
        ("lowercase null", "tfv 1\nr R1 Kv n\n".into()),
        ("null with body", "tfv 1\nr R1 Kv N0\n".into()),
        ("bool word", "tfv 1\nr R1 Kv true\n".into()),
        ("int plus", "tfv 1\nr R1 Kv I+1\n".into()),
        ("int leading zero", "tfv 1\nr R1 Kv I007\n".into()),
        ("int negative zero", "tfv 1\nr R1 Kv I-0\n".into()),
        ("int double minus", "tfv 1\nr R1 Kv I--1\n".into()),
        ("int empty", "tfv 1\nr R1 Kv I\n".into()),
        ("int with underscore", "tfv 1\nr R1 Kv I1_0\n".into()),
        ("int with exponent", "tfv 1\nr R1 Kv I1e3\n".into()),
        ("int arabic-indic digit", "tfv 1\nr R1 Kv I\u{661}\n".into()),
        ("int over 1000 digits", int_1001),
        ("float short", "tfv 1\nr R1 Kv D0\n".into()),
        ("float long", "tfv 1\nr R1 Kv D00000000000000000\n".into()),
        ("float uppercase hex", "tfv 1\nr R1 Kv D7FF8000000000000\n".into()),
        ("float decimal", "tfv 1\nr R1 Kv D1.5\n".into()),
        ("bytes odd length", "tfv 1\nr R1 Kv X0\n".into()),
        ("bytes uppercase", "tfv 1\nr R1 Kv X0A\n".into()),
        ("bytes non-hex", "tfv 1\nr R1 Kv Xzz\n".into()),
        ("bytes 0x prefix", "tfv 1\nr R1 Kv X0x00\n".into()),
        ("text odd length", "tfv 1\nr R1 Kv S6\n".into()),
        ("text uppercase", "tfv 1\nr R1 Kv S4A\n".into()),
        ("text lone continuation", "tfv 1\nr R1 Kv S80\n".into()),
        ("text overlong NUL", "tfv 1\nr R1 Kv Sc080\n".into()),
        ("text surrogate", "tfv 1\nr R1 Kv Seda080\n".into()),
        ("text above U+10FFFF", "tfv 1\nr R1 Kv Sf4908080\n".into()),
        ("text truncated sequence", "tfv 1\nr R1 Kv Se282\n".into()),
        ("text 0xff", "tfv 1\nr R1 Kv Sff\n".into()),
        ("list short", "tfv 1\nr R1 Kv L2 N\n".into()),
        ("list long", "tfv 1\nr R1 Kv L1 N N\n".into()),
        ("list leading zero", "tfv 1\nr R1 Kv L01 N\n".into()),
        ("list no count", "tfv 1\nr R1 Kv L\n".into()),
        ("list negative count", "tfv 1\nr R1 Kv L-1\n".into()),
        ("list huge count", format!("tfv 1\nr R1 Kv L{big_count} N\n")),
        ("map odd tokens", "tfv 1\nr R1 Kv M1 N\n".into()),
        ("map duplicate keys", "tfv 1\nr R1 Kv M2 I1 N I1 T\n".into()),
        ("map NaN duplicate keys", "tfv 1\nr R1 Kv M2 D7ff8000000000000 N D7ff0000000000001 N\n".into()),
        ("nesting 65", nested_65),
        ("unknown tag", "tfv 1\nr R1 Kv Q1\n".into()),
        ("non-ASCII tag", "tfv 1\nr R1 Kv \u{e9}1\n".into()),
        ("multi-byte tag char", "tfv 1\nr R1 Kv \u{1f600}\n".into()),
        ("NUL in token", "tfv 1\nr R1 Kv N\u{0}\n".into()),
        ("second record broken", "tfv 1\na R0\nb R1 Kx I-0\n".into()),
    ];
    for (reason, text) in &corpus {
        assert!(
            parse_file(text).is_err(),
            "accepted a malformed file ({reason}): {text:?}"
        );
    }
    assert!(corpus.len() >= 80);
}

#[test]
fn well_formed_corner_cases_are_accepted() {
    let accepted = [
        ("header only, no newline", "tfv 1"),
        ("header only, newline", "tfv 1\n"),
        ("comments and blanks", "tfv 1\n\n# a\n#\n\nr R0\n# end\n"),
        ("CR inside a comment is not a record CR", "tfv 1\n# c\r\nr R0\n"),
        ("empty record", "tfv 1\nr R0"),
        ("id with every allowed punctuation", "tfv 1\nA.b/c-d_9 R0\n"),
        ("1000-digit integer", &format!("tfv 1\nr R1 Kv I{}\n", "9".repeat(1000))),
        ("negative 1000-digit integer", &format!("tfv 1\nr R1 Kv I-{}\n", "9".repeat(1000))),
        ("nesting 64", &format!("tfv 1\nr R1 Kv {}N\n", "L1 ".repeat(64))),
        ("maps nest as deep as lists", &format!("tfv 1\nr R1 Kv {}N\n", "M1 N ".repeat(64))),
        ("text with a space inside", "tfv 1\nr R1 Kv S612062\n"),
        ("text NUL", "tfv 1\nr R1 Kv S00\n"),
        ("text U+10FFFF", "tfv 1\nr R1 Kv Sf48fbfbf\n"),
        ("noncharacter U+FFFF is valid UTF-8", "tfv 1\nr R1 Kv Sefbfbf\n"),
        ("U+FEFF inside text", "tfv 1\nr R1 Kv Sefbbbf\n"),
        ("keys are not sorted", "tfv 1\nr R2 Kz N Ka N\n"),
        ("same field name in two records", "tfv 1\na R1 Kx N\nb R1 Kx N\n"),
        ("record ids differing only in case", "tfv 1\na R0\nA R0\n"),
    ];
    for (reason, text) in accepted {
        let result = parse_file(text);
        assert!(result.is_ok(), "{reason}: {result:?}");
    }
    // An id that looks like a comment marker cannot be a record id.
    assert!(parse_file("tfv 1\n#r R0\n").unwrap().is_empty());
    // Lines are keyed by id: a duplicate id is caught even when the
    // comment between them is the only difference.
    let by_id: HashMap<String, usize> = parse_file("tfv 1\na R0\nb R0\n")
        .unwrap()
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.id, i))
        .collect();
    assert_eq!(by_id["a"], 0);
    assert_eq!(by_id["b"], 1);
}
