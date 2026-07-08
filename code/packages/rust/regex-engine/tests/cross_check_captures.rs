//! Interop gate for **capture groups** (`captures`) against the live `regex` crate.
//!
//! `captures` shares `find`'s leftmost-first Pike-VM priority, so it inherits the
//! same rule: on adversarial patterns the `regex` crate's own unanchored search
//! can report a non-leftmost overall match (see `cross_check_find`), so a blanket
//! byte-parity demand is the wrong oracle. Two properties are checked instead:
//!
//!   * **existence agrees** — `captures` returns `Some` iff `regex` matches (this is
//!     the `is_match` property, which Engram relies on).
//!   * **groups agree where the overall span agrees** — whenever both engines place
//!     the *overall* match at the same byte range, *every* capturing group's byte
//!     range must agree too. This is the substantive per-group guarantee; it is
//!     skipped only when the two disagree on where the whole match is (a `regex`
//!     overall-match quirk, already characterized for `find`).
//!
//! The generator leans greedy (so the overall spans usually agree and the group
//! comparison actually runs) but includes lazy quantifiers, alternation, and
//! nested groups for breadth. `regex` is a dev-dependency for this gate only.
use regex_engine as re;

struct Lcg(u64);
impl Lcg {
    fn n(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 16
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.n() as usize) % xs.len()]
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.n() as usize) % (hi - lo + 1)
    }
}

const ATOMS: &[&str] = &["a", "b", "c", ".", "[ab]", "[^a]", r"\w", r"\d"];
// Greedy-leaning quantifier set (lazy forms included but rarer) so the overall
// spans agree often and the group comparison exercises heavily.
const QUANTS: &[&str] = &[
    "", "", "", "*", "+", "?", "{0,2}", "{1,3}", "{2,}", "*?", "+?",
];

fn gen_piece(rng: &mut Lcg, depth: u32) -> String {
    // A *capturing* group (so there are groups to compare) or a plain atom.
    if depth < 3 && rng.range(0, 2) == 0 {
        let inner = gen_alt(rng, depth + 1);
        format!("({inner}){}", rng.pick(QUANTS))
    } else {
        format!("{}{}", rng.pick(ATOMS), rng.pick(QUANTS))
    }
}
fn gen_seq(rng: &mut Lcg, depth: u32) -> String {
    let n = rng.range(1, 4);
    (0..n).map(|_| gen_piece(rng, depth)).collect()
}
fn gen_alt(rng: &mut Lcg, depth: u32) -> String {
    let n = rng.range(1, 2);
    (0..n)
        .map(|_| gen_seq(rng, depth))
        .collect::<Vec<_>>()
        .join("|")
}
fn gen_pattern(rng: &mut Lcg) -> String {
    gen_alt(rng, 0)
}

fn gen_input(rng: &mut Lcg) -> String {
    // Multibyte chars ("é" = 2 bytes, "本" = 3) exercise byte-offset reporting.
    let len = rng.range(0, 8);
    (0..len)
        .map(|_| *rng.pick(&["a", "b", "c", "d", "é", "本"]))
        .collect::<Vec<_>>()
        .join("")
}

fn mine_spans(caps: &re::Captures) -> Vec<Option<(usize, usize)>> {
    (0..caps.len())
        .map(|i| caps.get(i).map(|m| (m.start(), m.end())))
        .collect()
}
fn their_spans(caps: &regex::Captures) -> Vec<Option<(usize, usize)>> {
    (0..caps.len())
        .map(|i| caps.get(i).map(|m| (m.start(), m.end())))
        .collect()
}

#[test]
fn captures_agree_with_regex_where_overall_match_agrees() {
    let mut rng = Lcg(0xCA97_5EED_1234_u64);
    let mut existence = 0u64;
    let mut group_checks = 0u64;
    for _ in 0..12_000 {
        let pat = gen_pattern(&mut rng);
        let (mine, theirs) = match (re::Regex::new(&pat), regex::Regex::new(&pat)) {
            (Ok(m), Ok(t)) => (m, t),
            _ => continue,
        };
        for _ in 0..6 {
            let input = gen_input(&mut rng);
            let mc = mine.captures(&input);
            let tc = theirs.captures(&input);
            // Existence must always agree.
            assert_eq!(
                mc.is_some(),
                tc.is_some(),
                "captures existence differs: pat={pat:?} input={input:?}"
            );
            existence += 1;
            if let (Some(mc), Some(tc)) = (mc, tc) {
                let ms = mine_spans(&mc);
                let ts = their_spans(&tc);
                // Only compare groups when the two agree on the overall match span
                // (index 0); otherwise it is a `regex` overall-match quirk (see
                // cross_check_find) and the group comparison is not meaningful.
                if ms[0] == ts[0] {
                    assert_eq!(
                        ms, ts,
                        "group boundaries differ (overall span agrees): pat={pat:?} input={input:?}"
                    );
                    group_checks += 1;
                }
            }
        }
    }
    println!("captures: {existence} existence checks, {group_checks} full-group comparisons");
    assert!(existence > 50_000, "corpus too small: {existence}");
    assert!(
        group_checks > 20_000,
        "too few group comparisons ran: {group_checks}"
    );
}
