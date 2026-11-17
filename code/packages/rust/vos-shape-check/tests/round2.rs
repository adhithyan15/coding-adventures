//! Regression tests for security-review ROUND 2 (the structural layer fails
//! closed; the lexer and TOML reader are the repository's grammar-driven
//! crates).  Each case is the reviewer's evading input; each fails (a false
//! PASS or a hang) on the code it replaced.  Checks run in batches (one
//! grammar lexer per test).

mod common;
use common::*;
use std::time::{Duration, Instant};
use vos_shape_check::CrateInput;

fn has(src: &str, id: &str) {
    check(body(src), Expect::Fires(id.into()), src);
}

fn lacks(src: &str, id: &str) {
    check(body(src), Expect::Silent(id.into()), src);
}

fn clean(src: &str) {
    check(body(src), Expect::Clean, src);
}

fn has_msg(src: &str, id: &str, needle: &str) {
    check(body(src), Expect::Msg(id.into(), needle.into()), src);
}

// M1 ----------------------------------------------------------------------

#[test]
fn m1_const_generics_do_not_truncate_a_type_definition() {
    let _b = Batch::begin();
    // the `const` inside the generic list used to end the extent: SH14 skipped
    has(
        "pub struct B<const N: usize> { pub mask: [u8; N] }\n",
        "SH14",
    );
    has(
        "pub union U<const N: usize> { pub a: [u8; N], b: u8 }\n",
        "SH14",
    );
    has("pub struct T<const N: usize>(pub [u8; N]);\n", "SH14");
    // and the interior-mutability closure: a wrapper with const generics
    has(
        "pub struct W<const N: usize> { c: Cell<u8> }\nconst D: W<1> = W { c: Cell::new(1) };\n",
        "SH3",
    );
    has(
        "pub struct W<const N: usize = { 1 + 1 }> { c: Cell<u8> }\nconst D: W = W { c: Cell::new(1) };\n",
        "SH3",
    );
    has(
        "pub struct R<'a, const N: usize> { r: &'a [u8; N] }\n",
        "SH10",
    );
    // const generics in an enum and a type alias are read too
    has(
        "pub enum E<const N: usize> { A([Cell<u8>; N]) }\nconst X: E<1> = E::A([Cell::new(0)]);\n",
        "SH3",
    );
}

// M2 ----------------------------------------------------------------------

#[test]
fn m2_keywords_inside_a_header_do_not_end_it() {
    let _b = Batch::begin();
    // `use<>` (precise capturing) is not a new item
    has("pub fn go(n: u8) -> impl Sized + use<> { go(n) }\n", "SH5");
    // `union` is a contextual keyword and a legal type name
    has(
        "pub struct union;\npub fn go(n: u8) -> union { go(n) }\n",
        "SH5",
    );
    // a return type with a function pointer and an `impl`
    has(
        "pub fn go(n: u8) -> impl Fn(fn(u8) -> u8) { go(n) }\n",
        "SH5",
    );
    // none of these is a gap
    for src in [
        "pub fn go(n: u8) -> impl Sized + use<> { n; }\n",
        "pub struct union;\npub fn go(n: u8) -> union { union }\n",
    ] {
        lacks(src, "VT03");
    }
}

// M3 ----------------------------------------------------------------------

fn bounds(prefix: &str, n: usize) -> String {
    (0..n).map(|i| format!("{prefix}{i}: Copy, ")).collect()
}

#[test]
fn m3_long_impl_headers_are_read_or_fail_closed() {
    let _b = Batch::begin();
    // 300 type parameters (about 900 tokens): over the old 256 cap, under 4096
    let long_generics = format!(
        "pub struct F<T>(T);\nimpl<T, {}> F<T> {{ pub fn g(self) {{ self.g() }} }}\n",
        bounds("A", 300)
    );
    has(&long_generics, "SH5");
    lacks(&long_generics, "VT03");
    // a where clause of 1,500 predicates (about 6,000 tokens would be a gap; 400 is read)
    let long_where = format!(
        "pub struct F<T>(T);\nimpl<T> F<T> where T: Copy, {} T: Copy {{ pub fn g(self) {{ self.g() }} }}\n",
        (0..400).map(|_| "T: Copy, ").collect::<String>()
    );
    has(&long_where, "SH5");
    lacks(&long_where, "VT03");
    // beyond the cap: never skipped silently, always VT03
    let huge_generics = format!(
        "pub struct F<T>(T);\nimpl<T, {}> F<T> {{ pub fn g(self) {{ self.g() }} }}\n",
        bounds("A", 1_500)
    );
    has(&huge_generics, "VT03");
    let huge_where = format!(
        "pub struct F<T>(T);\nimpl<T> F<T> where {} T: Copy {{ pub fn g(self) {{ self.g() }} }}\n",
        (0..2_500).map(|_| "T: Copy, ").collect::<String>()
    );
    has(&huge_where, "VT03");
    let huge_trait = format!(
        "pub trait Tr where {} Self: Sized {{ fn g(); }}\n",
        (0..2_500).map(|_| "Self: Sized, ").collect::<String>()
    );
    has(&huge_trait, "VT03");
    // unterminated headers
    for src in [
        "pub struct S<",
        "pub struct S<T",
        "pub enum E<T>",
        "pub fn f(",
        "impl<T",
    ] {
        has(&format!("{src}\n"), "VT03");
    }
}

#[test]
fn m3_every_scan_that_gives_up_reports_vt03() {
    let _b = Batch::begin();
    // a function header that runs into the next item
    has_msg("fn f fn g() {}\n", "VT03", "could not be read");
    // a type definition with no end
    has("struct A type B = u8;\n", "VT03");
    // a const whose type never ends
    has("const X: Vec<Vec<u8>\n", "VT03");
    // ghost functions too
    let ghost = |inner: &str| {
        let mut i = body(&format!(
            "#[cfg(feature = \"ghost\")]\npub mod ghost {{\n{inner}\n}}\n"
        ));
        i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
        i
    };
    check(
        ghost("pub fn vos_ghost_alpha(s: u8) -> u8 fn"),
        Expect::Fires("VT03".into()),
        "ghost fn header",
    );
}

// M4 ----------------------------------------------------------------------

#[test]
fn m4_comparisons_followed_by_a_shift_are_reported() {
    let _b = Batch::begin();
    for src in [
        "pub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) -> bool { a < b && c < d >> n }",
        "pub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) -> bool { a < b || c < d >> n }",
        "pub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) -> bool { g(a < b, c < d >> n) }\nfn g(_a: bool, _b: bool) -> bool { true }",
        "pub fn g(s: u8) -> bool { 1u32 << s > ::core::u32::MAX }",
        "pub fn g(x: u32, n: u8) -> bool { x << n as u32 > ::core::u32::MAX }",
        "pub fn g(x: u32, n: u32) -> bool { x >> n > ::core::u32::MAX }",
        "pub fn g(a: u32, b: u32, n: u32) -> bool { (a < b) && (a < b >> n) }",
    ] {
        has(&format!("{src}\n"), "SH9");
    }
}

#[test]
fn m4_real_generics_are_not_shifts() {
    let _b = Batch::begin();
    for src in [
        "pub fn f(_a: Option<Option<u8>>) {}",
        "pub struct T { a: Option<Option<u8>> }",
        "pub fn f() -> Option<Option<u8>> { None }",
        "pub fn f<T: Into<Option<Option<u8>>>>(_t: T) {}",
        "pub fn f() { let _x: Option<Option<u8>> = None; }",
        "pub fn f() { let _x = Option::<Option<u8>>::None; }",
        "pub trait Tr<T> {}\npub fn f<T>() where T: Tr<Option<Option<u8>>> {}",
        "pub fn f<'a>(_x: Option<&'a Option<u8>>) {}",
        "pub fn f(_x: Result<Option<u8>, Option<Option<u8>>>) {}",
    ] {
        lacks(&format!("{src}\n"), "SH9");
    }
}

// L1, L2 -------------------------------------------------------------------

#[test]
fn l1_match_arm_rebinding_of_the_loop_variable() {
    let _b = Batch::begin();
    for src in [
        "pub fn f(a: [u8; 4], n: u8) -> u8 { let mut t = 0; for i in 0..4 { t = match n { i => a[i] }; } t }",
        "pub fn f(a: [u8; 4], n: Option<usize>) -> u8 { let mut t = 0; for i in 0..4 { t = match n { Some(i) => a[i], None => 0 }; } t }",
        "pub fn f(a: [u8; 4], n: usize) -> u8 { let mut t = 0; for i in 0..4 { t = match n { 0 | i => a[i], _ => 0 }; } t }",
        "pub fn f(a: [u8; 4], n: usize) -> u8 { let mut t = 0; for i in 0..4 { t = match n { j @ i => a[i], _ => 0 }; } t }",
    ] {
        has(&format!("{src}\n"), "SH8");
    }
    // an index that precedes the match, and arms that do not bind the variable
    lacks(
        "pub fn f(a: [u8; 4], n: u8) -> u8 { let mut t = 0; for i in 0..4 { t = a[i]; t = match n { 0 => 1, _ => 2 }; } t }\n",
        "SH8",
    );
}

#[test]
fn l2_only_array_positions_may_precede_a_bracket() {
    let _b = Batch::begin();
    for src in [
        "pub struct W; impl W { pub async fn f(&self, i: u8) -> u8 { let a = g().await[i]; a } }\nasync fn g() -> [u8; 4] { [0; 4] }",
        "pub fn f(a: [u8; 4]) -> u8 { let n = 1; self::a()[n] }\nfn a() -> [u8; 4] { [0; 4] }",
        "pub fn f(x: u8) -> u8 { crate::t()[x as usize] }",
    ] {
        has(&format!("{src}\n"), "SH8");
    }
    // array literals after the allowed keywords are not indexes
    clean("pub fn f(x: bool) -> [u8; 2] { if x { return [1, 2]; } else { [3, 4] } }\n");
    clean("pub fn f(n: [u8; 2]) -> u8 { match n { [a, _] => a } }\n");
}

// L3 -----------------------------------------------------------------------

/// Asserts that `work` scales (at worst) linearly in its size argument.
///
/// An absolute wall-clock limit depends on the host: a shared CI runner can be
/// several times slower than a laptop and fail a perfectly linear scan.  What a
/// quadratic regression changes is the *shape* of the curve, so measure that.
///
/// Every `check_crate` call pays a fixed cost (building the grammar-driven
/// lexers) that is large next to the size-dependent part and that jitters from
/// run to run.  So do not compare raw times: time `work` at `n`, `k * n` and
/// `k * k * n` and compare the two *increments*, which cancel the fixed cost:
///
/// ```text
///   linear:     (t3 - t2) / (t2 - t1) = k      (cost c * size)
///   quadratic:  (t3 - t2) / (t2 - t1) = k * k  (cost c * size * size)
/// ```
///
/// and require the ratio to stay under 10 for k = 4 (linear 4, quadratic 16) or
/// 3.2 for k = 2 (linear 2, quadratic 4).  Each size keeps the best (minimum) of
/// up to three runs, which discards scheduler noise: one run per size is taken
/// first and more only while the ratio still looks too high, so a quiet host
/// pays for one pass while a noisy one gets three; a real quadratic fails all
/// three.  `n` is chosen so the increments are tens to hundreds of
/// milliseconds; the denominator is floored at 5 ms so a timer-resolution-sized
/// difference cannot inflate the ratio.  The absolute backstop is only a hang
/// guard, far above any real runner's speed.
fn scales_linearly(what: &str, n: usize, k: usize, work: impl Fn(usize)) {
    const BACKSTOP: Duration = Duration::from_secs(120);
    // linear is k, quadratic k * k: k = 4 leaves 2.5x of noise headroom
    // below the bound, k = 2 (only used where the 4096-fn cap forbids more)
    // is tighter
    let limit = if k == 2 { 3.2 } else { 10.0 };
    let sizes = [n, k * n, k * k * n];
    let mut best = [Duration::MAX; 3];
    let mut ratio = f64::MAX;
    for _ in 0..3 {
        for (slot, size) in best.iter_mut().zip(sizes) {
            let t = Instant::now();
            work(size);
            *slot = (*slot).min(t.elapsed());
        }
        let [t1, t2, t3] = best;
        assert!(
            t3 < BACKSTOP,
            "{what}: the largest run took {t3:?} (hang guard {BACKSTOP:?})"
        );
        let step = |hi: Duration, lo: Duration| {
            hi.saturating_sub(lo)
                .max(Duration::from_millis(5))
                .as_secs_f64()
        };
        ratio = step(t3, t2) / step(t2, t1);
        eprintln!(
            "perf {what}: n={n} k={k}: {t1:?} {t2:?} {t3:?}, ratio {ratio:.2} (limit {limit:.2})"
        );
        if ratio < limit {
            break;
        }
    }
    assert!(
        ratio < limit,
        "{what}: growth ratio {ratio:.1} >= {limit:.1}; linear is about {k}, quadratic about {}",
        k * k
    );
}

#[test]
fn l3_many_call_sites_of_one_callee_are_one_edge() {
    let _b = Batch::begin();
    // `types` types x 100 call sites of `self.f()`, all methods named `f`.
    // Without dedupe the per-site clone of the candidate list (and, at 4,000
    // methods, the cap) made this take minutes: quadratic in `types`.
    scales_linearly("call sites of one callee", 12, 4, |types| {
        let mut s = String::from(HEADER);
        let sites = "self.f(); ".repeat(100);
        for n in 0..types {
            s.push_str(&format!(
                "pub struct T{n}; impl T{n} {{ pub fn f(self) {{ {sites} }} }}\n"
            ));
        }
        let r = run(&lib(&s));
        let n = rules(&r).iter().filter(|x| **x == "SH5").count() + r.dropped;
        assert_eq!(n, types, "{}", &r.render()[..200.min(r.render().len())]);
    });
}

// ghost exports ------------------------------------------------------------

fn ghost_with(inner: &str) -> CrateInput {
    let mut i = body(&format!(
        "pub struct Bitmap {{ mask: u8 }}\n#[cfg(feature = \"ghost\")]\npub mod ghost {{\n    use super::*;\n    pub struct Img {{ pub mask: u8 }}\n{inner}\n}}\n"
    ));
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
    i
}

#[test]
fn ghost_exports_are_exact_names_not_prefixes() {
    let _b = Batch::begin();
    for evil in [
        "vos_ghost_alpha_evil",
        "vos_ghost_alpha_",
        "vos_ghost_cfg_evil",
        "vos_ghost_alphaX",
        "vos_ghost_cfg_nn",
    ] {
        check(
            ghost_with(&format!("    pub fn {evil}(s: Img) -> u8 {{ s.mask }}")),
            Expect::Msg("SH17".into(), "no UFCS call".into()),
            evil,
        );
    }
    for ok in ["vos_ghost_alpha", "vos_ghost_alpha_w", "vos_ghost_cfg_n"] {
        check(
            ghost_with(&format!("    pub fn {ok}(s: Img) -> u8 {{ s.mask }}")),
            Expect::Clean,
            ok,
        );
    }
}

// round 3 ------------------------------------------------------------------

#[test]
fn r3_m1_expression_comparisons_are_not_generics_even_with_brace_or_path_follow() {
    let _b = Batch::begin();
    for src in [
        // reviewer's m4: `c < d >> { n }` after a call argument comma
        "pub fn g(a: bool, b: bool) {}\npub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) { g(a < b, c < d >> { n }); }",
        // reviewer's m4b: `<<` then `> ::` with a cast in between
        "pub fn f(n: u32, y: u32) -> bool { let t = n << y as u32 > ::core::u32::MAX; t }",
        // variants: struct literal field, let binding, parenthesised, `::` follow
        "pub struct S { f: bool, g: bool }\npub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) -> S { S { f: a < b, g: c < d >> { n } } }",
        "pub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) -> bool { let t = (a < b, c < d >> (n)); t.0 }",
        "pub fn f(a: u8, b: u8, c: u8, d: u8) -> bool { a < b && c < d >> ::core::u8::MAX }",
        "pub fn f(a: u8, b: u8, c: u8, d: u8, n: u8) -> bool { let _ = [a < b, c < d >> { n }]; true }",
    ] {
        has(&format!("{src}\n"), "SH9");
    }
}

#[test]
fn r3_intended_false_positives_are_exactly_the_documented_ones() {
    let _b = Batch::begin();
    // a REAL generic in tuple-field position or behind `&` is reported: the safe direction (VOS03 section 6)
    for src in [
        "pub struct S(Option<Option<u8>>);",
        "pub fn f(_x: &Option<Option<u8>>) {}",
    ] {
        has(&format!("{src}\n"), "SH9");
    }
    // while the declaration positions stay clean
    for src in [
        "pub fn f(_a: Option<Option<u8>>) {}",
        "pub fn f() -> Option<Option<u8>> { None }",
        "pub fn f<T: Into<Option<Option<u8>>>>(_t: T) {}",
        "pub trait Tr<T> {}\nimpl Tr<Option<Option<u8>>> for u8 {}",
        "pub trait Tr<T> {}\npub struct Z;\nimpl Tr<u8> for Option<Option<u8>> {}",
        "pub fn f() { let _x: Option<Option<u8>> = None; }",
        "pub fn f(_x: Result<Option<u8>, Option<Option<u8>>>) {}",
        "pub fn f() { let _x = Option::<Option<u8>>::None; }",
    ] {
        lacks(&format!("{src}\n"), "SH9");
    }
}

#[test]
fn r3_one_hostile_crate_does_not_fail_the_others() {
    let forged = "__vos_shape_check_file_boundary__";
    let hostile = body(&format!("pub fn f() {{ {forged}; }}\n"));
    let clean = body("pub fn g() {}\n");
    let r = run_all(&[hostile, clean.clone(), clean]);
    assert!(
        r[0].findings.iter().any(|f| f.rule.id() == "VT03"),
        "{}",
        r[0].render()
    );
    assert!(
        r[1].passed() && r[2].passed(),
        "{}\n{}",
        r[1].render(),
        r[2].render()
    );
}
