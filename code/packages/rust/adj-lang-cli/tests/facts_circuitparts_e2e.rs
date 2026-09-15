//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/circuit-parts.adj`) driven through the built CLI:
//! a native `table` of basic circuit part → the role its source states resolves
//! a binding-query recall with the MIT K-12 Maker citation, runs backward
//! (role → part), and abstains on something that is not one of the basic parts
//! (a capacitor) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factscp_{tag}_{}", std::process::id()));
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
fn physics_circuit_parts_recall_binds_role_with_citation() {
    let dir = scratch("circuitparts");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/circuit-parts.adj");
    std::fs::copy(&src, dir.join("circuit-parts.adj")).expect("copy shipped circuit-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"circuit-parts.adj\"\n\
         ? circuit_part_role(battery, $Role)\n\
         ? circuit_part_role(switch, $Role)\n\
         ? circuit_part_role(resistor, $Role)\n\
         ? circuit_part_role($Part, carries_current)\n\
         ? circuit_part_role(capacitor, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each part to the role the MIT handout states.
    assert!(
        out.contains("\"Role\":\"provides_dc_power\""),
        "battery → provides_dc_power: {out}"
    );
    assert!(
        out.contains("\"Role\":\"opens_or_closes\""),
        "switch → opens_or_closes: {out}"
    );
    assert!(
        out.contains("\"Role\":\"slows_current\""),
        "resistor → slows_current: {out}"
    );
    assert!(
        out.contains("circuit_part_role(battery, provides_dc_power)"),
        "battery is governing-bound to provides_dc_power: {out}"
    );
    // The relation runs BACKWARD: bind the role `carries_current`, recall the part.
    assert!(
        out.contains("\"Part\":\"wire\""),
        "carries_current → wire (reverse recall): {out}"
    );
    // ONE CONTIGUOUS SPAN, WHOLE LOCATOR INCLUDED. This was
    // `contains("k12maker.mit.edu") && contains("\"trust\":\"consensus\"")`,
    // and that pair is why two separate defects moved nothing it could see:
    // the table went from ONE envelope span warranting all seven rows to
    // seven row spans, and the locator went from a filename that NEVER
    // EXISTED to a capture of one that did -- and `k12maker.mit.edu` is a
    // substring of both addresses. Two loose needles, the #15139 shape.
    assert!(
        out.contains(
            "\"source\":\"Battery: A device that converts chemical energy into electrical energy and provides direct current (DC) power.\",\"locator\":\"https://web.archive.org/web/20221117212630if_/https://k12maker.mit.edu/uploads/9/7/5/8/97583140/circuit_components.pdf\",\"trust\":\"consensus\""
        ),
        "the battery row's whole warrant, contiguous: {out}"
    );
    // A capacitor is NOT one of the basic parts in this table — honest abstention.
    assert!(out.contains("\"abstained\":true"), "capacitor abstains: {out}");
}

const LOCATOR: &str = "https://web.archive.org/web/20221117212630if_/https://k12maker.mit.edu/uploads/9/7/5/8/97583140/circuit_components.pdf";
const ENVELOPE: &str = "Battery: A device that converts chemical energy into electrical energy and provides direct current (DC) power.";

/// The address the table shipped for years. It 404s, and the Wayback CDX
/// index holds NO capture of it at any time, while `circuit_basics.pdf` and
/// `circuit_components.pdf` -- the two names it runs together -- both
/// returned 200 and are archived.
const NEVER_EXISTED: &str = "circuit_basics_and_components.pdf";

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    let src = facts_stdlib().join("physics/circuit-parts.adj");
    std::fs::copy(&src, dir.join("circuit-parts.adj")).expect("copy shipped circuit-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"circuit-parts.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

const PARTS: [(&str, &str, &str); 7] = [
        ("battery", "provides_dc_power", "Battery: A device that converts chemical energy into electrical energy and provides direct current (DC) power."),
        ("wire", "carries_current", "Conductor: Any material, usually metal, that carries an electrical current, such as wire, alligator clips, or copper tape."),
        ("switch", "opens_or_closes", "The handle moves side to side to open or close the circuit."),
        ("lamp", "converts_to_light", "Lamp: This traditional device converts electrical energy into light (and lots of heat) using a resistive filament."),
        ("resistor", "slows_current", "Resistor: Electronic component that slows the flow of electricity."),
        ("motor", "converts_to_motion", "Motor: There are many types of electric motors, but all convert electrical energy into rotary motion of an output shaft."),
        ("led", "gives_off_light", "LED: Light Emitting Diodes are components that give off light when current goes through in the correct direction, as indicated by the “arrow” in the symbol."),
];

#[test]
fn every_part_is_warranted_by_its_own_definition() {
    // THIS IS THE CONVERSION. The BATTERY sentence used to be the envelope
    // `source` and therefore the warrant for all seven rows, so
    // `? circuit_part_role(resistor, $R)` came back proved by a sentence
    // about batteries. Six of the seven were in that position.
    for (part, role, span) in PARTS {
        let out = ask(&format!("warrant_{part}"), &format!("circuit_part_role({part}, $R)"));
        // ONE ANSWER, ASSERTED, so the negative arm below is about THIS
        // answer and cannot be masked by a sibling's citation (#15164).
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {part}: {out}"
        );
        assert!(
            out.contains(&format!("\"R\":\"{role}\"")),
            "{part} still binds {role}: {out}"
        );
        assert!(
            out.contains(&format!(
                "\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\""
            )),
            "{part}'s warrant is its own definition, whole: {out}"
        );
        if part != "battery" {
            assert!(
                !out.contains(ENVELOPE),
                "the battery sentence must not warrant {part}: {out}"
            );
        }
    }
}

#[test]
fn the_envelope_span_reaches_no_answer_beyond_the_battery_row() {
    // The envelope keeps the BATTERY span because this source is a definition
    // table with no prose framing it, and `source` is a required field. Every
    // row overrides it, so it reaches answers ONLY through the battery row's
    // own copy -- which is why the count below is 2 (once under `citations`,
    // once under `steps`) and not 14.
    let out = ask("envelope", "circuit_part_role($P, $R)");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        7,
        "all seven parts answered: {out}"
    );
    assert_eq!(
        out.matches(ENVELOPE).count(),
        2,
        "the envelope span reaches exactly one answer -- the battery row's: {out}"
    );
    // THE COUNT ALONE HAS A BLIND SPOT, so this test does not rest on it.
    // The envelope and the battery row's span are the SAME STRING, so
    // deleting only the battery row's block leaves the count at 2 -- the
    // envelope would simply be covering that row again, which is the shape
    // this test exists to rule out. The structural test catches it by
    // counting source blocks; asserting it here too means this test stands
    // on its own.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/circuit-parts.adj"))
        .expect("read shipped circuit-parts.adj");
    assert_eq!(
        adj.matches("\n        source \"").count(),
        7,
        "all seven rows still carry their own source block"
    );
    // POSITIVE CONTROL: the other six spans are each emitted twice, so the
    // count above is a measurement and not a program that printed nothing.
    for (part, _, span) in PARTS {
        assert_eq!(
            out.matches(span).count(),
            2,
            "{part}'s span appears once per citations and once per steps: {out}"
        );
    }
}

#[test]
fn the_locator_is_not_the_filename_that_never_existed() {
    // A NAMED REGRESSION GUARD, because "this exact address was never real"
    // is precisely the sort of fact someone undoes while tidying a URL back
    // to what looks like its original form. The shipped address 404s AND has
    // no Wayback capture at any time; the two files whose names it runs
    // together both do.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/circuit-parts.adj"))
        .expect("read shipped circuit-parts.adj");
    // FORBIDDEN AS A LOCATOR VALUE, not as prose. The header quotes the
    // conflated filename while explaining the defect, which is the record
    // and must survive; what must never come back is that string in a
    // `locator`. (Written bluntly first, which is how the stale provenance
    // listing two hundred lines down -- still naming the dead URL as "the
    // same primary source" -- got found.)
    for line in adj.lines() {
        if line.split_whitespace().next() == Some("locator") {
            assert!(
                !line.contains(NEVER_EXISTED),
                "the conflated filename must not come back as a locator: {line:?}"
            );
        }
    }
    assert!(
        !adj.contains(&format!("97583140/{NEVER_EXISTED}")),
        "nor as a bare source URL anywhere: {NEVER_EXISTED}"
    );
    assert!(
        adj.contains(LOCATOR),
        "the locator is the verified capture of circuit_components.pdf"
    );
    // And it reaches the answers, not just the file.
    let out = ask("locator", "circuit_part_role(led, $R)");
    assert!(
        out.contains(LOCATOR),
        "the capture address is what an answer carries: {out}"
    );
    assert!(
        !out.contains(NEVER_EXISTED),
        "and the never-existent one is not: {out}"
    );
}

#[test]
fn every_row_carries_its_own_distinct_span_and_none_restates_the_locator() {
    // STRUCTURAL, READ FROM THE SHIPPED FILE.
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/circuit-parts.adj"))
        .expect("read shipped circuit-parts.adj");
    let body = &adj[adj.find("table circuit_part_role").expect("table")..];

    assert_eq!(body.matches("\n    row (").count(), 7, "seven rows");
    let mut spans: Vec<&str> = body
        .lines()
        .filter_map(|l| l.strip_prefix("        source \""))
        .map(|r| r.trim_end_matches(0x22 as char))
        .collect();
    assert_eq!(spans.len(), 7, "every row carries its own source block");
    let before = spans.len();
    spans.sort_unstable();
    spans.dedup();
    assert_eq!(spans.len(), before, "all seven spans are distinct: {spans:?}");

    // NO SPAN NAMES ANOTHER ROW'S PART. Checked against keys read from the
    // table, folded on both sides. `wire`'s span defines "Conductor" and
    // names wire as an example -- that is the page's wording, and it names
    // no OTHER part.
    let keys: Vec<String> = body
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("row ("))
        .map(|r| r.split(',').next().expect("row key").trim().to_lowercase())
        .collect();
    assert_eq!(keys.len(), 7, "seven row keys: {keys:?}");
    for (part, _, span) in PARTS {
        let low = span.to_lowercase();
        for k in &keys {
            if *k == part {
                continue;
            }
            assert!(
                !low.contains(k.as_str()),
                "the {part} span must name no other part, but names {k:?}"
            );
        }
    }

    // TOTAL DERIVATIONS: exactly one table-level source and locator, and no
    // row restates either -- all seven spans are in the one captured file.
    let table_sources: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    source \""))
        .collect();
    assert_eq!(table_sources.len(), 1, "exactly one table-level source");
    let table_locators: Vec<&str> = adj
        .lines()
        .filter(|l| l.starts_with("    locator \""))
        .collect();
    assert_eq!(table_locators.len(), 1, "exactly one table-level locator");
    assert_eq!(
        body.matches("\n        locator \"").count(),
        0,
        "no row restates the locator"
    );
    let envelope = table_sources[0]
        .trim_start()
        .trim_start_matches("source \"")
        .trim_end_matches(0x22 as char);
    assert_eq!(
        envelope, ENVELOPE,
        "the envelope keeps the battery span, disclosed as such"
    );
    assert!(
        adj.contains("    columns part, role"),
        "the shipped column names are unchanged"
    );
}
