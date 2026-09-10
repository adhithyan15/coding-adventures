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
    // The pin here used to be a HOSTNAME and a TRUST TIER:
    // `contains("nasa.gov") && contains(trust)`. Both were satisfied by the
    // welded 166-character `cites` that #13934 installment 4o split -- a
    // string that occurs ZERO times on the page it named, under either
    // extractor -- exactly as happily as by the two real spans that replaced
    // it. The same shape was repaired in 4j, 4k, 4l, 4m and 4n before this.
    //
    // The needle is the whole citation object as the serialiser actually
    // emits it, taken from a real run rather than from memory of the format,
    // and it CLOSES on the corroborations `]`. That bounds the source text,
    // every locator, the trust tier, and the corroboration SET -- so a
    // fabricated `cites` cannot be appended without reddening (#14735). What
    // it does NOT bound is how many citation objects the output holds:
    // nothing here asserts that `citations` has exactly one element.
    //
    // Its string appears EIGHT times in this test's output, at eight distinct
    // JSON paths: `recall/[n]/answers/[0]/citations/[0]` and `.../steps/[0]`
    // for each of the four queries above. That was counted BEFORE trusting
    // the needle. Six of the seven mutants touch a citation field and drive
    // all eight to zero together -- echoes of one field, not the
    // independently-driftable copies of #14745. The seventh, DROP-ROW,
    // touches no citation field: it drops the count to six, and is caught by
    // the `step_2` assertion above instead.
    assert!(
        out.contains(
            "\"source\":\"Ask a question or make a statement that you can test by an experiment. This statement is called a hypothesis.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\",\"trust\":\"authoritative\",\"corroborations\":[{\"source\":\"Define the parts of your experiment that will change. These are called variables.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"},{\"source\":\"Define the parts of your experiment that will not change. These are called controls.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"},{\"source\":\"Find out what people have already said or written about the subject.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"},{\"source\":\"Think of an experiment to test the hypothesis.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"},{\"source\":\"Now do the experiment and carefully record the data.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"},{\"source\":\"Figure out what the data means.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"},{\"source\":\"Draw conclusions and write a report.\",\"locator\":\"https://spaceplace.nasa.gov/review/science-fair/scientific-method.html\"}]"
        ),
        "carries the NASA citation with exactly these seven corroborations, in this \
         order, with the corroborations `]` closing the set: {out}"
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
