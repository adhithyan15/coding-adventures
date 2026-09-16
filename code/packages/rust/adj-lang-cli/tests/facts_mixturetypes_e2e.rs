//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/mixture-types.adj`) driven through the built
//! CLI: a native `table` of mixture kind → the everyday example the source names
//! resolves binding-query recalls (forward and backward) with the source's
//! LibreTexts citation, and abstains on a kind not in the table (alloy) — 0
//! model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the `homogeneous` span, and the other FOUR sentences sat in table-level
//! `cites` — riding along as corroborations rather than as their rows'
//! warrants. So asking what a colloid is returned `milk` warranted primarily by
//! a sentence about homogeneous salt water.
//!
//! FOUR RELOCATED `cites` IS WHY `only_citation()` CLOSES ON AN EMPTY
//! corroborations ARRAY. Before the conversion that array was non-empty on
//! every answer, so this needle could not have matched at all; it became
//! satisfiable only when the four `cites` moved into their own rows.
//!
//! NEGATIVE ARMS NAME WHOLE SPANS, because bare needles are exclusive to no
//! row. Measured over the shipped spans: `homogeneous` and `solution` share
//! "salt" and "water"; `homogeneous` and `heterogeneous` share "composition",
//! "mixture", "throughout" and "uniform". No span is a substring of another,
//! but no single word separates them either — the same hazard `plant-parts`
//! has with "roots" and `comet-part` with "nucleus", reached by a third route.
//!
//! THE ENVELOPE'S SPACES ARE U+00A0. The page writes "A mixture is" with two
//! NON-BREAKING spaces; the ASCII spelling occurs ZERO times on the page, so
//! shipping it would cite a sentence the source does not contain.
//! `the_envelope_keeps_the_pages_own_nbsp` pins both directions.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmix_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

fn place_lib(dir: &Path) {
    std::fs::copy(
        facts_stdlib().join("chemistry/mixture-types.adj"),
        dir.join("mixture-types.adj"),
    )
    .expect("copy shipped mixture-types.adj");
}

const LOCATOR: &str = "https://chem.libretexts.org/Courses/Mendocino_College/Introduction_to_Chemistry_(CHM_200)/09:_Solutions_and_Aqueous_Mixtures/9.01:_Mixtures";

/// The page's framing definition of a mixture. It names no row key as a whole
/// word, so it warrants no row by itself.
///
/// ITS TWO SPACES ARE U+00A0. The page encodes "A mixture is" with non-breaking
/// spaces; the ASCII spelling occurs ZERO times there, so shipping the ASCII
/// form would cite a sentence the source does not contain. Written with
/// `\u{a0}` escapes so the bytes survive any editor that would helpfully
/// normalize them — and so this const cannot be mistaken for the ASCII twin
/// when read in a terminal or a grep result, both of which render U+00A0 as an
/// ordinary space.
const ENVELOPE: &str =
    "A\u{a0}mixture\u{a0}is a combination of two or more substances in any proportion.";

/// Shares its VALUE (`salt_water`) with `solution`, but not its span.
const HOMOGENEOUS: &str = "A homogeneous mixture is a mixture in which the composition is uniform throughout the mixture. The salt water described above is homogeneous because the dissolved salt is evenly distributed throughout the entire salt water sample.";
/// Shares its VALUE (`salt_water`) with `homogeneous`, but not its span.
const SOLUTION: &str = "When the salt is thoroughly mixed into the water in this glass, it will form a solution.";
const HETEROGENEOUS: &str = "A heterogeneous mixture is a mixture in which the composition is not uniform throughout the mixture. Vegetable soup is a heterogeneous mixture.";
/// The row whose evidence was hardest to find: the page's FIRST mention of
/// salad dressing lists it among "liquid\u{a0}mixtures" -- plural, with a
/// U+00A0 before the word -- and never says suspension. The sentence that does
/// is later on the page.
///
/// The earlier wording here quoted "liquid mixture" singular and said the real
/// sentence was "four blocks later". Both were paraphrase: the singular form
/// does not occur, and I had no measurement for the block distance. In a test
/// whose subject is verbatim fidelity under invisible bytes, that is the one
/// place the same hazard recurred.
const SUSPENSION: &str = "The salad dressing in this bottle is a suspension.";
const COLLOID: &str = "Homogenized milk is a colloid.";

/// (mixture kind, the example atom, the LibreTexts sentence naming that kind)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("homogeneous", "salt_water", HOMOGENEOUS),
        ("solution", "salt_water", SOLUTION),
        ("heterogeneous", "vegetable_soup", HETEROGENEOUS),
        ("suspension", "salad_dressing", SUSPENSION),
        ("colloid", "milk", COLLOID),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
///
/// NOTE: before the RS-5e conversion this table carried FOUR table-level
/// `cites`, so `corroborations` was non-empty on every answer and this needle
/// could not match at all. It became satisfiable only when those four moved
/// into their own rows' `source` fields.
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("chemistry/mixture-types.adj"))
        .expect("read shipped mixture-types.adj");
    adj[adj.find("table mixture_example").expect("table")..].to_string()
}

#[test]
fn chemistry_mixture_example_recall_binds_example_with_citation() {
    let dir = scratch("mixturetypes");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mixture-types.adj\"\n\
         ? mixture_example(colloid, $Example)\n\
         ? mixture_example(suspension, $Example)\n\
         ? mixture_example($Kind, vegetable_soup)\n\
         ? mixture_example(alloy, $Example)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");

    // THIS PIN USED TO ASSERT THE HOMOGENEOUS SPAN AS THE ENVELOPE, AND BEFORE
    // THAT A FIVE-CLAUSE JOINED VALUE. Its methodology was right both times --
    // anchor on the JSON key, close on the terminating quote, never pin a
    // fragment -- and it caught a bad repair that grounded none of the five
    // rows. But an anchored pin defends whatever it is pointed at, including a
    // defect: first a constructed span no page displays (#14070), then a real
    // sentence that was the wrong row's (#14986). It is now pointed at each
    // ANSWER's own citation.
    //
    // THREE, not four. This case asks FOUR queries -- colloid, suspension, the
    // backward `vegetable_soup` bind, and `alloy` -- but `alloy` ABSTAINS, and
    // an abstaining query produces no citations array at all.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        3,
        "three citations arrays -- one per ANSWERING query; alloy abstains: {out}"
    );
    // A colloid's everyday example is milk; a suspension's is salad dressing —
    // the recalled example values (forward binds).
    assert!(out.contains("\"Example\":\"milk\""), "colloid -> milk: {out}");
    assert!(
        out.contains("\"Example\":\"salad_dressing\""),
        "suspension -> salad_dressing: {out}"
    );
    // The relation runs BACKWARD: bind the example vegetable_soup, recall the
    // kind the source classifies it as — heterogeneous.
    assert!(
        out.contains("\"Kind\":\"heterogeneous\""),
        "vegetable_soup -> heterogeneous (reverse recall): {out}"
    );
    // Separate assertions, not one `&&`: joined, a failure cannot say which arm
    // broke. Each answer carries ITS OWN sentence, whole, as its only citation.
    assert!(
        out.contains(&only_citation(COLLOID)),
        "the colloid answer carries the sentence about colloids: {out}"
    );
    assert!(
        out.contains(&only_citation(SUSPENSION)),
        "the suspension answer carries the sentence naming suspension: {out}"
    );
    assert!(
        out.contains(&only_citation(HETEROGENEOUS)),
        "the reverse answer carries the heterogeneous sentence: {out}"
    );
    // The envelope's wording is primary for no answer.
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
    // The homogeneous span no longer rides on every answer. This is the whole
    // defect, stated as a needle: before the conversion it appeared on all
    // three of the above.
    assert!(
        !out.contains(HOMOGENEOUS),
        "the homogeneous sentence must not warrant a colloid, suspension or \
         vegetable-soup answer: {out}"
    );
    // "alloy" is not in the table — honest abstention, never a fabricated example.
    assert!(out.contains("\"abstained\":true"), "alloy abstains: {out}");
}

#[test]
fn every_kind_answer_carries_only_the_sentence_about_that_kind() {
    for (kind, example, span) in scale() {
        let dir = scratch(&format!("kind_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"mixture-types.adj\"\n? mixture_example({kind}, $Example)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {kind}: {out}"
        );
        assert!(
            out.contains(&format!("\"Example\":\"{example}\"")),
            "{kind} -> {example}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{kind}: the LibreTexts sentence naming it, whole, and the only citation: {out}"
        );
        // WHOLE SPANS only. `homogeneous` and `solution` share "salt" and
        // "water"; `homogeneous` and `heterogeneous` share "composition",
        // "mixture", "throughout" and "uniform". A bare word is exclusive to no
        // row, so a needle short of a whole span proves nothing here.
        for other in [HOMOGENEOUS, SOLUTION, HETEROGENEOUS, SUSPENSION, COLLOID] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another kind's sentence must not reach {kind}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {kind}: {out}"
        );
    }
}

#[test]
fn the_two_salt_water_kinds_cite_different_sentences() {
    // THE HONEST DUPLICATE, WHICH NOTHING PINNED BEFORE THIS CHANGE. Both
    // `homogeneous` and `solution` bind `salt_water` -- the source's own point,
    // since a solution IS a homogeneous mixture. The file's header argues that
    // at length; `grep salt_water` over this test previously returned nothing,
    // and the query example never binds it either (its reverse query binds
    // `vegetable_soup`). The property the header defends hardest was the one
    // with no coverage.
    //
    // Sharing a VALUE must not become sharing a CITATION. Before the
    // conversion it did: the homogeneous span was the envelope, so both rows
    // answered with it. An edit that deduplicated by value would leave the
    // forward and backward binds passing unchanged.
    let body = shipped_table();
    assert!(
        body.contains(&format!("        source \"{HOMOGENEOUS}\"\n")),
        "homogeneous cites the sentence about homogeneous mixtures: {body}"
    );
    assert!(
        body.contains(&format!("        source \"{SOLUTION}\"\n")),
        "solution cites the sentence about forming a solution: {body}"
    );
    assert_ne!(
        HOMOGENEOUS, SOLUTION,
        "the two spans must not be the same sentence"
    );

    // And through the engine, not only in the file. Measured: this query
    // returns BOTH rows, so no arm here may assert one citations array, nor
    // that either span is absent -- both legitimately appear.
    let dir = scratch("saltwater");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"mixture-types.adj\"\n? mixture_example($Kind, salt_water)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        2,
        "a reverse recall on salt_water honestly returns BOTH kinds: {out}"
    );
    assert!(
        out.contains("\"Kind\":\"homogeneous\""),
        "salt_water -> homogeneous: {out}"
    );
    assert!(
        out.contains("\"Kind\":\"solution\""),
        "salt_water -> solution: {out}"
    );
    assert!(
        out.contains(&only_citation(HOMOGENEOUS)),
        "the homogeneous answer carries its own sentence: {out}"
    );
    assert!(
        out.contains(&only_citation(SOLUTION)),
        "the solution answer carries its own sentence, NOT the homogeneous one \
         it shares a value with: {out}"
    );
}

#[test]
fn the_envelope_keeps_the_pages_own_nbsp() {
    // The page writes "A\u{a0}mixture\u{a0}is" with U+00A0. The ASCII spelling
    // occurs ZERO times on the page, so shipping it would cite a sentence the
    // source does not contain -- #15324's defect for soil-texture-class.
    //
    // This arm is worth more than the apostrophe equivalents, because THREE
    // display layers render U+00A0 as an ordinary space: the terminal, ripgrep,
    // and diff views. A reviewer reading any of them sees the ASCII twin and
    // cannot tell the difference. Only a byte comparison can.
    let body = shipped_table();

    // TWO DIFFERENT SCOPES, because the two arms make different claims.
    //
    // The POSITIVE arm is about the TABLE-LEVEL envelope, so it reads only
    // four-space `source` lines. Scoped with `trim_start()` it would also see
    // the eight-space row sources, and would then be satisfied by a mutant that
    // deleted the envelope line and planted the string on a row.
    let envelope_line: String = body
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        envelope_line.contains(ENVELOPE),
        "the table-level envelope ships with the page's own U+00A0: {envelope_line:?}"
    );

    // The NEGATIVE arm is about ANY shipped citation, so it reads every
    // `source` line at any indent -- a row carrying an ASCII-spaced envelope is
    // just as wrong as the envelope line carrying one. It stays scoped to
    // `source` lines rather than the whole block, because a `%` comment
    // quoting the sentence is not a shipped citation.
    let source_lines: String = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    let ascii_variant = ENVELOPE.replace('\u{a0}', " ");
    assert!(
        !source_lines.contains(&ascii_variant),
        "no `source` line at any indent may carry an ASCII-spaced envelope -- \
         that spelling occurs zero times on the page: {source_lines:?}"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        5,
        "five row sources"
    );
    // THIS ARM WOULD HAVE FAILED ON THE PRE-CONVERSION FILE, which shipped FOUR
    // table-level `cites`, each with its own redundant `locator` repeating the
    // envelope's. Keyword-anchored, so a `cites` at any indent fails it, and no
    // false positive on the word appearing in header prose.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent: {body}"
    );
    // Indent-independent: the envelope has a `locator` and a `trust` of its
    // own, so the pin is EXACTLY one of each. The pre-conversion file had FIVE
    // locator lines -- the envelope's and one inside each of the four `cites`.
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("locator "))
            .count(),
        1,
        "exactly one locator line, the envelope's: {body}"
    );
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("trust "))
            .count(),
        1,
        "exactly one trust line, the envelope's: {body}"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n"
        )),
        "the envelope is the page's mixture-framing definition, at the consensus tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{HOMOGENEOUS}\"\n    locator")),
        "not the homogeneous span as the envelope again"
    );
    // WHOLE WORDS, not substrings, and PLURALS TOO. A singular-only check has a
    // hole: the token "solutions" never equals the key "solution", and the page
    // has a sentence reading "...three different types: solutions, suspensions,
    // and colloids." An envelope naming the kinds in plural is just as wrong as
    // one naming them in singular.
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    let tokens: Vec<&str> = shipped_envelope
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for word in [
        "homogeneous",
        "heterogeneous",
        "solution",
        "solutions",
        "suspension",
        "suspensions",
        "colloid",
        "colloids",
    ] {
        assert!(
            !tokens.contains(&word),
            "the shipped envelope must name no mixture kind, but contains {word:?} \
             as a whole word: {shipped_envelope:?}"
        );
    }
}

#[test]
fn every_row_example_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check, in the form this schema
    // needs. This is what `element-categories` (#15318) failed -- a row whose
    // span named no category at all.
    //
    // NOTE the atoms here are VALUES (`salt_water`, `milk`), not phrases lifted
    // from the sentence, so comet-tail-type's "span states the whole atom as a
    // contiguous phrase" check does not transfer: the page writes "salt water"
    // and "Homogenized milk", never the atom spelling. Per-row needles instead.
    let body = shipped_table();
    for (kind, example, span) in scale() {
        // ANCHORED ON THE ROW HEADER, not on the bare `source` line. Matching
        // only `        source "<span>"` proves the span sits among the
        // eight-space source lines SOMEWHERE -- it does not prove it belongs to
        // THIS row. A mutant swapping two rows' spans satisfies the weaker
        // needle completely, which is exactly what happened: mutant M07 swapped
        // the two `salt_water` spans and this test passed.
        assert!(
            body.contains(&format!(
                "row ({kind}, {example}) {{\n        source \"{span}\"\n"
            )),
            "{kind} carries its own span IN ITS OWN ROW: {body}"
        );
        // The token checks below read `span`, which is a const in THIS FILE --
        // so on their own they can never fail for any edit to the `.adj`. They
        // are self-checks on the literal. What makes them measure the shipped
        // data is the assertion directly above, which ties this literal to the
        // row it claims to describe; the chain, not either link, is the pin.
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        // The span must NAME its kind as a whole word. Stated per row rather
        // than guessed at with a stemmer.
        assert!(
            tokens.contains(&kind),
            "{kind}: its own span must name the kind as a whole word, but it is \
             not a token of {normalized:?}"
        );
        // And it must name the EXAMPLE the row binds, in the page's own
        // spelling. EXHAUSTIVE, no catch-all: a `_` arm would silently apply
        // one row's needle to any row added later.
        let example_words: &[&str] = match kind {
            "homogeneous" => &["salt", "water"],
            "solution" => &["salt", "water"],
            "heterogeneous" => &["vegetable", "soup"],
            "suspension" => &["salad", "dressing"],
            "colloid" => &["milk"],
            other => panic!("no example needle registered for row {other}"),
        };
        for needle in example_words {
            assert!(
                tokens.contains(needle),
                "{kind}: its span must name {needle:?}, part of the example atom \
                 {example:?}, but it is not a token of {normalized:?}"
            );
        }
    }
}
