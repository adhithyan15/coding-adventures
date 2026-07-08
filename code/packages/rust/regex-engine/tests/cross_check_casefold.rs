//! Interop gate for Unicode case folding — compares this engine with
//! `case_insensitive(true)` against the live `regex` crate's default `(?i)`
//! (Unicode) across random patterns and inputs rich in *cased* characters
//! (Latin with accents, Greek σ/ς/Σ, the Kelvin/Ångström signs, long-s ſ,
//! titlecase digraphs). `regex` is a dev-dependency for this gate.
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

// Characters chosen to exercise case folding, including the tricky orbits:
// σ/ς/Σ (Greek final sigma), K/Kelvin-sign, Å/Ångström-sign, s/ſ (long s),
// ǆ/ǅ/Ǆ (titlecase digraph), plus accented Latin.
const CASED: &[&str] = &[
    "a", "A", "e", "E", "z", "Z", "é", "É", "ñ", "Ñ", "σ", "ς", "Σ", "ω", "Ω", "K", "\u{212A}",
    "Å", "\u{212B}", "å", "ſ", "s", "ǆ", "ǅ", "Ǆ", "ı", "I", "ß",
];

fn gen_atom(rng: &mut Lcg) -> String {
    match rng.range(0, 4) {
        0 => (*rng.pick(CASED)).to_string(),
        1 => ".".to_string(),
        2 => format!("[{}{}]", rng.pick(CASED), rng.pick(CASED)),
        _ => (*rng.pick(CASED)).to_string(),
    }
}

fn gen_piece(rng: &mut Lcg, depth: u32) -> String {
    let atom = if depth < 2 && rng.range(0, 4) == 0 {
        let inner = gen_alt(rng, depth + 1);
        format!("(?:{inner})")
    } else {
        gen_atom(rng)
    };
    let quant = *rng.pick(&["", "", "*", "+", "?", "{1,2}"]);
    format!("{atom}{quant}")
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
    let mut p = String::new();
    if rng.range(0, 3) == 0 {
        p.push('^');
    }
    p.push_str(&gen_alt(rng, 0));
    if rng.range(0, 3) == 0 {
        p.push('$');
    }
    p
}

fn gen_input(rng: &mut Lcg) -> String {
    let len = rng.range(0, 7);
    (0..len).map(|_| *rng.pick(CASED)).collect()
}

#[test]
fn case_insensitive_unicode_matches_regex() {
    let mut rng = Lcg(0xF01D_CA5E_u64);
    let mut checked = 0u64;
    let mut skipped = 0u64;
    for _ in 0..80_000 {
        let pat = gen_pattern(&mut rng);
        let mine = match re::RegexBuilder::new(&pat).case_insensitive(true).build() {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        // `regex` default is Unicode; `(?i)` gives Unicode case folding.
        let theirs = match regex::RegexBuilder::new(&pat)
            .case_insensitive(true)
            .build()
        {
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
                "casefold is_match differ: pat={pat:?} input={input:?}"
            );
            checked += 1;
        }
    }
    println!("casefold: cross-checked {checked} pairs; skipped {skipped} patterns");
    assert!(checked > 60_000, "corpus too small: {checked}");
}
