//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/sun-layer.adj`) driven through the built
//! CLI: a native `table` naming two layers of the Sun and what each
//! actually is, quoted verbatim from NASA's "Layers of the Sun" blog
//! post. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the `photosphere` span, so a `corona` recall was warranted primarily by a
//! sentence about the photosphere.
//!
//! BOTH ROW SPANS CARRY NON-ASCII BYTES. The photosphere span's dash is
//! U+2013; the corona span carries U+2013 and the U+2019 in "the Sun's".
//! Counted on the fetched page, the ASCII spelling of each occurs ZERO times,
//! so shipping one would cite a sentence the source does not contain. The
//! consts below use `\u{2013}` / `\u{2019}` escapes rather than literal bytes,
//! because a terminal, a grep result and a diff view all render those
//! characters as an ordinary hyphen and apostrophe.
//!
//! The envelope is plain ASCII and names NEITHER layer.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_sun_layer_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/sun-layer.adj");
    std::fs::copy(&src, dir.join("sun-layer.adj")).expect("copy shipped sun-layer.adj");
}

const LOCATOR: &str = "https://science.nasa.gov/blogs/the-sun-spot/2023/09/26/layers-of-the-sun/";

/// The page's framing sentence. Names neither layer, so it warrants neither
/// row by itself. Plain ASCII, unlike both row spans.
const ENVELOPE: &str = "The Sun and its atmosphere consist of several zones or layers.";

const PHOTOSPHERE: &str = "The Photosphere \u{2013} the visible surface of the Sun.";
const CORONA: &str = "The Corona \u{2013} the Sun\u{2019}s outer atmosphere.";

/// (layer, its description atom, the NASA sentence stating that description)
fn scale() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("photosphere", "the_visible_surface_of_the_sun", PHOTOSPHERE),
        ("corona", "the_suns_outer_atmosphere", CORONA),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/sun-layer.adj"))
        .expect("read shipped sun-layer.adj");
    adj[adj.find("table sun_layer").expect("table")..].to_string()
}

#[test]
fn sun_layer_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sun-layer.adj\"\n\
         ? sun_layer(photosphere, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"the_visible_surface_of_the_sun\""),
        "photosphere means the_visible_surface_of_the_sun: {out}"
    );
    // THIS PIN WAS `contains("science.nasa.gov") &&
    // contains("\"trust\":\"authoritative\"")` -- satisfied by ANY NASA
    // citation, constraining no sentence text, and before the conversion
    // satisfied by the photosphere span riding on the corona answer too. Now
    // the answer is pinned to its own whole citations array.
    assert!(
        out.contains(&only_citation(PHOTOSPHERE)),
        "the photosphere answer carries the sentence about the photosphere: {out}"
    );
    assert!(
        !out.contains(ENVELOPE),
        "the envelope's wording is primary for no answer: {out}"
    );
}

#[test]
fn sun_layer_reverse_binds_the_layer_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sun-layer.adj\"\n\
         ? sun_layer($L, the_suns_outer_atmosphere)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"L\":\"corona\""),
        "the shipped the_suns_outer_atmosphere example is corona: {out}"
    );
    // THE WHOLE DEFECT, AS A NEEDLE. Before this change the corona answer
    // carried the PHOTOSPHERE sentence as its warrant.
    assert!(
        out.contains(&only_citation(CORONA)),
        "the corona answer carries the sentence about the corona: {out}"
    );
    assert!(
        !out.contains(PHOTOSPHERE),
        "the photosphere sentence must not warrant the corona answer: {out}"
    );
}

#[test]
fn sun_layer_abstains_honestly_on_an_untabled_layer() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sun-layer.adj\"\n\
         ? sun_layer(chromosphere, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "chromosphere is a real solar layer the same source names, but its sentence bundles position with a temperature range rather than one clean fact -- honest abstention, never invented: {out}"
    );
    // An abstaining query emits no citations array at all.
    assert_eq!(
        out.matches("\"citations\":[").count(),
        0,
        "an abstention carries no citation: {out}"
    );
}

#[test]
fn every_layer_answer_carries_only_the_sentence_about_that_layer() {
    for (layer, description, span) in scale() {
        let dir = scratch(&format!("layer_{layer}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"sun-layer.adj\"\n? sun_layer({layer}, $D)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "one answer for {layer}: {out}"
        );
        assert!(
            out.contains(&format!("\"D\":\"{description}\"")),
            "{layer} -> {description}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{layer}: the NASA sentence about it, whole, and the only citation: {out}"
        );
        // WHOLE SPANS. The two spans share only "the" and "sun", so no short
        // needle is exclusive to either row.
        for other in [PHOTOSPHERE, CORONA] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "the other layer's sentence must not reach {layer}: {out}"
                );
            }
        }
        assert!(
            !out.contains(ENVELOPE),
            "the envelope is not primary for {layer}: {out}"
        );
    }
}

#[test]
fn the_row_spans_keep_the_pages_own_dash_and_apostrophe() {
    // Counted on the fetched page: each span's ASCII spelling occurs ZERO
    // times, so shipping one would cite a sentence the source does not
    // contain. This arm is worth more than it looks, because a terminal, a
    // ripgrep result and a diff view ALL render U+2013 as "-" and U+2019 as
    // "'" -- a reviewer reading any of them cannot tell the two apart.
    let body = shipped_table();

    // POSITIVE: the row `source` lines carry the real bytes. Scoped to
    // eight-space lines, because this claim is about the ROWS.
    let row_lines: String = body
        .lines()
        .filter(|l| l.starts_with("        source \""))
        .collect::<Vec<_>>()
        .join("\n");
    for span in [PHOTOSPHERE, CORONA] {
        assert!(
            row_lines.contains(span),
            "a row ships this span with the page's own bytes: {span:?} not in {row_lines:?}"
        );
    }

    // NEGATIVE: no shipped citation at ANY indent may carry an ASCII twin.
    // Scoped to `source` lines rather than the whole block, because a `%`
    // comment quoting a span in ASCII is not a shipped citation.
    let source_lines: String = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    for span in [PHOTOSPHERE, CORONA] {
        let ascii = span.replace('\u{2013}', "-").replace('\u{2019}', "'");
        assert!(
            !source_lines.contains(&ascii),
            "no `source` line may carry an ASCII twin -- that spelling occurs zero \
             times on the page: {ascii:?}"
        );
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(
        body.matches("\n        source \"").count(),
        2,
        "two row sources"
    );
    // Keyword-anchored so a `cites` at any indent fails. SCOPE: this table's
    // rows differ only in `source`. A row-level `cites` is legal ADJ, so this
    // pins a convention local to this table, not a language rule.
    //
    // MEASURED 2026-09-16, NOT INHERITED: 19 shipped fact files carry a
    // row-level `cites` (86 such lines), over 362 files matching
    // adj-facts-stdlib/**/*.adj with CHANGELOG.d and *.query.adj excluded. Two
    // predicates agree on the same file set -- a `cites` at 8-space indent, and
    // a `cites` inside a real `row (...) {` block -- because every row-level one
    // in this corpus is written at 8 spaces, while a TABLE-level `cites` sits at
    // 4 spaces before the table's closing brace (122 such lines, all legal).
    //
    // THE 8-AND-4 PAIR IS NOT EXHAUSTIVE, and a predicate that treats it as if
    // it were returns 87 rather than 86. TWO do: relaxing the indent rule from
    // `== 8` to `> 4`, and asking merely that the innermost construct is not
    // `table`. Each picks up one `cites` at 7-space indent inside a top-level
    // `rule { }` block in geometry/shape-composition.adj -- neither row- nor
    // table-level -- and adds nothing else. The full account is
    //
    //     row 86 + table 122 + rule 1 = 209 `cites` corpus-wide
    //
    // Shard 03620 published 87 as its row-level count and now carries a dated
    // retraction. WHICH of the two rules it ran is not recoverable from the
    // shard, so neither is named here as the cause.
    //
    // This comment used to say "18 shipped tables use one" with no predicate and
    // no denominator (#15378). THIS FILE IS THE FIFTH COPY, and the issue named
    // only four: it enumerated the sites its author had tripped over rather than
    // scanning for them -- the very defect it was filed to correct.
    //
    // A sweep keyed on the CLAIM CLASS rather than on one phrasing fires on TEN
    // files, EIGHT of which needed correcting; the count of corrections rose
    // three times while that sweep was being repaired:
    //
    //   four  -- the literal phrase "18 shipped tables"
    //   five  -- + comment prefixes stripped and whitespace flattened, which
    //            found THIS file, where the phrase wraps a line break
    //   six   -- + keyed on the claim class: "85 row-level `cites` lines"
    //   seven -- + shard 03580, a third phrasing again ("18 have a row-level
    //            `cites`, 12 a row-level `locator`, 5 a row-level `trust`")
    //   eight -- + chemistry/states-of-matter.adj, "209 `cites` in all, 87 of
    //            them inside a `row` block"
    //
    // Every rise came from widening the needle, never from looking harder at
    // the sites already known -- the whole argument for keying on the claim
    // rather than on the sentence. The eighth is the sharpest instance: a
    // security review found it by running the needle the way the shipped prose
    // DESCRIBED it ("any number near a cites-count noun") rather than the way
    // it was implemented (a fixed list of nouns that lacked both "cites in all"
    // and "inside a row block"). The sweep had reported that file clean.
    //
    // The sweep fires on FIFTEEN files. Five state a different predicate -- a
    // per-domain or per-table count rather than this corpus-wide claim -- which
    // leaves TEN carrying the claim class, and all ten are edited here. Every
    // narrowing after the fifteen is my triage, not the instrument's: eight
    // carried an inherited copy and are re-measured; CHANGELOG.d/03620 is the
    // ORIGIN of the 87 rather than a copy, so it gets a dated retraction; and
    // facts_plateboundaries_e2e.rs already stated the number with its
    // predicate, so only its prose changed.
    //
    // This comment ALSO used to cite "a loose scan over 4,109 files". That
    // denominator could not be reproduced and has been withdrawn rather than
    // re-derived into something that sounds better: the sweep that finds these
    // sites runs over the two trees that carry the claim,
    // code/packages/rust/adj-lang-cli/tests and code/specs/data/adj-facts-stdlib,
    // and prints its own file count when it runs. A number nobody can reproduce
    // is the defect, whether it is the count or the denominator.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "this table ships no corroboration at any indent: {body}"
    );
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
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's framing sentence, at the authoritative tier"
    );
    assert!(
        !body.contains(&format!("\n    source \"{PHOTOSPHERE}\"\n    locator")),
        "not the photosphere span as the envelope again"
    );
    // The envelope must name NO LAYER OF THE SUN -- not merely no row key.
    // Whole words, because "corona" is a substring of "coronal" and the page
    // uses both.
    //
    // THE LIST IS THE PAGE'S WHOLE LAYER TAXONOMY, and that is deliberate.
    // Forbidding only the two row keys leaves the frame free to be about some
    // OTHER layer: a mutant swapping in the page's own "The Convection Zone –
    // the outermost layer of the solar interior…" is real page text, names
    // neither row key, and would frame a table it has nothing to do with.
    // This test already forbade `chromosphere`, which is not a row key either,
    // so extending to the remaining four is the same rule applied
    // consistently rather than a new one. The names are exactly those this
    // table's own header enumerates as the page's six layers.
    //
    // A legitimate framing sentence names the set, not a member -- so this
    // fails on no correct envelope.
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
        "photosphere",
        "corona",
        "chromosphere",
        "core",
        "radiative",
        "convection",
        "transition",
    ] {
        assert!(
            !tokens.contains(&word),
            "the envelope must name no layer of the Sun, but contains {word:?} \
             as a whole word: {shipped_envelope:?}"
        );
    }
}

#[test]
fn every_row_description_is_supported_by_the_span_that_row_carries() {
    // The "does the span name its row key?" check (#15318).
    let body = shipped_table();
    for (layer, description, span) in scale() {
        // ANCHORED ON THE ROW HEADER, not the bare `source` line: the weaker
        // needle proves a span sits among the eight-space source lines
        // SOMEWHERE, not that it belongs to THIS row. On `mixture-types` a
        // mutant swapping two rows' spans satisfied the weaker needle fully.
        assert!(
            body.contains(&format!(
                "row ({layer}, {description}) {{\n        source \"{span}\"\n"
            )),
            "{layer} carries its own span IN ITS OWN ROW: {body}"
        );
        let normalized: String = span
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        let tokens: Vec<&str> = normalized.split(' ').filter(|t| !t.is_empty()).collect();
        assert!(
            tokens.contains(&layer),
            "{layer}: its own span must name the layer as a whole word, but it is \
             not a token of {normalized:?}"
        );
        // And it must state the content the atom compresses. EXHAUSTIVE, no
        // catch-all: a `_` arm would silently apply one row's needle to any row
        // added later.
        let content: &[&str] = match layer {
            "photosphere" => &["visible", "surface"],
            "corona" => &["outer", "atmosphere"],
            other => panic!("no content needle registered for row {other}"),
        };
        for needle in content {
            assert!(
                normalized.contains(*needle),
                "{layer}: its span must state {needle:?}, the content the atom \
                 {description:?} compresses, but it is not in {normalized:?}"
            );
        }
    }
}
