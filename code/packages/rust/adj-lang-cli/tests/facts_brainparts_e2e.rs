//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/brain-parts.adj`) driven through the built CLI:
//! a native `table` of brain part → primary function resolves a binding-query
//! recall with the source's NCI SEER Training citation, runs the relation
//! backward (function → part), and abstains on a non-part (a neuron) — 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsbrain_{tag}_{}", std::process::id()));
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

#[test]
fn anatomy_brain_parts_recall_binds_function_with_citation() {
    let dir = scratch("brainparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/brain-parts.adj");
    std::fs::copy(&src, dir.join("brain-parts.adj")).expect("copy shipped brain-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"brain-parts.adj\"\n\
         ? brain_part_function(cerebellum, $Job)\n\
         ? brain_part_function(hippocampus, $Job)\n\
         ? brain_part_function($Part, breathing)\n\
         ? brain_part_function(neuron, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The cerebellum coordinates voluntary movement; the hippocampus does
    // memory — the recalled functions, each a single verbatim token/phrase.
    assert!(
        out.contains("\"Job\":\"coordination_of_voluntary_movement\""),
        "cerebellum → coordination_of_voluntary_movement: {out}"
    );
    assert!(out.contains("\"Job\":\"memory\""), "hippocampus → memory: {out}");
    // The relation runs backward: the function `breathing` recalls the brainstem.
    assert!(
        out.contains("\"Part\":\"brainstem\""),
        "breathing → brainstem (reverse recall): {out}"
    );
    // THIS ASSERTION WAS THE DEFECT IN TEST FORM. It required every answer
    // to carry `training.seer.cancer.gov` — the table's old envelope locator —
    // and it passed, because the envelope covered all fifteen rows. But that
    // SEER page contains no brain-stem function, no thalamus sentence, and the
    // word "hippocampus" zero times. The test was pinning the wrong page onto
    // thirteen rows.
    //
    // Since #14986 each row carries the locator of the page that states it, so
    // the queries above cite StatPearls, not SEER. Pinned per row instead.
    assert!(
        out.contains(
            "\"locator\":\"https://www.ncbi.nlm.nih.gov/books/NBK551718/\",\"trust\":\"authoritative\""
        ),
        "the cerebellum and brainstem answers cite the page that states them: {out}"
    );
    assert!(
        out.contains(
            "\"locator\":\"https://www.ncbi.nlm.nih.gov/books/NBK482171/\",\"trust\":\"authoritative\""
        ),
        "the hippocampus answer cites the hippocampus chapter: {out}"
    );
    assert!(
        !out.contains("training.seer.cancer.gov"),
        "and none of these four answers cites the SEER page, which states none of them: {out}"
    );
    // A neuron is a cell, not one of these gross brain parts — honest
    // abstention, never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "unknown part abstains: {out}");
}

#[test]
fn anatomy_brain_parts_extension_recalls_newly_added_brainstem_functions() {
    let dir = scratch("brainparts_ext");
    let src = facts_stdlib().join("anatomy/brain-parts.adj");
    std::fs::copy(&src, dir.join("brain-parts.adj")).expect("copy shipped brain-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"brain-parts.adj\"\n\
         ? brain_part_function(brainstem, $Job)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // brainstem's own cited source sentence always listed ten autonomic
    // functions, but only "breathing" was ever shipped as a row until this
    // cycle -- the other nine are pure additions, each a new row sharing
    // the same brainstem key.
    for job in [
        "breathing",
        "temperature_regulation",
        "respiration",
        "heart_rate",
        "wake_sleep_cycles",
        "coughing",
        "sneezing",
        "digestion",
        "vomiting",
        "swallowing",
    ] {
        assert!(
            out.contains(&format!("brain_part_function(brainstem, {job})")),
            "brainstem recalls {job} (added this cycle unless it's breathing): {out}"
        );
    }
}

const SEER_BRAIN: &str = "https://training.seer.cancer.gov/brain/tumors/anatomy/brain.html";
const PHYSIOLOGY_BRAIN: &str = "https://www.ncbi.nlm.nih.gov/books/NBK551718/";
const HIPPOCAMPUS_CH: &str = "https://www.ncbi.nlm.nih.gov/books/NBK482171/";

const BRAINSTEM_SPAN: &str = "The brainstem houses the principal centers that perform autonomic functions such as breathing, temperature regulation, respiration, heart rate, wake-sleep cycles, coughing, sneezing, digestion, vomiting, and swallowing.";

/// Assert one row's warrant, binding the FUNCTION so exactly one row answers.
///
/// Binding the part would return ten rows for `brainstem`, and a whole-stdout
/// `contains` is then satisfied by any sibling's intact copy — the masking
/// defect found in `joint-types` (#15164). Every function in this table is
/// unique to one row, so the function direction is single-answer.
fn assert_part(tag: &str, function: &str, part: &str, span: &str, locator: &str) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("anatomy/brain-parts.adj"),
        dir.join("brain-parts.adj"),
    )
    .expect("copy shipped brain-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"brain-parts.adj\"\n? brain_part_function($P, {function})\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row has the function {function}: {out}"
    );
    assert!(
        out.contains(&format!("\"P\":\"{part}\"")),
        "{function} binds {part}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{locator}\",\"trust\":\"authoritative\""
        )),
        "{function} is warranted by a sentence about {part}, on the page that states it: {out}"
    );
    out
}

/// #14986, and the sharpest mis-citation in the cascade after `skeleton-bones`
/// (#15171). The CEREBRUM sentence was this table's `source` — the field that
/// carries the tier — for all fifteen rows, on a SEER page that contains **no
/// brain-stem function at all** (its only brain stem sentence is anatomical),
/// **no thalamus sentence** ("thalamus" occurs there only inside
/// "hypothalamus"), and the word **"hippocampus" zero times**.
///
/// The evidence for those thirteen rows was already in the file, one line
/// below, as untiered `cites` on two StatPearls pages.
#[test]
fn every_row_cites_a_page_that_states_it() {
    assert_part(
        "bpconscious", "conscious_activity", "cerebrum",
        "The cerebrum is the part of the brain that receives and processes conscious sensation, generates thought, and controls conscious activity.",
        SEER_BRAIN,
    );
    assert_part(
        "bpcoord", "coordination_of_voluntary_movement", "cerebellum",
        "The cerebellum controls the coordination of voluntary movement and receives sensory information from the brain and spinal cord to fine-tune the precision and accuracy of motor activity.",
        PHYSIOLOGY_BRAIN,
    );
    assert_part(
        "bphunger", "hunger", "hypothalamus",
        "The hypothalamus controls hunger, thirst, and sleep.",
        PHYSIOLOGY_BRAIN,
    );
    assert_part(
        "bprelays", "relays", "thalamus",
        "Sensory neurons bring sensory input from the body to the thalamus, which relays this information to the cerebrum.",
        PHYSIOLOGY_BRAIN,
    );
    assert_part(
        "bpmemory", "memory", "hippocampus",
        "The hippocampus is the \\\"flash drive\\\" of the human brain and is often associated with memory consolidation and decision-making, but it is far more complex in structure and function than a flash drive.",
        HIPPOCAMPUS_CH,
    );
}

/// TEN rows share the brainstem sentence — it lists ten autonomic functions.
/// Each is pinned separately: one broken copy behind eight intact ones is the
/// failure `skeleton-bones` shipped and every entry since has designed out.
#[test]
fn all_ten_brainstem_functions_carry_their_own_copy() {
    for (tag, function) in [
        ("bs1", "breathing"),
        ("bs2", "temperature_regulation"),
        ("bs3", "respiration"),
        ("bs4", "heart_rate"),
        ("bs5", "wake_sleep_cycles"),
        ("bs6", "coughing"),
        ("bs7", "sneezing"),
        ("bs8", "digestion"),
        ("bs9", "vomiting"),
        ("bs10", "swallowing"),
    ] {
        assert_part(tag, function, "brainstem", BRAINSTEM_SPAN, PHYSIOLOGY_BRAIN);
    }
}

/// The envelope is the framing sentence — three parts, no function — so it
/// warrants none of the fifteen rows. Its wording is pinned against the shipped
/// file rather than disclosed as unreachable (#15176).
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("bpenvelope");
    std::fs::copy(
        facts_stdlib().join("anatomy/brain-parts.adj"),
        dir.join("brain-parts.adj"),
    )
    .expect("copy shipped brain-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"brain-parts.adj\"\n? brain_part_function($P, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        15,
        "all fifteen rows answer: {out}"
    );
    assert!(
        !out.contains("The 3 main parts of the human brain"),
        "the framing span warrants no row: {out}"
    );
    // The SEER page now warrants exactly ONE row — the cerebrum — where it
    // used to be the locator on all fifteen. Twice: citations and steps.
    assert_eq!(
        out.matches(SEER_BRAIN).count(),
        2,
        "the SEER page warrants the cerebrum row and nothing else: {out}"
    );
    let adj = std::fs::read_to_string(
        facts_stdlib().join("anatomy/brain-parts.adj"),
    )
    .expect("read shipped brain-parts.adj");
    assert!(
        adj.contains(
            "    source \"The 3 main parts of the human brain are the cerebrum, cerebellum, and brainstem.\"\n    locator \"https://www.ncbi.nlm.nih.gov/books/NBK551718/\"\n    trust authoritative"
        ),
        "the envelope carries the framing sentence, verbatim"
    );
}

/// Parse `(row key, its own `source`)` out of a shipped `.adj`.
///
/// The full two-column key: column 1 alone is not a row identity. A first pass
/// at the #15193 census keyed on column 1 and printed brain-parts as
/// `brainstem+brainstem+…`, because ten rows there share that part and differ
/// only in the function.
fn row_spans(rel: &str) -> Vec<(String, String)> {
    let adj = std::fs::read_to_string(facts_stdlib().join(rel))
        .unwrap_or_else(|e| panic!("read shipped {rel}: {e}"));
    let mut out: Vec<(String, String)> = Vec::new();
    let mut key: Option<String> = None;
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix("    row (") {
            key = rest.split(')').next().map(|k| {
                k.split(',').map(|p| p.trim()).collect::<Vec<_>>().join(", ")
            });
        } else if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            let span = rest.trim_end_matches(0x22 as char).to_string();
            out.push((key.clone().expect("a row precedes every source"), span));
        }
    }
    out
}

/// The groups of rows that share one span, in first-appearance order.
fn sharing_groups(pairs: &[(String, String)]) -> Vec<Vec<String>> {
    let mut order: Vec<String> = Vec::new();
    for (_, span) in pairs {
        if !order.contains(span) {
            order.push(span.clone());
        }
    }
    order
        .iter()
        .map(|span| {
            pairs
                .iter()
                .filter(|(_, s)| s == span)
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>()
        })
        .filter(|g| g.len() > 1)
        .collect()
}

/// #15193. The sharing structure of this table, asserted against the SHIPPED
/// FILE rather than described in a comment.
///
/// Ten rows share the brainstem sentence, which lists ten autonomic
/// functions. That is the shape that hid five unpinned rows in
/// `skeleton-bones` (#15171).
#[test]
fn the_shared_spans_are_exactly_the_declared_ones() {
    let pairs = row_spans("anatomy/brain-parts.adj");
    assert_eq!(pairs.len(), 15, "every row carries its own source: {pairs:?}");
    let groups = sharing_groups(&pairs);
    let expected: Vec<Vec<&str>> = vec![
        vec!["brainstem, breathing", "brainstem, temperature_regulation", "brainstem, respiration", "brainstem, heart_rate", "brainstem, wake_sleep_cycles", "brainstem, coughing", "brainstem, sneezing", "brainstem, digestion", "brainstem, vomiting", "brainstem, swallowing"],
    ];
    assert_eq!(
        groups, expected,
        "the shared sentences are shared by exactly these rows: {groups:?}"
    );
    let shared: usize = groups.iter().map(|g| g.len()).sum();
    assert_eq!(
        pairs.len() - shared,
        5,
        "and 5 rows have a sentence to themselves: {groups:?}"
    );
    // THE LOCATOR RULE. `reference-lines.adj` ships it: restate a row locator
    // when its page DIFFERS from the envelope's, inherit when it is the same.
    // Two rows here are on other pages and keep theirs; thirteen inherit.
    //
    // Added because the deletion that made this true was not self-guarding:
    // re-adding the envelope's URL to a row was caught only incidentally, by
    // tests that noticed the changed citation, never by the rule.
    let adj_text = std::fs::read_to_string(facts_stdlib().join("anatomy/brain-parts.adj"))
        .expect("read shipped brain-parts.adj");
    // THE ENVELOPE LOCATOR IS DERIVED TOTALLY, not positionally. An earlier
    // form took the FIRST four-space `locator` line -- and review showed a row
    // locator written with four spaces instead of eight is accepted by the
    // parser, shipped as that answer's locator, AND adopted by this guard as
    // "the envelope", so the guard passed on exactly the mutation it exists
    // for. Requiring EXACTLY ONE four-space locator, and every locator line to
    // be indented four or eight, makes that mutation reddening rather than
    // invisible.
    let table_locators: Vec<&str> = adj_text
        .lines()
        .filter(|l| l.starts_with("    locator \"") && !l.starts_with("     "))
        .collect();
    assert_eq!(
        table_locators.len(),
        1,
        "exactly one table-level locator: {table_locators:?}"
    );
    for l in adj_text.lines() {
        if l.trim_start().starts_with("locator \"") {
            let indent = l.len() - l.trim_start().len();
            assert!(
                indent == 4 || indent == 8,
                "every locator line is indented 4 (table) or 8 (row): {l:?}"
            );
        }
    }
    let envelope = table_locators[0]
        .trim_start()
        .trim_start_matches("locator \"")
        .trim_end_matches(0x22 as char);
    let row_locators: Vec<&str> = adj_text
        .lines()
        .filter_map(|l| l.strip_prefix(r#"        locator ""#))
        .collect();
    assert_eq!(
        row_locators.len(),
        2,
        "two rows are on other pages and restate: {row_locators:?}"
    );
    for l in &row_locators {
        assert_ne!(
            l.trim_end_matches(0x22 as char),
            envelope,
            "no row restates the envelope's own page: {row_locators:?}"
        );
    }
}
