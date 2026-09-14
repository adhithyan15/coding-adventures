//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/element-groups.adj`) driven through the built
//! CLI: a native `table` of common element → periodic-table group family
//! resolves binding-query recalls (forward and backward) with the source's
//! Wikipedia citation, and abstains on an element not in the table (gold) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factseg_{tag}_{}", std::process::id()));
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
fn chemistry_element_group_family_recall_binds_family_with_citation() {
    let dir = scratch("elementgroups");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/element-groups.adj");
    std::fs::copy(&src, dir.join("element-groups.adj")).expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n\
         ? element_group_family(sodium, $Family)\n\
         ? element_group_family(chlorine, $Family)\n\
         ? element_group_family(iron, $Family)\n\
         ? element_group_family($E, noble_gas)\n\
         ? element_group_family(gold, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Sodium is an alkali metal, chlorine a halogen, iron a transition metal —
    // the recalled families (forward binds).
    assert!(
        out.contains("\"Family\":\"alkali_metal\""),
        "sodium → alkali_metal: {out}"
    );
    assert!(
        out.contains("\"Family\":\"halogen\""),
        "chlorine → halogen: {out}"
    );
    assert!(
        out.contains("\"Family\":\"transition_metal\""),
        "iron → transition_metal: {out}"
    );
    // The relation runs BACKWARD: bind the family noble_gas, recall an element in
    // it — helium, the first noble gas in the table.
    assert!(
        out.contains("\"E\":\"helium\""),
        "noble_gas → helium (reverse recall into the noble gases): {out}"
    );
    // THIS ASSERTION WAS THE DEFECT IN TEST FORM. It required the answers to
    // carry `Alkali_metal` — the table's old envelope locator — and it passed,
    // because that one page was the locator on all 27 rows. But the alkali
    // sentence names lithium through francium and no other element, so it
    // warranted nothing about chlorine or iron.
    //
    // Since #14986 each row cites the page that states it, so these four
    // answers span three different pages.
    assert!(
        out.contains("en.wikipedia.org/wiki/Alkali_metal")
            && out.contains("\"trust\":\"consensus\""),
        "sodium's answer cites the alkali-metal page, at consensus trust: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org/wiki/Halogen"),
        "chlorine's answer cites the halogen page: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org/wiki/Transition_metal"),
        "iron's answer cites the transition-metal page: {out}"
    );
    // "gold" is not in the table — honest abstention, never a fabricated family.
    assert!(out.contains("\"abstained\":true"), "gold abstains: {out}");
}

#[test]
fn chemistry_element_group_family_extension_recalls_newly_added_elements() {
    let dir = scratch("elementgroups_ext");
    let src = facts_stdlib().join("chemistry/element-groups.adj");
    std::fs::copy(&src, dir.join("element-groups.adj")).expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n\
         ? element_group_family($E, noble_gas)\n\
         ? element_group_family(caesium, $Family)\n\
         ? element_group_family(cobalt, $Family)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Each family's own cited Wikipedia sentence always named more members than
    // had ever been turned into rows — this cycle added the rest as pure
    // additions sharing the existing family key. The noble_gas reverse recall
    // now returns all six shipped noble gases (oganesson deliberately excluded
    // as a considered exclusion — its source sentence hedges it "in some
    // cases").
    for gas in ["helium", "neon", "argon", "krypton", "xenon", "radon"] {
        assert!(
            out.contains(&format!("element_group_family({gas}, noble_gas)")),
            "noble_gas recalls {gas} (krypton/xenon/radon added this cycle): {out}"
        );
    }
    // oganesson is deliberately NOT a row. Assert that STRUCTURALLY -- no
    // binding and no governing term -- rather than as a bare substring
    // absence over the whole blob.
    //
    // The previous form was `!out.contains("oganesson")`, which also forbade
    // the word appearing inside quoted EVIDENCE. Encoding the Wikipedia
    // noble-gas sentence puts "oganesson (Og)" into the citation text, so
    // that assertion failed while the property it cares about still held.
    // Note the sentence it tripped on is the same one this test's comment
    // cites as its justification ("hedges it in some cases") -- the check
    // forbade the output carrying the evidence for its own reasoning.
    assert!(
        !out.contains("element_group_family(oganesson, noble_gas)"),
        "oganesson is never a governing term: {out}"
    );
    assert!(
        !out.contains("\"E\":\"oganesson\""),
        "oganesson is never bound as an answer: {out}"
    );
    assert!(
        out.contains("\"Family\":\"alkali_metal\""),
        "caesium → alkali_metal (added this cycle): {out}"
    );
    assert!(
        out.contains("\"Family\":\"transition_metal\""),
        "cobalt → transition_metal (added this cycle): {out}"
    );
}

const ALKALI: &str = "The alkali metals consist of the chemical elements lithium (Li), sodium (Na), potassium (K),[note 1] rubidium (Rb), caesium (Cs),[note 2] and francium (Fr).";
const AEM: &str = "The alkaline earth metals are six chemical elements in group 2 of the periodic table. They are beryllium (Be), magnesium (Mg), calcium (Ca), strontium (Sr), barium (Ba), and radium (Ra).";
const HALOGEN: &str = "The halogens are a group in the periodic table consisting of six chemically related elements, fluorine (F), chlorine (Cl), bromine (Br), iodine (I), and the radioactive elements astatine (At) and tennessine (Ts), though some authors[1] would exclude tennessine as its chemistry is unknown and is theoretically expected to be more like that of gallium.";
const NOBLE: &str = "The noble gases (historically the inert gases, sometimes referred to as aerogens[1]) are the members of group 18 of the periodic table: helium (He), neon (Ne), argon (Ar), krypton (Kr), xenon (Xe), radon (Rn) and, in some cases, oganesson (Og).";
const TRANS: &str = "All of the elements that are ferromagnetic near room temperature are transition metals (iron, cobalt and nickel) or inner transition metals (gadolinium).";

const L_ALKALI: &str = "https://en.wikipedia.org/wiki/Alkali_metal";
const L_AEM: &str = "https://en.wikipedia.org/wiki/Alkaline_earth_metal";
const L_HALOGEN: &str = "https://en.wikipedia.org/wiki/Halogen";
const L_NOBLE: &str = "https://en.wikipedia.org/wiki/Noble_gas";
const L_TRANS: &str = "https://en.wikipedia.org/wiki/Transition_metal";

/// Assert one row's warrant, binding the ELEMENT so exactly one row answers.
///
/// Binding the family would return six rows, and a whole-stdout `contains` is
/// then satisfied by any sibling's intact copy — the masking defect found in
/// `joint-types` (#15164). Every element here belongs to exactly one family, so
/// the element direction is single-answer.
fn assert_element(tag: &str, element: &str, family: &str, span: &str, locator: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("chemistry/element-groups.adj"),
        dir.join("element-groups.adj"),
    )
    .expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"element-groups.adj\"\n? element_group_family({element}, $F)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {element}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"F\":\"{family}\"")),
        "{element} binds {family}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{locator}\",\"trust\":\"consensus\""
        )),
        "{element} is warranted by the sentence that names it, on the page that states it: {out}"
    );
}

/// #14986. The ALKALI-METAL sentence was this table's `source` — the field that
/// carries the tier — for all 27 rows, and it names lithium through francium
/// and nothing else. Twenty-one rows were warranted by a sentence about a
/// different family on a different page.
///
/// All 27 are pinned individually. Six rows share each of four sentences, which
/// is the shape where, in `skeleton-bones` (#15171), testing one row per shared
/// span left five rows pinned by nothing.
#[test]
fn every_row_carries_the_sentence_that_names_it() {
    for (tag, e) in [
        ("egli", "lithium"),
        ("egna", "sodium"),
        ("egk", "potassium"),
        ("egrb", "rubidium"),
        ("egcs", "caesium"),
        ("egfr", "francium"),
    ] {
        assert_element(tag, e, "alkali_metal", ALKALI, L_ALKALI);
    }
    for (tag, e) in [
        ("egbe", "beryllium"),
        ("egmg", "magnesium"),
        ("egca", "calcium"),
        ("egsr", "strontium"),
        ("egba", "barium"),
        ("egra", "radium"),
    ] {
        assert_element(tag, e, "alkaline_earth_metal", AEM, L_AEM);
    }
    for (tag, e) in [
        ("egf", "fluorine"),
        ("egcl", "chlorine"),
        ("egbr", "bromine"),
        ("egi", "iodine"),
        ("egat", "astatine"),
        ("egts", "tennessine"),
    ] {
        assert_element(tag, e, "halogen", HALOGEN, L_HALOGEN);
    }
    for (tag, e) in [
        ("eghe", "helium"),
        ("egne", "neon"),
        ("egar", "argon"),
        ("egkr", "krypton"),
        ("egxe", "xenon"),
        ("egrn", "radon"),
    ] {
        assert_element(tag, e, "noble_gas", NOBLE, L_NOBLE);
    }
    for (tag, e) in [("egfe", "iron"), ("egni", "nickel"), ("egco", "cobalt")] {
        assert_element(tag, e, "transition_metal", TRANS, L_TRANS);
    }
}

/// The tennessine caveat, which used to be held by a whole-list corroboration
/// pin, is now inside the halogen rows' own `source`.
///
/// The page's sentence ends "...and tennessine (Ts), though some authors[1]
/// would exclude tennessine as its chemistry is unknown..." — while this table
/// ships `row (tennessine, halogen)`. Where the page qualifies its own claim,
/// the qualification is part of the evidence, so the row that the caveat is
/// ABOUT is the one asserted here.
#[test]
fn the_tennessine_row_carries_the_pages_own_caveat_about_it() {
    assert_element("egtscaveat", "tennessine", "halogen", HALOGEN, L_HALOGEN);
    assert!(
        HALOGEN.contains("though some authors[1] would exclude tennessine"),
        "the span pinned above is the one carrying the caveat"
    );
}

/// The transition-metal sentence WAS on the live page all along.
///
/// A shipped comment here said it "is not on the live page under any extractor
/// fix, and the nearest candidate names only iron, indirectly". Measured this
/// cycle against the rendered article: the full sentence is verbatim, and the
/// TRUNCATED form the file shipped is not. The earlier probe searched for the
/// truncation, so the truncation is what it failed to find — a defect reporting
/// itself as evidence that no source existed.
///
/// Truncating at "(iron, cobalt and nickel)" also turns the page's "A or B"
/// into a bare "A", a stronger claim than it makes. Pinned whole so the "or
/// inner transition metals (gadolinium)" clause cannot be trimmed back off.
#[test]
fn the_transition_metal_span_keeps_the_clause_that_was_truncated_off_it() {
    assert!(
        TRANS.ends_with("or inner transition metals (gadolinium)."),
        "the pinned span keeps the page's second disjunct"
    );
    for (tag, e) in [("egtfe", "iron"), ("egtni", "nickel"), ("egtco", "cobalt")] {
        assert_element(tag, e, "transition_metal", TRANS, L_TRANS);
    }
}

/// The envelope is the definition of a periodic-table group. It names no
/// element, so it warrants none of the 27 rows — and each family's sentence now
/// reaches exactly the rows it names, twice each (once under `citations`, once
/// under `steps`), instead of one sentence reaching all 27.
#[test]
fn the_framing_envelope_never_reaches_an_answer_and_is_pinned() {
    let dir = scratch("egenvelope");
    std::fs::copy(
        facts_stdlib().join("chemistry/element-groups.adj"),
        dir.join("element-groups.adj"),
    )
    .expect("copy shipped element-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-groups.adj\"\n? element_group_family($E, $F)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        27,
        "all 27 rows answer: {out}"
    );
    assert!(
        !out.contains("In chemistry, a group (also known as a family)"),
        "the framing span warrants no row: {out}"
    );
    // Each sentence reaches its own family and no further. Before #14986 the
    // alkali sentence's count here would have been 54.
    for (span, rows, family) in [
        (ALKALI, 6, "alkali_metal"),
        (AEM, 6, "alkaline_earth_metal"),
        (HALOGEN, 6, "halogen"),
        (NOBLE, 6, "noble_gas"),
        (TRANS, 3, "transition_metal"),
    ] {
        assert_eq!(
            out.matches(span).count(),
            rows * 2,
            "the {family} sentence warrants its {rows} rows and no others: {out}"
        );
    }
    // AND PIN THE ENVELOPE ITSELF — source, locator AND tier. A pin that stops
    // at `\n    locator` asserts only that a locator follows (#15183). No row
    // inherits the envelope's locator here, so nothing else can catch a
    // repointed one.
    let adj = std::fs::read_to_string(facts_stdlib().join("chemistry/element-groups.adj"))
        .expect("read shipped element-groups.adj");
    assert!(
        adj.contains(
            "    source \"In chemistry, a group (also known as a family)[1] is a column of elements in the periodic table of the chemical elements.\"\n    locator \"https://en.wikipedia.org/wiki/Group_(periodic_table)\"\n    trust consensus"
        ),
        "the envelope carries the page's definition of a group, verbatim"
    );
    // The three `cites` were PROMOTED into the rows they warrant, not dropped:
    // every span that was a corroboration is asserted above as some row's own
    // `source`, at the envelope's tier rather than untiered.
    assert!(
        !adj.contains("    cites "),
        "no corroboration survives at table level: each is now a row's own source"
    );
}
