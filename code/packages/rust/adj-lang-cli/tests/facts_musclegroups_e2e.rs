//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/muscle-groups.adj`) driven through the built CLI:
//! a native `table` of skeletal muscle → body region resolves a binding-query
//! recall with the source's Wikipedia (consensus) citation, runs the relation
//! backward (region → every muscle in it, one-to-many), and abstains on a
//! non-muscle (the femur, a bone) — 0 model calls.
//!
//! Each row carries its OWN sentence and its OWN article: every sentence names
//! both its muscle and its region. The envelope used to be the biceps sentence,
//! with the other eight as table-level `cites` reaching every answer. One span
//! was never on its page: the pectoralis major sentence has a NON-BREAKING
//! SPACE (U+00A0) between "pectus" and "'breast'".

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmuscle_{tag}_{}", std::process::id()));
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

const ENVELOPE: &str = "The muscle is made up of muscle fascicles lying parallel with one another, and are collected together into larger bundles separated by fibrous septa.";
const ENVELOPE_LOCATOR: &str = "https://en.wikipedia.org/wiki/Gluteus_maximus_muscle";
const OLD_ENVELOPE: &str = "The biceps or biceps brachii (Latin: musculus biceps brachii, \"two-headed muscle of the arm\") is a large muscle that lies on the front of the upper arm between the shoulder and the elbow.";
/// The pectoralis phrase as it used to ship, with an ordinary space.
const PECT_PLAIN: &str = "from Latin pectus 'breast'";

/// (muscle, region, that row's own sentence, that row's own article) --
/// generated from the converter's page-verified spans, not retyped.
const ROWS: [(&str, &str, &str, &str); 9] = [
    ("biceps_brachii", "arm", "The biceps or biceps brachii (Latin: musculus biceps brachii, \"two-headed muscle of the arm\") is a large muscle that lies on the front of the upper arm between the shoulder and the elbow.", "https://en.wikipedia.org/wiki/Biceps"),
    ("triceps_brachii", "arm", "The triceps, or triceps brachii (Latin for \"three-headed muscle of the arm\"), is a large muscle on the back of the upper limb of many vertebrates.", "https://en.wikipedia.org/wiki/Triceps"),
    ("deltoid", "shoulder", "The deltoid muscle (or musculus deltoideus) is the muscle[1] forming the rounded contour of the human shoulder.", "https://en.wikipedia.org/wiki/Deltoid_muscle"),
    ("pectoralis_major", "chest", "The pectoralis major (from Latin pectus\u{a0}'breast') is a thick, fan-shaped or triangular convergent muscle of the human chest.", "https://en.wikipedia.org/wiki/Pectoralis_major"),
    ("rectus_abdominis", "abdomen", "The rectus abdominis, (Latin: straight abdominal) also known as the \"abdominal muscle\" or simply better known as the \"abs\", and sometimes informally referred to as the \"six-pack\", is a pair of segmented skeletal muscle on the ventral aspect of a person's abdomen.", "https://en.wikipedia.org/wiki/Rectus_abdominis_muscle"),
    ("gluteus_maximus", "hip", "The gluteus maximus is the main extensor muscle of the hip in humans.", "https://en.wikipedia.org/wiki/Gluteus_maximus_muscle"),
    ("quadriceps", "thigh", "The quadriceps femoris muscle (/ˈkwɒdrɪsɛps ˈfɛmərɪs/, also called the quadriceps extensor, quadriceps or quads) is a large muscle group that includes the four prevailing muscles on the front of the thigh.", "https://en.wikipedia.org/wiki/Quadriceps"),
    ("sartorius", "thigh", "The sartorius muscle (/sɑːrˈtɔːriəs/), historically known as couturier (French for \"tailor\"), is the longest muscle in the human body.[2] It is a long, thin, superficial muscle that runs down the length of the thigh in the anterior compartment.", "https://en.wikipedia.org/wiki/Sartorius_muscle"),
    ("gastrocnemius", "leg", "The gastrocnemius muscle (plural gastrocnemii) is a superficial two-headed muscle. It is located superficial to the soleus in the posterior (back) compartment of the leg.", "https://en.wikipedia.org/wiki/Gastrocnemius_muscle"),
];

fn row(muscle: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    *ROWS.iter().find(|r| r.0 == muscle).expect("known muscle")
}

/// A string as the serializer writes it inside JSON: a quote becomes \".
fn json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The whole primary citation a row's answer must carry.
fn row_citation(muscle: &str) -> String {
    let (_, _, sentence, locator) = row(muscle);
    format!(
        "\"source\":\"{}\",\"locator\":\"{locator}\",\"trust\":\"consensus\",\"corroborations\":[]",
        json(sentence)
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/muscle-groups.adj"))
        .expect("read shipped muscle-groups.adj");
    adj[adj.find("table muscle_region").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    std::fs::copy(facts_stdlib().join("anatomy/muscle-groups.adj"), dir.join("muscle-groups.adj"))
        .expect("copy shipped muscle-groups.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"muscle-groups.adj\"\n? {query}\n")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn anatomy_muscle_groups_recall_binds_region_with_citation() {
    let dir = scratch("musclegroups");
    let src = facts_stdlib().join("anatomy/muscle-groups.adj");
    std::fs::copy(&src, dir.join("muscle-groups.adj")).expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n\
         ? muscle_region(biceps_brachii, $R)\n\
         ? muscle_region(deltoid, $R)\n\
         ? muscle_region(quadriceps, $R)\n\
         ? muscle_region($M, arm)\n\
         ? muscle_region(femur, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"R\":\"arm\""), "biceps_brachii → arm: {out}");
    assert!(out.contains("\"R\":\"shoulder\""), "deltoid → shoulder: {out}");
    assert!(out.contains("\"R\":\"thigh\""), "quadriceps → thigh: {out}");
    assert!(
        out.contains("\"M\":\"biceps_brachii\"") && out.contains("\"M\":\"triceps_brachii\""),
        "arm → biceps_brachii AND triceps_brachii (reverse recall): {out}"
    );
    // WHOLE CONTIGUOUS RUNS, not `contains("en.wikipedia.org") && contains(trust)`
    // (#15209's two-loose-needles shape).
    for muscle in ["biceps_brachii", "deltoid", "quadriceps", "triceps_brachii"] {
        assert!(out.contains(&row_citation(muscle)), "{muscle}'s own sentence and article: {out}");
    }
    assert!(out.contains("\"abstained\":true"), "unknown muscle abstains: {out}");
}

#[test]
fn muscle_groups_deltoid_answer_keeps_its_footnote_marker() {
    let out = ask("cite_delt", "muscle_region(deltoid, $R)");
    let (_, _, sentence, _) = row("deltoid");
    // The header once quoted this without the page's "[1]" -- real rendered
    // text. Pinned now on deltoid's OWN primary source.
    assert!(sentence.contains("muscle[1] forming"), "the constant keeps the marker");
    assert!(out.contains(&row_citation("deltoid")), "deltoid's answer keeps the page's footnote marker: {out}");
}

#[test]
fn muscle_groups_quadriceps_answer_keeps_the_parenthetical_its_header_had_deleted() {
    let out = ask("cite_quad", "muscle_region(quadriceps, $R)");
    let (_, _, sentence, _) = row("quadriceps");
    // THE WORST HEADER-QUOTE SUBTYPE FOUND: the IPA-and-alias parenthetical
    // was once deleted with no ellipsis at all. Pinned on quadriceps' own
    // primary source.
    assert!(sentence.contains("also called the quadriceps extensor, quadriceps or quads)"), "the constant keeps it");
    assert!(out.contains(&row_citation("quadriceps")), "quadriceps' answer keeps the parenthetical: {out}");
}

#[test]
fn muscle_groups_arm_reverse_answer_cites_each_muscles_own_article() {
    // This test used to pin "all eight corroborations in order" on the arm
    // answer -- a description of the defect this change removes. The arm
    // reverse recall now yields two answers, each with its own article, and
    // neither carries any other muscle's sentence.
    let out = ask("cite_arm", "muscle_region($M, arm)");
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers: {out}");
    assert!(out.contains(&row_citation("biceps_brachii")), "biceps answer: {out}");
    assert!(out.contains(&row_citation("triceps_brachii")), "triceps answer: {out}");
    for (muscle, _, sentence, _) in ROWS {
        if muscle != "biceps_brachii" && muscle != "triceps_brachii" {
            assert!(!out.contains(&json(sentence)), "{muscle}'s sentence must not reach the arm answers: {out}");
        }
    }
}

#[test]
fn muscle_groups_biceps_citation_keeps_the_pages_latin_gloss() {
    let out = ask("biceps_gloss_4d", "muscle_region(biceps_brachii, $R)");
    let (_, _, sentence, _) = row("biceps_brachii");
    assert!(sentence.contains("(Latin: musculus biceps brachii, \"two-headed muscle of the arm\")"), "the constant keeps it");
    assert!(out.contains(&row_citation("biceps_brachii")), "the biceps citation matches its page: {out}");
}

#[test]
fn every_muscle_is_warranted_by_its_own_articles_sentence() {
    for (muscle, region, sentence, _) in ROWS {
        let out = ask(&format!("row_{muscle}"), &format!("muscle_region({muscle}, $R)"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {muscle}: {out}");
        assert!(out.contains(&format!("\"R\":\"{region}\"")), "{muscle} → {region}: {out}");
        assert!(out.contains(&row_citation(muscle)), "{muscle}: own sentence and article: {out}");
        for (other, _, other_sentence, _) in ROWS {
            if other != muscle && other_sentence != sentence {
                assert!(!out.contains(&json(other_sentence)), "the {other} sentence must not reach {muscle}: {out}");
            }
        }
        assert!(!out.contains(&json(ENVELOPE)), "the framing envelope warrants no row, including {muscle}: {out}");
    }
}

#[test]
fn the_pectoralis_sentence_carries_the_pages_non_breaking_space() {
    let (_, _, sentence, _) = row("pectoralis_major");
    assert_eq!(sentence.matches('\u{a0}').count(), 1, "exactly one U+00A0");
    let plain_sentence = sentence.replace('\u{a0}', " ");
    assert!(plain_sentence.contains(PECT_PLAIN), "it differs from the old string only there");
    let out = ask("nbsp", "muscle_region(pectoralis_major, $R)");
    assert!(out.contains(&row_citation("pectoralis_major")), "the answer carries the page's character: {out}");
    assert!(!out.contains(&json(&plain_sentence)), "the ordinary-space string reaches no answer: {out}");
    // SCOPED TO `source "` LINES (#15415), the correction #15337, #15338 and
    // #15417 made for their tables. `shipped_table()` runs to end of file and
    // four `%` comment lines sit inside this block, so, read against that slice,
    // this arm FAILED A CORRECT FILE when a comment quoted the old phrase --
    // measured by mutant, not reasoned. A comment is not a shipped citation.
    //
    // PECT_PLAIN is a FRAGMENT, not a whole sentence, so this still guards
    // every `source` line in the table, not only the pectoralis row. The
    // trade: nothing now pins "not anywhere in the block".
    let source_lines: String = shipped_table()
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!source_lines.contains(PECT_PLAIN), "and no `source` line ships it: {source_lines}");
}

#[test]
fn the_table_shape_is_nine_sources_with_their_own_articles() {
    let body = shipped_table();
    let row_sources = body.lines().filter(|l| l.starts_with("        source \"")).count();
    let row_locators = body.lines().filter(|l| l.starts_with("        locator \"")).count();
    assert_eq!((row_sources, row_locators), (9, 9), "each row restates source and locator");
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("source \"")).count(),
        10,
        "nine row sources and one envelope, at any indentation"
    );
    assert!(!body.lines().any(|l| l.trim_start().starts_with("cites ")), "no corroboration anywhere");
    assert!(!body.lines().any(|l| l.starts_with("        trust ")), "no row restates trust");
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{ENVELOPE_LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the framing sentence at its article"
    );
    assert!(!body.contains(&format!("\n    source \"{}\"", json(OLD_ENVELOPE))), "the envelope is not the biceps sentence again");
    let folded = ENVELOPE.to_lowercase();
    for (muscle, region, _, _) in ROWS {
        let first = muscle.split('_').next().expect("word");
        assert!(!folded.contains(first), "the envelope must name no muscle, but names {first:?}");
        assert!(!folded.contains(region), "the envelope must name no region, but names {region:?}");
    }
    for (muscle, region, sentence, locator) in ROWS {
        assert!(
            body.contains(&format!("    row ({muscle}, {region}) {{\n"))
                && body.contains(&format!("        source \"{}\"\n        locator \"{locator}\"\n    }}", json(sentence))),
            "row ({muscle}, {region}) is shipped with its own sentence and article"
        );
    }
}
