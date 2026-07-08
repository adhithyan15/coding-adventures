//! Interop gate for Unicode mode — compares this engine (Unicode by default,
//! like `regex`) against the real `regex` crate with NO `(?-u)` prefix, across
//! random patterns using Unicode classes (`\d\w\s`, `\p{Alphabetic|Mark|Nd}`)
//! and non-ASCII inputs. Case-sensitive only (Unicode case folding is a later
//! addition). `regex` is a dev-dependency for this gate.
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

fn gen_atom(rng: &mut Lcg) -> String {
    match rng.range(0, 7) {
        0 => (*rng.pick(&["a", "e", "n", "Z", "0", "_", "é", "ñ", "日", "ω", "١"])).to_string(),
        1 => ".".to_string(),
        2 => (*rng.pick(&[r"\d", r"\w", r"\s", r"\D", r"\W", r"\S"])).to_string(),
        3 => {
            (*rng.pick(&[r"\p{Alphabetic}", r"\p{Mark}", r"\p{Nd}", r"\P{Alphabetic}"])).to_string()
        }
        4 => (*rng.pick(&["[abcé]", "[^abc]", "[a-z0-9]", r"[\p{Nd}_]", r"[\w-]"])).to_string(),
        5 => r"\.".to_string(),
        _ => (*rng.pick(&["a", "é", "5", " "])).to_string(),
    }
}

fn gen_piece(rng: &mut Lcg, depth: u32) -> String {
    let atom = if depth < 2 && rng.range(0, 3) == 0 {
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
    // ASCII + accented Latin + combining mark + CJK + Greek + Arabic-Indic and
    // Devanagari digits + whitespace variants.
    let alpha = [
        'a', 'b', 'Z', '0', '5', '_', ' ', '\n', 'é', 'ñ', '\u{0301}', '日', '本', 'ω', '١', '५',
        '\u{00A0}', '-',
    ];
    let len = rng.range(0, 9);
    (0..len).map(|_| *rng.pick(&alpha)).collect()
}

#[test]
fn is_match_matches_regex_unicode_mode() {
    let mut rng = Lcg(0x1CEB_00DA_5EED_u64);
    let mut checked = 0u64;
    let mut skipped = 0u64;
    for _ in 0..55_000 {
        let pat = gen_pattern(&mut rng);
        let mine = match re::Regex::new(&pat) {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let theirs = match regex::Regex::new(&pat) {
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
                "unicode is_match differ: pat={pat:?} input={input:?}"
            );
            checked += 1;
        }
    }
    println!("unicode: cross-checked {checked} pairs; skipped {skipped} patterns");
    assert!(checked > 80_000, "corpus too small: {checked}");
}
