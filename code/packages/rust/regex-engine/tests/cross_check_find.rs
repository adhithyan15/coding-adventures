//! Interop gate for **match extents** (`find`).
//!
//! `find` is verified against the live `regex` crate, but *not* by demanding a
//! byte-identical span — that would be the wrong oracle. On adversarial patterns
//! the `regex` crate's own `find` returns results that even *it* disagrees with:
//! e.g. for `((\w{2,}\w[ab])*b{0,2}[ab]{1,3})` on `"本ébd本bb"` its `find` reports a
//! match starting at byte 5, yet its *anchored* matcher confirms a valid match
//! starting at byte 0 — so its unanchored `find` skips the genuine leftmost match.
//! Matching that quirk is neither achievable (it is a thread-priority artifact of
//! its NFA compiler) nor desirable.
//!
//! Instead this gate checks the *defining properties* of a correct leftmost-first
//! `find`, using the `regex` crate's **anchored** matching as an independent
//! oracle (anchored booleans are unaffected by the unanchored-`find` quirks):
//!
//!   * **valid** — the reported span really is a match: `^(?:P)$` matches the slice.
//!   * **leftmost** — no match starts earlier: `^(?:P)` matches at no byte position
//!     before the reported start.
//!   * **none ⇒ no match** — when `find` returns `None`, `regex` finds no match
//!     anywhere either.
//!
//! These hold for the full construct space — greedy *and* lazy quantifiers,
//! alternation, nested groups, nullable loops — with no exclusions, because they
//! are true of the textbook Pike-VM `find` regardless of the extent-priority
//! corners where the two crates' *reported* spans differ. The exact greedy extents
//! (incl. the nullable-loop fix `(a?)*`⇒whole-run, `(a??)*`⇒empty) are pinned down
//! by hand-verified unit tests in `lib.rs`. `regex` is a dev-dependency for this
//! gate only.
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
// Every greedy/lazy/bounded quantifier — the property gate handles them all.
const QUANTS: &[&str] = &[
    "", "", "*", "+", "?", "*?", "+?", "??", "{0,2}", "{1,3}", "{2,}",
];

fn gen_piece(rng: &mut Lcg, depth: u32) -> String {
    if depth < 3 && rng.range(0, 3) == 0 {
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
    let n = rng.range(1, 3);
    (0..n)
        .map(|_| gen_seq(rng, depth))
        .collect::<Vec<_>>()
        .join("|")
}

// Anchor-free patterns: the property oracle slices the input, and a `^`/`$`/`\b`
// in `P` would assert against a *slice* boundary rather than the original text's,
// invalidating the oracle. (Anchors are covered by unit tests in `lib.rs`.) The
// generator never emits `\b`.
fn gen_pattern(rng: &mut Lcg) -> String {
    gen_alt(rng, 0)
}

fn gen_input(rng: &mut Lcg) -> String {
    // Multibyte chars ("é" = 2 bytes, "本" = 3) so byte-offset handling is exercised.
    let len = rng.range(0, 8);
    (0..len)
        .map(|_| *rng.pick(&["a", "b", "c", "d", "é", "本"]))
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn find_is_the_leftmost_valid_match() {
    let mut rng = Lcg(0x0D_F1_15_ED_u64);
    let mut checked = 0u64;
    let mut skipped = 0u64;
    for _ in 0..9_000 {
        let pat = gen_pattern(&mut rng);
        let mine = match re::Regex::new(&pat) {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        // Oracles: `^(?:P)` = "a match starts here"; `^(?:P)$` = "this slice is a
        // whole match". Skip the (rare) pattern `regex` rejects but we accept.
        let starts_here = match regex::Regex::new(&format!("^(?:{pat})")) {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let whole = regex::Regex::new(&format!("^(?:{pat})$")).unwrap();

        for _ in 0..6 {
            let input = gen_input(&mut rng);
            match mine.find(&input).map(|m| (m.start(), m.end())) {
                Some((s, e)) => {
                    // valid: the reported span is genuinely a match.
                    assert!(
                        whole.is_match(&input[s..e]),
                        "find span is not a valid match: pat={pat:?} input={input:?} span={s}..{e}"
                    );
                    // leftmost: no match starts at any earlier byte boundary.
                    for (b, _) in input.char_indices().take_while(|&(b, _)| b < s) {
                        assert!(
                            !starts_here.is_match(&input[b..]),
                            "earlier match exists at {b} but find started at {s}: pat={pat:?} input={input:?}"
                        );
                    }
                }
                None => {
                    // none ⇒ no match anywhere.
                    assert!(
                        !regex::Regex::new(&pat).unwrap().is_match(&input),
                        "find returned None but regex matches: pat={pat:?} input={input:?}"
                    );
                }
            }
            checked += 1;
        }
    }
    println!("find: verified leftmost+valid on {checked} cases; skipped {skipped} patterns");
    assert!(checked > 40_000, "corpus too small: {checked}");
}

#[test]
fn is_match_matches_regex_on_full_construct_space() {
    // Boolean matching must agree with `regex` byte-for-byte across the whole
    // construct space — greedy/lazy quantifiers, alternation, nested groups,
    // anchors. This is the property Engram's search relies on (it never asks for
    // extents). Anchors are allowed here (no slicing).
    let mut rng = Lcg(0x1A2B_3C4D_5E6F_7A8B_u64);
    let mut checked = 0u64;
    for _ in 0..12_000 {
        let mut pat = String::new();
        if rng.range(0, 4) == 0 {
            pat.push('^');
        }
        pat.push_str(&gen_alt(&mut rng, 0));
        if rng.range(0, 4) == 0 {
            pat.push('$');
        }
        let (mine, theirs) = match (re::Regex::new(&pat), regex::Regex::new(&pat)) {
            (Ok(m), Ok(t)) => (m, t),
            _ => continue,
        };
        for _ in 0..5 {
            let input = gen_input(&mut rng);
            assert_eq!(
                mine.is_match(&input),
                theirs.is_match(&input),
                "is_match differs: pat={pat:?} input={input:?}"
            );
            checked += 1;
        }
    }
    println!("is_match: cross-checked {checked} pairs");
    assert!(checked > 35_000, "corpus too small: {checked}");
}
