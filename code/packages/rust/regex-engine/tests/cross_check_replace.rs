//! Interop gate for **`replace_all`** (and the non-overlapping match iteration it
//! is built on) against the live `regex` crate.
//!
//! `replace_all` drives `captures`, so it inherits the leftmost-first priority and
//! the same rule as `find`/`captures`: where `regex`'s own unanchored search
//! reports a non-leftmost overall match (see `cross_check_find`), a blanket
//! byte-parity demand is the wrong oracle. So the gate compares in two stages:
//!
//!   * **iteration agrees** — the sequence of non-overlapping match byte ranges
//!     from `find_iter` matches `regex`'s. This also validates the empty-match
//!     non-overlap rule (resume at the previous end; skip an empty match sitting
//!     exactly there).
//!   * **replacement agrees where iteration does** — when the match sequences are
//!     identical, `replace_all` with a `$0`/`$1` replacement string must produce
//!     byte-identical output to `regex` (the `$`-expansion is what this proves).
//!
//! `regex` is a dev-dependency for this gate only.
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
// Greedy-leaning (some lazy) so iteration usually agrees and the replacement
// comparison exercises heavily.
const QUANTS: &[&str] = &["", "", "", "*", "+", "?", "{0,2}", "{1,3}", "{2,}", "*?"];

fn gen_piece(rng: &mut Lcg, depth: u32) -> String {
    if depth < 2 && rng.range(0, 2) == 0 {
        let inner = gen_alt(rng, depth + 1);
        format!("({inner}){}", rng.pick(QUANTS))
    } else {
        format!("{}{}", rng.pick(ATOMS), rng.pick(QUANTS))
    }
}
fn gen_seq(rng: &mut Lcg, depth: u32) -> String {
    let n = rng.range(1, 3);
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
    let len = rng.range(0, 8);
    (0..len)
        .map(|_| *rng.pick(&["a", "b", "c", "d", "é", "本"]))
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn replace_all_agrees_with_regex_where_iteration_agrees() {
    let mut rng = Lcg(0x5EED_9E9A_CE00_u64);
    let mut iter_checks = 0u64;
    let mut replace_checks = 0u64;
    // A replacement string exercising the whole match, a numbered group, `$$`, and
    // literal text — identical syntax in both engines.
    const REP: &str = "<$0|$1$$>";
    for _ in 0..14_000 {
        let pat = gen_pattern(&mut rng);
        let (mine, theirs) = match (re::Regex::new(&pat), regex::Regex::new(&pat)) {
            (Ok(m), Ok(t)) => (m, t),
            _ => continue,
        };
        for _ in 0..6 {
            let input = gen_input(&mut rng);
            let mine_spans: Vec<_> = mine
                .find_iter(&input)
                .map(|m| (m.start(), m.end()))
                .collect();
            let their_spans: Vec<_> = theirs
                .find_iter(&input)
                .map(|m| (m.start(), m.end()))
                .collect();
            iter_checks += 1;
            if mine_spans == their_spans {
                assert_eq!(
                    mine.replace_all(&input, REP),
                    theirs.replace_all(&input, REP),
                    "replace_all differs (iteration agrees): pat={pat:?} input={input:?}"
                );
                replace_checks += 1;
            }
        }
    }
    println!("replace: {iter_checks} iteration checks, {replace_checks} replacement comparisons");
    assert!(iter_checks > 50_000, "corpus too small: {iter_checks}");
    assert!(
        replace_checks > 20_000,
        "too few replacement comparisons: {replace_checks}"
    );
}
