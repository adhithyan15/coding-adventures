//! Interop gate — compares this engine against the real `regex` crate (in
//! `(?-u)` ASCII mode) across randomly-generated valid patterns and inputs.
//! `regex` is a dev-dependency; this test stays as the living correctness gate
//! for the ASCII-mode core.
use regex_engine as re;

// Deterministic LCG so the corpus is reproducible without an rng dependency.
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

fn gen_atom(rng: &mut Lcg) -> String {
    match rng.range(0, 6) {
        0 => (*rng.pick(&["a", "b", "c", "0", "1", "_"])).to_string(),
        1 => ".".to_string(),
        2 => (*rng.pick(&[r"\d", r"\w", r"\s", r"\D", r"\W", r"\S"])).to_string(),
        3 => (*rng.pick(&["[abc]", "[^abc]", "[a-c0-9]", "[0-9_]", "[^ ]"])).to_string(),
        4 => r"\.".to_string(),
        _ => (*rng.pick(&["a", "b", "1", " "])).to_string(),
    }
}

fn gen_piece(rng: &mut Lcg, depth: u32) -> String {
    let atom = if depth < 2 && rng.range(0, 3) == 0 {
        // A nested group.
        let inner = gen_alt(rng, depth + 1);
        if rng.range(0, 2) == 0 {
            format!("({inner})")
        } else {
            format!("(?:{inner})")
        }
    } else {
        gen_atom(rng)
    };
    let quant = match rng.range(0, 6) {
        0 => "*",
        1 => "+",
        2 => "?",
        3 => "{2}",
        4 => "{1,3}",
        _ => "",
    };
    let lazy = if !quant.is_empty() && rng.range(0, 2) == 0 {
        "?"
    } else {
        ""
    };
    format!("{atom}{quant}{lazy}")
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

fn gen_pattern(rng: &mut Lcg) -> String {
    let mut p = String::new();
    if rng.range(0, 3) == 0 {
        p.push('^');
    }
    if rng.range(0, 4) == 0 {
        p.push_str(r"\b");
    }
    p.push_str(&gen_alt(rng, 0));
    if rng.range(0, 4) == 0 {
        p.push_str(r"\b");
    }
    if rng.range(0, 3) == 0 {
        p.push('$');
    }
    p
}

fn gen_input(rng: &mut Lcg) -> String {
    let alpha = ['a', 'b', 'c', '0', '1', '_', ' ', '\n'];
    let len = rng.range(0, 8);
    (0..len).map(|_| *rng.pick(&alpha)).collect()
}

#[test]
fn is_match_matches_regex_across_random_corpus() {
    let mut rng = Lcg(0xD15EA5E);
    let mut checked = 0u64;
    let mut skipped = 0u64;
    for _ in 0..120_000 {
        let pat = gen_pattern(&mut rng);
        let mine = match re::Regex::new(&pat) {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        // Force the real crate into ASCII mode to match this engine's classes.
        let theirs = match regex::Regex::new(&format!("(?-u){pat}")) {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        for _ in 0..6 {
            let input = gen_input(&mut rng);
            assert_eq!(
                mine.is_match(&input),
                theirs.is_match(&input),
                "is_match differ: pat={pat:?} input={input:?}"
            );
            checked += 1;
        }
    }
    println!("cross-checked {checked} (pattern,input) pairs; skipped {skipped} patterns");
    assert!(checked > 100_000, "corpus too small: {checked}");
}

#[test]
fn case_insensitive_matches_regex() {
    let mut rng = Lcg(0xCA5E);
    for _ in 0..20_000 {
        let pat = gen_pattern(&mut rng);
        let mine = match re::RegexBuilder::new(&pat).case_insensitive(true).build() {
            Ok(r) => r,
            Err(_) => continue,
        };
        let theirs = match regex::RegexBuilder::new(&format!("(?-u){pat}"))
            .case_insensitive(true)
            .build()
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        for _ in 0..4 {
            let input = gen_input(&mut rng);
            assert_eq!(
                mine.is_match(&input),
                theirs.is_match(&input),
                "ci is_match differ: pat={pat:?} input={input:?}"
            );
        }
    }
}
