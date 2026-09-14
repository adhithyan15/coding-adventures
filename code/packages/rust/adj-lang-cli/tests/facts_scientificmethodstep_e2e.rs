//! End-to-end test for the science FACTS library
//! (`adj-facts-stdlib/science/scientific-method-step.adj`) driven through the
//! built CLI: a native `table` naming the seven steps NASA Space Place's own
//! "Steps in Scientific Method" student page gives -- state a hypothesis,
//! define variables and controls, research, design the experiment, run the
//! experiment and record data, analyze the data, and draw conclusions.
//! 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_scimethodstep_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("science/scientific-method-step.adj");
    std::fs::copy(&src, dir.join("scientific-method-step.adj"))
        .expect("copy shipped scientific-method-step.adj");
}

#[test]
fn scientific_method_step_recall_binds_the_action_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n\
         ? scientific_method_step(step_1, $A)\n\
         ? scientific_method_step(step_2, $A)\n\
         ? scientific_method_step(step_4, $A)\n\
         ? scientific_method_step(step_7, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"A\":\"ask_a_question_or_state_a_hypothesis\""),
        "step 1 means ask_a_question_or_state_a_hypothesis: {out}"
    );
    // The row this installment repaired. Nothing bound it before: the two new
    // spans are pinned through the table-wide citation envelope, so a
    // regression that dropped the `step_2` row entirely would have left every
    // assertion here green. A DROP-ROW mutant trips this one.
    assert!(
        out.contains("\"A\":\"define_variables_and_controls\""),
        "step 2 means define_variables_and_controls: {out}"
    );
    assert!(
        out.contains("\"A\":\"design_the_experiment\""),
        "step 4 means design_the_experiment: {out}"
    );
    assert!(
        out.contains("\"A\":\"draw_conclusions_and_write_a_report\""),
        "step 7 means draw_conclusions_and_write_a_report: {out}"
    );
    // First, the welded value must not come back. This names the DEFECT
    // rather than a citation, so no duplicate elsewhere in the output can
    // satisfy it on the real one's behalf. It is asserted BEFORE the citation
    // object deliberately: with the order the other way round a RESTORE mutant
    // trips the citation assertion first and this one is never once observed
    // to fire, which makes it decoration rather than a test.
    assert!(
        !out.contains("These are called variables. Define the parts"),
        "the two NASA spans are not welded back into one: {out}"
    );
    // THE WHOLE-CHAIN PIN THAT STOOD HERE IS GONE, AND WHY MATTERS.
    //
    // It ran from the step-1 `source` through all SEVEN `corroborations` and
    // closed on the corroborations `]`, which bounded the source text, every
    // locator, the tier, and the corroboration SET — so a fabricated `cites`
    // could not be appended without reddening (#14735). Its own comment
    // recorded what it did not bound: how many citation objects the output
    // holds.
    //
    // #14986 dissolves it. Those seven corroborations were the evidence for
    // the other six rows, hung off every answer because one envelope carried
    // them; they are now each the `source` of the row they state, and only
    // `step_2` keeps a corroboration (the page states variables and controls
    // in separate sentences). A chain pin cannot survive the chain being
    // distributed.
    //
    // WHAT REPLACES IT KEEPS THE PROPERTY. `assert_step` below closes each
    // needle on the row's own `corroborations` — `[]` for six rows, and the
    // controls entry for `step_2` — so an appended `cites` still reddens, now
    // per row instead of once for the whole table.
    assert!(
        out.contains(
            "\"source\":\"Ask a question or make a statement that you can test by an experiment. This statement is called a hypothesis.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\",\"trust\":\"authoritative\",\"corroborations\":[]"
        ),
        "step_1's answer carries its own sentence and NO corroborations — the six \
         spans that used to hang off it are now the sources of the rows they state: {out}"
    );
}

#[test]
fn scientific_method_step_reverse_binds_the_step_for_that_action() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n\
         ? scientific_method_step($S, analyze_the_data)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S\":\"step_6\""),
        "the shipped analyze_the_data action is step_6: {out}"
    );
}

#[test]
fn scientific_method_step_abstains_honestly_on_an_undefined_step_number() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n\
         ? scientific_method_step(step_8, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the source page only ever numbers steps 1 through 7 -- there is no step 8 to \
         recall, so honest abstention, never invented: {out}"
    );
}

const NASA: &str = "https://spaceplace.nasa.gov/review/science-fair/scientific-method.html";

/// Assert one step's warrant, binding the STEP so exactly one row answers, and
/// CLOSING ON ITS `corroborations` — `[]` for six of the seven rows.
///
/// Closing on the corroboration set is what the whole-chain pin this replaces
/// did for the table as a whole (#14735): an appended `cites` reddens rather
/// than passing unnoticed. Per row now, because the chain was distributed.
fn assert_step(tag: &str, step: &str, action: &str, span: &str, corro: &str) {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"scientific-method-step.adj\"\n? scientific_method_step({step}, $D)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {step}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"D\":\"{action}\"")),
        "{step} binds {action}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{NASA}\",\"trust\":\"authoritative\",\"corroborations\":[{corro}]"
        )),
        "{step} is warranted by the sentence that states it, and by nothing else: {out}"
    );
}

const CONTROLS: &str = "{\"source\":\"Define the parts of your experiment that will not change. These are called controls.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"}";

/// #14986. The STEP 1 sentence was this table's `source` — the field that
/// carries the tier — for all seven rows, so a recall of `step_6` came back
/// proved by a sentence about forming a hypothesis. The other seven spans were
/// already in the file as untiered `cites`, all on the same page: the LOCATOR
/// was never wrong here, only the span.
#[test]
fn every_step_carries_the_sentence_that_states_it() {
    assert_step(
        "sm1", "step_1", "ask_a_question_or_state_a_hypothesis",
        "Ask a question or make a statement that you can test by an experiment. This statement is called a hypothesis.",
        "",
    );
    assert_step(
        "sm3", "step_3", "research_what_is_already_known",
        "Find out what people have already said or written about the subject.",
        "",
    );
    assert_step(
        "sm4", "step_4", "design_the_experiment",
        "Think of an experiment to test the hypothesis.",
        "",
    );
    assert_step(
        "sm5", "step_5", "do_the_experiment_and_record_the_data",
        "Now do the experiment and carefully record the data.",
        "",
    );
    assert_step(
        "sm6", "step_6", "analyze_the_data",
        "Figure out what the data means.",
        "",
    );
    assert_step(
        "sm7", "step_7", "draw_conclusions_and_write_a_report",
        "Draw conclusions and write a report.",
        "",
    );
}

/// ONE ROW NEEDS TWO SENTENCES. `step_2` is `define_variables_and_controls`,
/// and the page states variables and controls separately — so that row carries
/// the variables sentence as `source` and the controls sentence as a row-level
/// `cites`, which emits as a `corroborations` entry on THAT row's answer and no
/// other. Established by running it, not by reading the lowering code.
#[test]
fn the_two_part_step_carries_both_of_its_sentences() {
    assert_step(
        "sm2", "step_2", "define_variables_and_controls",
        "Define the parts of your experiment that will change. These are called variables.",
        CONTROLS,
    );
    // And the controls sentence reaches NO other row. Counted over the whole
    // table, twice per answer it warrants (citations and steps).
    let dir = scratch("sm2only");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n? scientific_method_step($S, $D)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("These are called controls.").count(),
        2,
        "the controls sentence corroborates step_2 and nothing else: {out}"
    );
}

/// The envelope is NOT a framing span here, and that is disclosed rather than
/// dressed up. Narrowed to what was checked: no sentence OUTSIDE the
/// seven `Step N.` labels frames the method. The non-step text is TWO
/// headings — "Steps in Scientific Method" and "One Way to Do
/// Science..." — and a worked kitten example. An envelope `source` is REQUIRED, so the envelope keeps the
/// step-1 sentence; every row overrides `source`, so it reaches no answer, and
/// `step_1` carries its own copy.
#[test]
fn the_envelope_span_reaches_no_answer_beyond_step_one_and_is_pinned() {
    let dir = scratch("smenvelope");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"scientific-method-step.adj\"\n? scientific_method_step($S, $D)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        7,
        "all seven rows answer: {out}"
    );
    // THE MEASUREMENT THAT MATTERS. The step-1 sentence used to be the
    // `source` on all seven rows — 14 occurrences across citations and steps.
    // It now warrants one row: 2.
    assert_eq!(
        out.matches("This statement is called a hypothesis.").count(),
        2,
        "the step-1 sentence warrants step_1 and nothing else: {out}"
    );
    // And no answer carries a corroboration except step_2's.
    assert_eq!(
        out.matches("\"corroborations\":[]").count(),
        12,
        "six of the seven rows carry no corroboration, twice each: {out}"
    );
    let adj = std::fs::read_to_string(
        facts_stdlib().join("science/scientific-method-step.adj"),
    )
    .expect("read shipped scientific-method-step.adj");
    assert!(
        adj.contains(
            "    source \"Ask a question or make a statement that you can test by an experiment. This statement is called a hypothesis.\"\n    locator \"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"\n    trust authoritative"
        ),
        "the envelope carries the step-1 sentence, verbatim, with its locator and tier"
    );
    // THE `columns` LINE. A SURVIVING MUTANT IS A FINDING: renaming it
    // survived this table's whole harness, because column names are positional
    // and never reach the output. The same survivor turned up in
    // `heredity-term` (#15188) and was found the same way.
    assert!(
        adj.contains("    columns step, action"),
        "the shipped column names are unchanged"
    );
    assert!(
        !adj.contains("\n    cites \""),
        "no corroboration survives at TABLE level: the seven that did are now \
         the sources of the rows they state, and the one that is still a \
         corroboration sits on step_2's row"
    );
    // THE LOCATOR RULE. `reference-lines.adj` ships it: restate a row locator
    // when its page DIFFERS from the envelope's, inherit when it is the same.
    // Every span here is on the one page, so NO row may carry a locator.
    //
    // Added because the deletion that made this true was not self-guarding:
    // re-adding the envelope's URL to a row passed the entire suite.
    let row_locators: Vec<&str> = adj
        .lines()
        .filter_map(|l| l.strip_prefix(r#"        locator ""#))
        .collect();
    assert!(
        row_locators.is_empty(),
        "no row carries its own locator; all inherit the envelope's: {row_locators:?}"
    );
}
