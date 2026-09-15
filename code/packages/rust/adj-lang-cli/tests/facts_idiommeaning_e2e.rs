//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/idiom-meaning.adj`) driven through the built
//! CLI: a native `table` of 23 common English idioms and what each one
//! actually means, per Oxford International English's idioms article.
//! 0 answer-time model calls.
//!
//! Each row CITES the page's own "Meaning:" line for its idiom. `cites`, not
//! `source`: on the page the idiom is a numbered heading and the meaning a
//! separate paragraph that never names it -- #13934's held question, decided
//! the same way as `physics/energy-form-family.adj`. The envelope is the
//! page's definition of an idiomatic expression; it used to be
//! `piece_of_cake`'s own meaning, and so was the primary source of 22
//! answers it said nothing about.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_idiommeaning_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/idiom-meaning.adj");
    std::fs::copy(&src, dir.join("idiom-meaning.adj")).expect("copy shipped idiom-meaning.adj");
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/idiom-meaning.adj"))
        .expect("read shipped idiom-meaning.adj");
    adj[adj.find("table idiom_meaning").expect("table")..].to_string()
}

const LOCATOR: &str = "https://www.oxfordinternationalenglish.com/30-useful-english-idiomatic-expressions-their-meanings/";
const ENVELOPE: &str = "Idiomatic expressions are phrases with meanings that are different from the literal meanings of the words.";

/// (idiom, meaning, the page's own Meaning line) -- generated from the spans
/// the converter took from the page text, not retyped.
const ROWS: [(&str, &str, &str); 23] = [
    ("piece_of_cake", "very_easy_to_do", "Something very easy to do."),
    ("break_the_ice", "start_a_conversation", "To start a conversation or make people feel more comfortable."),
    ("under_the_weather", "feeling_slightly_ill", "Feeling slightly ill."),
    ("cut_corners", "done_the_easiest_or_cheapest_way_often_badly", "To do something in the easiest or cheapest way, often badly."),
    ("hit_the_nail_on_the_head", "describes_something_exactly_right", "To describe something exactly right."),
    ("cost_an_arm_and_a_leg", "very_expensive", "To be very expensive"),
    ("bite_off_more_than_you_can_chew", "try_to_do_more_than_you_can_manage", "To try to do more than you can manage."),
    ("beat_around_the_bush", "avoid_talking_about_what_is_important", "To avoid talking about what’s important."),
    ("cry_over_spilled_milk", "upset_about_something_already_happened_and_unchangeable", "To be upset about something that has already happened and can’t be changed."),
    ("get_your_act_together", "organise_yourself_and_improve_your_behaviour", "To organise yourself and improve your behaviour."),
    ("kill_two_birds_with_one_stone", "solve_two_problems_with_one_action", "To solve two problems with one action."),
    ("let_the_cat_out_of_the_bag", "reveal_a_secret_by_mistake", "To reveal a secret by mistake."),
    ("pull_someones_leg", "joke_with_someone_by_telling_them_something_untrue", "To joke with someone by telling them something that isn’t true."),
    ("burn_the_midnight_oil", "work_late_into_the_night", "To work late into the night."),
    ("bite_the_bullet", "do_something_difficult_or_unpleasant_being_avoided", "To do something difficult or unpleasant that you have been avoiding."),
    ("break_a_leg", "wish_someone_good_luck_especially_before_a_performance", "A way of wishing someone good luck, especially before a performance."),
    ("call_it_a_day", "stop_working_on_something", "To stop working on something."),
    ("steal_someones_thunder", "take_attention_away_from_someone_elses_achievement", "To take attention away from someone else’s achievement."),
    ("the_ball_is_in_your_court", "your_turn_to_take_action_or_make_a_decision", "It’s your turn to take action or make a decision."),
    ("throw_in_the_towel", "give_up", "To give up."),
    ("speak_of_the_devil", "said_when_someone_appears_as_you_are_talking_about_them", "Said when someone appears just as you’re talking about them."),
    ("once_in_a_blue_moon", "very_rarely", "Very rarely."),
    ("catch_someone_red_handed", "catch_someone_while_they_are_doing_something_wrong", "To catch someone while they are doing something wrong."),
];

/// The whole primary citation, as the serializer emits it.
fn primary() -> String {
    format!("\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\"")
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"idiom-meaning.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn idiom_meaning_recall_binds_the_meaning_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning(piece_of_cake, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"M\":\"very_easy_to_do\""),
        "piece of cake means very easy to do: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains(host) && contains(trust)` -- two
    // halves satisfiable by different parts of the output (#15209).
    assert!(
        out.contains(&primary()),
        "carries the Oxford International English citation, whole: {out}"
    );
}

#[test]
fn idiom_meaning_reverse_binds_the_idiom_for_that_meaning() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning($I, feeling_slightly_ill)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"I\":\"under_the_weather\""),
        "under the weather means feeling slightly ill: {out}"
    );
}

#[test]
fn idiom_meaning_abstains_honestly_on_an_untabled_idiom() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning(raining_cats_and_dogs, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "raining cats and dogs is a real idiom but not one of the tabled examples -- honest abstention, never invented: {out}"
    );
}

#[test]
fn idiom_meaning_extension_recalls_the_newly_added_rows() {
    let dir = scratch("ext");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"idiom-meaning.adj\"\n\
         ? idiom_meaning(let_the_cat_out_of_the_bag, $M)\n\
         ? idiom_meaning($I, give_up)\n\
         ? idiom_meaning(once_in_a_blue_moon, $M)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"M\":\"reveal_a_secret_by_mistake\""),
        "let the cat out of the bag → reveal a secret by mistake: {out}"
    );
    assert!(
        out.contains("\"I\":\"throw_in_the_towel\""),
        "throw in the towel means give up: {out}"
    );
    assert!(
        out.contains("\"M\":\"very_rarely\""),
        "once in a blue moon → very rarely: {out}"
    );
}

#[test]
fn every_idiom_is_corroborated_by_its_own_meaning_line() {
    // WHAT THIS CHANGE ADDED. Before it, every answer's only span was
    // `piece_of_cake`'s meaning.
    for (idiom, meaning, span) in ROWS {
        let out = ask(&format!("corr_{idiom}"), &format!("idiom_meaning({idiom}, $M)"));
        // ONE ANSWER, ASSERTED, so the negative arm below is about THIS
        // answer and cannot be masked by a sibling's citation (#15164).
        assert_eq!(
            out.matches("\"citations\":[").count(),
            1,
            "exactly one answer for {idiom}: {out}"
        );
        assert!(
            out.contains(&format!("\"M\":\"{meaning}\"")),
            "{idiom} still binds {meaning}: {out}"
        );
        // A CORROBORATION, NOT A SOURCE, pinned as one contiguous run: the
        // envelope stays the PRIMARY warrant, because the Meaning line never
        // names the idiom.
        assert!(
            out.contains(&format!(
                "{},\"corroborations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\"}}]",
                primary()
            )),
            "{idiom}: the envelope is primary and its own Meaning line corroborates: {out}"
        );
        // NEGATIVE ARM: no other idiom's line reaches this answer.
        for (other, _, other_span) in ROWS {
            if other != idiom {
                assert!(
                    !out.contains(other_span),
                    "the {other} line must not reach the {idiom} answer: {out}"
                );
            }
        }
    }
}

#[test]
fn no_row_carries_a_source_because_no_line_names_its_idiom() {
    // THE ZERO IS THE INSTRUMENT. On the page each idiom is a heading and its
    // meaning a separate paragraph; no line names both. A row `source` here
    // would assert a warrant none of these lines carries.
    let body = shipped_table();
    assert_eq!(body.matches("\n    row (").count(), 23, "twenty-three rows");
    // A construct, not a layout: every row line opens a block.
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("row (") && l.trim_end().ends_with('{'))
            .count(),
        23,
        "every row opens its own block"
    );
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("cites \"")).count(),
        23,
        "each row carries exactly one corroboration"
    );
    // INDENTATION-INSENSITIVE: a row `source` at any depth is counted.
    let source_lines: Vec<&str> = body
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect();
    assert_eq!(
        source_lines,
        vec![format!("    source \"{ENVELOPE}\"").as_str()],
        "exactly one `source` line in the table -- the envelope's"
    );
    // SEMANTIC too: the envelope is primary on a real answer.
    let out = ask("nosource", "idiom_meaning(break_a_leg, $M)");
    assert!(out.contains(&primary()), "the envelope is the PRIMARY source: {out}");
}

#[test]
fn the_envelope_defines_an_idiom_and_names_none() {
    let body = shipped_table();
    // The OLD envelope was one row's own meaning. It must not come back.
    assert!(
        !body.contains("source \"Something very easy to do.\""),
        "the envelope must not be piece_of_cake's meaning again"
    );
    assert!(body.contains(&format!("    source \"{ENVELOPE}\"")), "the envelope is shipped");
    // WHOLE PHRASES, read from the table. A word-level check passes vacuously
    // on `let_the_cat_out_of_the_bag`, whose every word is three letters or
    // fewer; the phrase itself cannot.
    let folded = ENVELOPE.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim();
            let phrase = key.replace('_', " ");
            keys += 1;
            assert!(
                !folded.contains(&phrase),
                "the envelope must name no idiom, but names {phrase:?}"
            );
        }
    }
    assert_eq!(keys, 23, "all twenty-three keys were actually checked");
    // And the table's own row lines agree with ROWS, so the constants above
    // cannot silently describe a different table.
    for (idiom, meaning, span) in ROWS {
        assert!(
            body.contains(&format!("    row ({idiom}, {meaning}) {{\n")),
            "row ({idiom}, {meaning}) is shipped"
        );
        assert!(
            body.contains(&format!("cites \"{span}\" locator \"{LOCATOR}\"")),
            "{idiom}'s shipped cites line is the page line"
        );
    }
}

#[test]
fn apostrophes_are_the_pages_own_curly_ones() {
    // The page writes U+2019 in all six of these; a straight apostrophe
    // occurs zero times in their Meaning lines. The header used to quote
    // them with straight ones while promising verbatim.
    let curly = ["beat_around_the_bush", "cry_over_spilled_milk", "pull_someones_leg", "steal_someones_thunder", "the_ball_is_in_your_court", "speak_of_the_devil"];
    let body = shipped_table();
    for (idiom, _, span) in ROWS {
        let expect_curly = curly.contains(&idiom);
        assert_eq!(
            span.contains('\u{2019}'),
            expect_curly,
            "{idiom}: curly apostrophe presence"
        );
        assert!(!span.contains('\''), "{idiom}: no straight apostrophe in a page line");
    }
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites \"") && l.contains('\'')),
        "no shipped cites line carries a straight apostrophe"
    );
}
