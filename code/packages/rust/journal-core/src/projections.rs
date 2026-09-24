//! Projections — everything a screen shows that can be *derived* from the state.
//!
//! None of this is stored. Each function reads a [`JournalState`] and returns a
//! fresh answer, so a projection can never go stale: change an entry and the next
//! call reflects it.
//!
//! | projection | the screen that needs it |
//! | --- | --- |
//! | [`timeline`] | the main list — entries grouped under their day |
//! | [`on_this_day`] | "one year ago today" recall |
//! | [`search`] | the search box and its results |
//! | [`tag_counts`] | the tag browser |
//! | [`month_activity`] | a calendar with a dot on each day that has entries |
//!
//! Every projection takes an [`EntryFilter`], so "the Work journal, tagged
//! #travel, starred only" is one value that means the same thing everywhere.
//!
//! ## Ordering is total
//!
//! Wherever entries are listed, the order is: newer **day** first; within a day,
//! newer `created_at_ms` first; and finally the entry id. The id tiebreak matters —
//! two entries imported in one batch share a timestamp, and without it they would
//! swap places between backends.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::date::days_in_month;
use crate::{text, Date, Entry, EntryId, JournalId, JournalState, Tag};

/// Which entries a projection considers. The default considers all of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", default))]
pub struct EntryFilter {
    /// Only entries in this journal.
    pub journal: Option<JournalId>,
    /// Only entries carrying this tag.
    pub tag: Option<Tag>,
    /// Only starred entries.
    pub starred_only: bool,
}

impl EntryFilter {
    /// True if `e` passes every condition.
    pub fn accepts(&self, e: &Entry) -> bool {
        self.journal.as_ref().is_none_or(|j| &e.journal == j)
            && self.tag.as_ref().is_none_or(|t| e.has_tag(t))
            && (!self.starred_only || e.starred)
    }
}

/// The canonical list order: newest day, then newest instant, then id.
fn newest_first(a: &Entry, b: &Entry) -> Ordering {
    b.date
        .cmp(&a.date)
        .then(b.created_at_ms.cmp(&a.created_at_ms))
        .then(a.id.cmp(&b.id))
}

fn filtered<'a>(
    state: &'a JournalState,
    filter: &'a EntryFilter,
) -> impl Iterator<Item = &'a Entry> {
    state.entries.values().filter(move |e| filter.accepts(e))
}

// ── timeline ─────────────────────────────────────────────────────────────────

/// One day's heading and the entries beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DayGroup {
    /// The day.
    pub date: Date,
    /// Its entries, in list order.
    pub entries: Vec<EntryId>,
}

/// Entries grouped by day, newest day first.
///
/// ```text
///   2026-09-23   ├ e7  (written 21:40)
///                └ e6  (written 08:15)
///   2026-09-20   └ e5
/// ```
pub fn timeline(state: &JournalState, filter: &EntryFilter) -> Vec<DayGroup> {
    let mut list: Vec<&Entry> = filtered(state, filter).collect();
    list.sort_by(|a, b| newest_first(a, b));
    let mut out: Vec<DayGroup> = Vec::new();
    for e in list {
        match out.last_mut() {
            Some(g) if g.date == e.date => g.entries.push(e.id.clone()),
            _ => out.push(DayGroup {
                date: e.date,
                entries: vec![e.id.clone()],
            }),
        }
    }
    out
}

// ── on this day ──────────────────────────────────────────────────────────────

/// Entries from one earlier year that fall on today's month and day.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct YearGroup {
    /// How many years before `today` — 1 means "one year ago today".
    pub years_ago: i32,
    /// The actual day the entries are on (differs from today's month/day only for
    /// a 29 February entry recalled on 28 February).
    pub date: Date,
    /// Its entries, in list order.
    pub entries: Vec<EntryId>,
}

/// Does an entry dated `(m, d)` belong on the recall screen for `today`?
///
/// Normally the month and day must match. The exception is **29 February**: in a
/// year without one, a leap-day entry would never be recalled. So on 28 February of
/// a non-leap year, leap-day entries are recalled too.
///
/// ```text
///   entry 2024-02-29   today 2025-02-28 → recalled (2025 has no 29th)
///   entry 2024-02-29   today 2028-02-28 → not yet; 2028-02-29 exists
///   entry 2024-02-29   today 2028-02-29 → recalled
/// ```
fn recalled_on(entry_md: (u8, u8), today_y: i32, today_md: (u8, u8)) -> bool {
    if entry_md == today_md {
        return true;
    }
    entry_md == (2, 29) && today_md == (2, 28) && days_in_month(today_y, 2) == 28
}

/// Entries written on this month and day in earlier years, most recent year first.
pub fn on_this_day(state: &JournalState, today: Date, filter: &EntryFilter) -> Vec<YearGroup> {
    let (ty, tm, td) = today.to_ymd();
    let mut list: Vec<&Entry> = filtered(state, filter)
        .filter(|e| {
            let (y, m, d) = e.date.to_ymd();
            y < ty && recalled_on((m, d), ty, (tm, td))
        })
        .collect();
    list.sort_by(|a, b| newest_first(a, b));
    let mut out: Vec<YearGroup> = Vec::new();
    for e in list {
        match out.last_mut() {
            Some(g) if g.date == e.date => g.entries.push(e.id.clone()),
            _ => out.push(YearGroup {
                years_ago: ty - e.date.year(),
                date: e.date,
                entries: vec![e.id.clone()],
            }),
        }
    }
    out
}

// ── search ───────────────────────────────────────────────────────────────────

/// Most distinct terms a query is searched for; later terms are ignored.
pub const MAX_QUERY_TERMS: usize = 16;
/// Longest query, in characters, that is read; the rest is ignored.
pub const MAX_QUERY_CHARS: usize = 1024;

/// Longest snippet, in characters, including any ellipses.
pub const SNIPPET_CHARS: usize = 160;
/// Characters of context kept before the first match in a snippet.
const SNIPPET_LEAD: usize = 48;

/// One search result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SearchHit {
    /// The matching entry.
    pub entry: EntryId,
    /// Relevance: higher is better. See [`search`] for how it is computed.
    pub score: u32,
    /// At most [`SNIPPET_CHARS`] characters of the body around the first match
    /// (or its opening, if only the title or tags matched), on one line.
    pub snippet: String,
}

/// Case-insensitive search over title, body, and tags.
///
/// The query is split on whitespace into **terms**, and an entry matches only if
/// **every** term appears somewhere in it — typing more words narrows the results,
/// which is what people expect of a search box. An empty query matches nothing.
///
/// Each term scores by the best place it was found, and an entry's score is the
/// sum over terms:
///
/// | where | points |
/// | --- | --- |
/// | title | 3 |
/// | a tag | 2 |
/// | body | 1 |
///
/// Results are ordered by score, then by the canonical list order.
///
/// ## Bounding the work
///
/// Each term is a scan of every body, so the cost is terms × text. A pasted
/// paragraph as a query would multiply that by hundreds, on a single-threaded
/// WebAssembly host that freezes while it runs. So the query is read up to
/// [`MAX_QUERY_CHARS`], repeated terms are searched once, and at most
/// [`MAX_QUERY_TERMS`] distinct terms are used — in the order typed, so the
/// words a person actually meant come first.
pub fn search(state: &JournalState, query: &str, filter: &EntryFilter) -> Vec<SearchHit> {
    let head: String = query.chars().take(MAX_QUERY_CHARS).collect();
    let folded_query = text::fold(&head);
    let mut terms: Vec<&str> = Vec::new();
    for t in folded_query.split_whitespace() {
        if terms.len() == MAX_QUERY_TERMS {
            break;
        }
        if !terms.contains(&t) {
            terms.push(t);
        }
    }
    if terms.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(&Entry, SearchHit)> = Vec::new();
    'entries: for e in filtered(state, filter) {
        let title = text::fold(&e.title);
        let (body, map) = text::fold_with_map(&e.body);
        let mut score = 0u32;
        let mut first_body: Option<usize> = None;
        for term in &terms {
            let in_body = body.find(term);
            let points = if title.contains(term) {
                3
            } else if e.tags.iter().any(|t| t.key().contains(term)) {
                2
            } else if in_body.is_some() {
                1
            } else {
                continue 'entries;
            };
            score += points;
            if let Some(at) = in_body {
                first_body = Some(first_body.map_or(at, |f| f.min(at)));
            }
        }
        let snippet = snippet(&e.body, first_body.map(|at| map[at]));
        hits.push((
            e,
            SearchHit {
                entry: e.id.clone(),
                score,
                snippet,
            },
        ));
    }
    hits.sort_by(|(ea, a), (eb, b)| b.score.cmp(&a.score).then_with(|| newest_first(ea, eb)));
    hits.into_iter().map(|(_, h)| h).collect()
}

/// Cut a one-line snippet of at most [`SNIPPET_CHARS`] characters from `body`,
/// starting a little before byte offset `at` (always a character boundary), or at
/// the start when `at` is `None`. Elided ends are marked with `…`.
///
/// Whitespace runs (including newlines) are collapsed first, so a snippet reads
/// as one line. Everything is counted in **characters**, so the cut never lands
/// inside a multi-byte character.
fn snippet(body: &str, at: Option<usize>) -> String {
    // Characters before the match, so we can back up by characters, not bytes.
    let chars_before = at.map_or(0, |a| body[..a].chars().count());
    let start_char = chars_before.saturating_sub(SNIPPET_LEAD);
    // Collapse whitespace over a bounded window only: a 1 MiB body must not cost
    // 1 MiB of work per hit. Four times the snippet length leaves room for
    // whitespace runs that collapse away.
    let window: String = body
        .chars()
        .skip(start_char)
        .take(SNIPPET_CHARS * 4)
        .collect();
    let tidy = text::tidy(&window);
    let lead = if start_char > 0 { "…" } else { "" };
    let budget = SNIPPET_CHARS - lead.chars().count();
    let more_after =
        tidy.chars().count() > budget || body.chars().nth(start_char + SNIPPET_CHARS * 4).is_some();
    let mut s = String::from(lead);
    if more_after {
        s.extend(tidy.chars().take(budget - 1));
        s = s.trim_end().to_string();
        s.push('…');
    } else {
        s.push_str(&tidy);
    }
    s
}

// ── tags ─────────────────────────────────────────────────────────────────────

/// A tag and how many entries carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TagCount {
    /// The tag (spelled as on the first entry, in id order, that carries it).
    pub tag: Tag,
    /// Entries carrying it.
    pub count: usize,
}

/// Every tag in use, most used first, then alphabetically by key.
pub fn tag_counts(state: &JournalState, filter: &EntryFilter) -> Vec<TagCount> {
    let mut by_key: BTreeMap<String, TagCount> = BTreeMap::new();
    for e in filtered(state, filter) {
        for t in &e.tags {
            by_key
                .entry(t.key().to_string())
                .or_insert_with(|| TagCount {
                    tag: t.clone(),
                    count: 0,
                })
                .count += 1;
        }
    }
    let mut out: Vec<TagCount> = by_key.into_values().collect();
    // Stable sort over key-ordered input: equal counts stay alphabetical.
    out.sort_by_key(|t| core::cmp::Reverse(t.count));
    out
}

// ── calendar ─────────────────────────────────────────────────────────────────

/// One day of a month that has entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DayActivity {
    /// Day of the month, 1-based.
    pub day: u8,
    /// Entries on that day.
    pub count: usize,
}

/// Days of `year`-`month` with at least one entry, in day order. An invalid month
/// yields an empty list rather than an error: a calendar asking about month 13 has
/// nothing to draw.
pub fn month_activity(
    state: &JournalState,
    year: i32,
    month: u32,
    filter: &EntryFilter,
) -> Vec<DayActivity> {
    let Some(first) = Date::from_ymd(year, month, 1) else {
        return Vec::new();
    };
    // `month` is 1..=12 here, so the narrowing is lossless.
    let last = first.add_days(i32::from(days_in_month(year, month as u8)) - 1);
    let mut counts = [0usize; 32];
    for e in filtered(state, filter) {
        if e.date >= first && e.date <= last {
            counts[usize::from(e.date.to_ymd().2)] += 1;
        }
    }
    (1u8..=31)
        .filter(|&d| counts[usize::from(d)] > 0)
        .map(|d| DayActivity {
            day: d,
            count: counts[usize::from(d)],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{apply, Command};

    fn day(s: &str) -> Date {
        Date::parse_iso(s).unwrap()
    }

    /// Build a state from `(id, journal, date, created_ms, title, body, tags, starred)`.
    #[allow(clippy::type_complexity)]
    fn state(rows: &[(&str, &str, &str, u64, &str, &str, &[&str], bool)]) -> JournalState {
        let mut s = JournalState::new(JournalId::from("p"), "Personal", 0).unwrap();
        apply(
            &mut s,
            Command::CreateJournal {
                id: JournalId::from("w"),
                name: "Work".into(),
            },
            0,
        )
        .unwrap();
        for &(id, j, d, ms, title, body, tags, starred) in rows {
            apply(
                &mut s,
                Command::CreateEntry {
                    id: EntryId::from(id),
                    journal: JournalId::from(j),
                    date: day(d),
                    title: title.into(),
                    body: body.into(),
                },
                ms,
            )
            .unwrap();
            apply(
                &mut s,
                Command::SetTags {
                    id: EntryId::from(id),
                    tags: tags.iter().map(|t| t.to_string()).collect(),
                },
                ms,
            )
            .unwrap();
            apply(
                &mut s,
                Command::SetStarred {
                    id: EntryId::from(id),
                    starred,
                },
                ms,
            )
            .unwrap();
        }
        s
    }

    fn ids(v: &[EntryId]) -> Vec<&str> {
        v.iter().map(EntryId::as_str).collect()
    }

    #[test]
    fn timeline_groups_by_day_newest_first_with_a_total_order() {
        let s = state(&[
            ("a", "p", "2026-09-20", 100, "", "", &[], false),
            ("b", "p", "2026-09-23", 100, "", "", &[], false),
            ("c", "p", "2026-09-23", 300, "", "", &[], false),
            // Same day and instant as `b`: the id breaks the tie.
            ("a2", "p", "2026-09-23", 100, "", "", &[], false),
        ]);
        let t = timeline(&s, &EntryFilter::default());
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].date, day("2026-09-23"));
        assert_eq!(ids(&t[0].entries), ["c", "a2", "b"]);
        assert_eq!(ids(&t[1].entries), ["a"]);
    }

    #[test]
    fn filters_combine_journal_tag_and_star() {
        let s = state(&[
            ("a", "p", "2026-01-01", 1, "", "", &["Travel"], true),
            ("b", "w", "2026-01-02", 1, "", "", &["travel"], true),
            ("c", "w", "2026-01-03", 1, "", "", &["travel"], false),
            ("d", "w", "2026-01-04", 1, "", "", &[], true),
        ]);
        let f = EntryFilter {
            journal: Some(JournalId::from("w")),
            tag: Some(Tag::new("TRAVEL").unwrap()),
            starred_only: true,
        };
        let t = timeline(&s, &f);
        assert_eq!(t.len(), 1);
        assert_eq!(ids(&t[0].entries), ["b"]);
        assert!(timeline(&s, &EntryFilter::default()).len() == 4);
    }

    #[test]
    fn on_this_day_recalls_earlier_years_only() {
        let s = state(&[
            ("y1", "p", "2025-09-23", 1, "", "", &[], false),
            ("y3", "p", "2023-09-23", 1, "", "", &[], false),
            ("y3b", "p", "2023-09-23", 2, "", "", &[], false),
            ("today", "p", "2026-09-23", 1, "", "", &[], false),
            ("other", "p", "2025-09-22", 1, "", "", &[], false),
        ]);
        let r = on_this_day(&s, day("2026-09-23"), &EntryFilter::default());
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].years_ago, ids(&r[0].entries)), (1, vec!["y1"]));
        assert_eq!((r[1].years_ago, ids(&r[1].entries)), (3, vec!["y3b", "y3"]));
    }

    #[test]
    fn a_leap_day_entry_is_recalled_on_the_28th_in_common_years() {
        let s = state(&[("leap", "p", "2024-02-29", 1, "", "", &[], false)]);
        let f = EntryFilter::default();
        let on = |d: &str| on_this_day(&s, day(d), &f).len();
        assert_eq!(
            on("2025-02-28"),
            1,
            "2025 has no 29th, so recall on the 28th"
        );
        assert_eq!(on("2025-03-01"), 0);
        assert_eq!(on("2028-02-28"), 0, "2028 has a 29th; wait for it");
        assert_eq!(on("2028-02-29"), 1);
        let r = on_this_day(&s, day("2025-02-28"), &f);
        assert_eq!(r[0].date, day("2024-02-29"));
    }

    #[test]
    fn search_requires_every_term_and_ranks_title_over_tag_over_body() {
        let s = state(&[
            (
                "body",
                "p",
                "2026-01-03",
                1,
                "Tuesday",
                "we walked to the lighthouse",
                &[],
                false,
            ),
            (
                "tag",
                "p",
                "2026-01-02",
                1,
                "Wednesday",
                "a quiet day",
                &["Lighthouse"],
                false,
            ),
            (
                "title",
                "p",
                "2026-01-01",
                1,
                "The Lighthouse",
                "nothing else",
                &[],
                false,
            ),
            ("none", "p", "2026-01-04", 1, "Thursday", "rain", &[], false),
        ]);
        let f = EntryFilter::default();
        let hits = search(&s, "LIGHTHOUSE", &f);
        let order: Vec<&str> = hits.iter().map(|h| h.entry.as_str()).collect();
        assert_eq!(order, ["title", "tag", "body"]);
        assert_eq!(hits.iter().map(|h| h.score).collect::<Vec<_>>(), [3, 2, 1]);

        // Both terms must match somewhere.
        let both = search(&s, "lighthouse walked", &f);
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].entry.as_str(), "body");
        assert_eq!(both[0].score, 2);

        assert!(search(&s, "   ", &f).is_empty());
        assert!(search(&s, "zebra", &f).is_empty());
    }

    #[test]
    fn repeated_and_excess_terms_are_bounded() {
        let s = state(&[("e", "p", "2026-01-01", 1, "", "rain rain", &[], false)]);
        let f = EntryFilter::default();
        // "rain" typed three times is one term, scored once.
        assert_eq!(search(&s, "rain RAIN rain", &f)[0].score, 1);
        // Terms past the cap are ignored, so a trailing miss does not matter…
        let words: Vec<String> = (0..MAX_QUERY_TERMS).map(|i| format!("w{i}")).collect();
        let s = state(&[("e", "p", "2026-01-01", 1, "", &words.join(" "), &[], false)]);
        let padded = format!("{} zebra", words.join(" "));
        assert_eq!(search(&s, &padded, &f).len(), 1);
        // …but the same miss within the first sixteen distinct terms does.
        assert!(search(&s, "w0 zebra", &f).is_empty());
        // Characters past MAX_QUERY_CHARS are not read.
        let long = format!("{}zebra", " ".repeat(MAX_QUERY_CHARS));
        assert!(search(&s, &long, &f).is_empty());
    }

    #[test]
    fn search_ties_fall_back_to_list_order() {
        let s = state(&[
            ("old", "p", "2026-01-01", 1, "", "rain", &[], false),
            ("new", "p", "2026-02-01", 1, "", "rain", &[], false),
        ]);
        let hits = search(&s, "rain", &EntryFilter::default());
        assert_eq!(hits[0].entry.as_str(), "new");
    }

    #[test]
    fn snippets_are_bounded_single_line_and_char_safe() {
        let long = format!(
            "{}İstanbul\n\nwas   warm {}",
            "é".repeat(300),
            "ü".repeat(300)
        );
        let s = state(&[("e", "p", "2026-01-01", 1, "", &long, &[], false)]);
        // "İ" folds to "i" + a combining dot, so search the part after it.
        let hit = &search(&s, "stanbul", &EntryFilter::default())[0];
        assert!(hit.snippet.chars().count() <= SNIPPET_CHARS);
        assert!(hit.snippet.starts_with('…') && hit.snippet.ends_with('…'));
        assert!(hit.snippet.contains("İstanbul was warm"), "{}", hit.snippet);
        assert!(!hit.snippet.contains('\n'));
    }

    #[test]
    fn a_short_body_snippet_is_the_whole_body() {
        let s = state(&[(
            "e",
            "p",
            "2026-01-01",
            1,
            "Lunch",
            "tacos\nand  tea",
            &[],
            false,
        )]);
        let hit = &search(&s, "lunch", &EntryFilter::default())[0];
        // Title-only match: the snippet is the body's opening, whole.
        assert_eq!(hit.snippet, "tacos and tea");
    }

    #[test]
    fn a_title_only_match_on_a_long_body_shows_its_opening() {
        let body = "word ".repeat(200);
        let s = state(&[("e", "p", "2026-01-01", 1, "Lunch", &body, &[], false)]);
        let hit = &search(&s, "lunch", &EntryFilter::default())[0];
        assert!(hit.snippet.starts_with("word"));
        assert!(hit.snippet.ends_with('…'));
        assert!(hit.snippet.chars().count() <= SNIPPET_CHARS);
    }

    #[test]
    fn a_match_near_the_end_of_a_huge_body_stays_bounded() {
        // More text after the window than the window collects: still elided.
        let body = format!("start {} needle {}", "x ".repeat(10), "y ".repeat(2000));
        let s = state(&[("e", "p", "2026-01-01", 1, "", &body, &[], false)]);
        let hit = &search(&s, "needle", &EntryFilter::default())[0];
        assert!(hit.snippet.ends_with('…'));
        assert!(hit.snippet.chars().count() <= SNIPPET_CHARS);
    }

    #[test]
    fn tag_counts_rank_by_use_then_name() {
        let s = state(&[
            (
                "a",
                "p",
                "2026-01-01",
                1,
                "",
                "",
                &["Food", "travel"],
                false,
            ),
            ("b", "p", "2026-01-02", 1, "", "", &["food", "art"], false),
            ("c", "w", "2026-01-03", 1, "", "", &["zen"], false),
        ]);
        let all = tag_counts(&s, &EntryFilter::default());
        let shown: Vec<(&str, usize)> = all.iter().map(|t| (t.tag.display(), t.count)).collect();
        assert_eq!(shown, [("Food", 2), ("art", 1), ("travel", 1), ("zen", 1)]);

        let work = tag_counts(
            &s,
            &EntryFilter {
                journal: Some(JournalId::from("w")),
                ..EntryFilter::default()
            },
        );
        assert_eq!(work.len(), 1);
    }

    #[test]
    fn month_activity_counts_days_within_the_month_only() {
        let s = state(&[
            ("a", "p", "2024-02-01", 1, "", "", &[], false),
            ("b", "p", "2024-02-29", 1, "", "", &[], false),
            ("c", "p", "2024-02-29", 2, "", "", &[], false),
            ("d", "p", "2024-03-01", 1, "", "", &[], false),
            ("e", "p", "2024-01-31", 1, "", "", &[], false),
        ]);
        let f = EntryFilter::default();
        assert_eq!(
            month_activity(&s, 2024, 2, &f),
            [
                DayActivity { day: 1, count: 1 },
                DayActivity { day: 29, count: 2 }
            ]
        );
        assert!(month_activity(&s, 2024, 13, &f).is_empty());
        assert!(month_activity(&s, 2024, 0, &f).is_empty());
        assert!(month_activity(&s, 2023, 2, &f).is_empty());
        // Out-of-range years are empty, not an arithmetic overflow.
        assert!(month_activity(&s, i32::MAX, 1, &f).is_empty());
        assert!(month_activity(&s, i32::MIN, 1, &f).is_empty());
    }
}
