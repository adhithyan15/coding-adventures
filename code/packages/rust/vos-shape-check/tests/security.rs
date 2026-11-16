//! Regression tests for the security review of VOS-D002 (round 1).  Every
//! test here feeds the reviewer's evading input and asserts the SPECIFIC rule
//! id; each failed (a false PASS or a hang) on the code before the fix.  The
//! item numbers are those of the consolidated fix list.

mod common;
use common::*;
use std::time::{Duration, Instant};
use vos_shape_check::{CrateInput, SourceFile};

fn has_msg(input: &CrateInput, id: &str, needle: &str) {
    let r = run(input);
    assert!(
        r.findings
            .iter()
            .any(|f| f.rule.id() == id && f.message.contains(needle)),
        "expected {id} mentioning {needle:?}, got:\n{}",
        r.render()
    );
}

fn has(input: &CrateInput, id: &str) -> bool {
    let r = run(input);
    let ok = rules(&r).contains(&id);
    assert!(ok, "expected {id}, got:\n{}", r.render());
    ok
}

fn src_has(src: &str, id: &str) {
    has(&body(src), id);
}

fn src_lacks(src: &str, id: &str) {
    let r = run(&body(src));
    assert!(
        !rules(&r).contains(&id),
        "did not expect {id}:\n{src}\n{}",
        r.render()
    );
}

fn clean(src: &str) {
    let r = run(&body(src));
    assert!(
        r.findings.is_empty(),
        "expected clean:\n{src}\n{}",
        r.render()
    );
}

fn with_features(src: &str) -> CrateInput {
    let mut i = body(src);
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
    i
}

fn within(limit: Duration, what: &str, f: impl FnOnce()) {
    let t = Instant::now();
    f();
    let d = t.elapsed();
    assert!(d < limit, "{what} took {d:?} (limit {limit:?})");
}

// 1 ---------------------------------------------------------------- shebang

#[test]
fn item1_shebang_lookalike_is_an_inner_attribute_not_a_skipped_line() {
    // rustc reads `#! [attr]` as an inner attribute: the `unsafe` after it is code.
    // (Assert the `unsafe` finding itself, not just any SH2: the root also
    // lacks `forbid`, which would satisfy a bare rule-id check.)
    let unsafe_found = |src: &str| {
        let r = run(&lib(src));
        assert!(
            r.findings
                .iter()
                .any(|f| f.rule.id() == "SH2" && f.message.contains("`unsafe`")),
            "{src:?}\n{}",
            r.render()
        );
    };
    unsafe_found("#! [allow(dead_code)] unsafe fn evil() {}\n");
    unsafe_found("#!/* c */[allow(dead_code)] unsafe fn evil() {}\n");
    unsafe_found("#!\n// c\n[allow(dead_code)] unsafe fn evil() {}\n");
    // a real shebang is still skipped (the line, not the rest of the file)
    let r = run(&lib(
        "#!/usr/bin/env run\n#![no_std]\n#![forbid(unsafe_code)]\npub fn f() {}\n",
    ));
    assert!(r.passed(), "{}", r.render());
    unsafe_found("#!/usr/bin/env run\nunsafe fn evil() {}\n");
}

// 2 ------------------------------------------------------------ raw idents

#[test]
fn item2_raw_identifiers_are_normalised_so_name_rules_still_fire() {
    let table: &[(&str, &str)] = &[
        ("SH8", "pub fn f(o: Option<u8>) -> u8 { o.r#unwrap() }"),
        (
            "SH8",
            "pub fn f(o: Option<u8>) -> u8 { Option::r#unwrap(o) }",
        ),
        ("SH8", "pub fn f(o: Option<u8>) -> u8 { o.r#expect(\"x\") }"),
        ("SH8", "pub fn f() { r#panic!(); }"),
        ("SH11", "pub fn f(a: r#f32) {}"),
        ("SH11", "pub fn f(a: r#f64) {}"),
        ("SH10", "pub fn f(a: r#usize) {}"),
        ("SH10", "pub fn f(a: r#isize) {}"),
        ("SH10", "pub fn f(a: r#Vec) {}"),
        ("SH10", "pub fn f(a: r#Box) {}"),
        ("SH10", "pub fn f(a: r#String) {}"),
        ("SH4", "pub fn f<F: r#Fn(u8)>(g: F) {}"),
        ("SH3", "const C: r#Cell<u8> = Cell::new(0);"),
        ("SH3", "const C: r#AtomicU8 = AtomicU8::new(0);"),
        ("SH3", "r#thread_local!(static X: u8 = 1;);"),
        (
            "SH1",
            "pub fn f() -> u8 { r#std::mem::size_of::<u8>() as u8 }",
        ),
        ("SH1", "use r#alloc::vec::Vec;"),
        ("SH7", "pub fn f(a: u32) -> u32 { a.r#div(2) }"),
        ("SH7", "pub fn f(a: u32) -> u32 { a.r#rem_euclid(2) }"),
        ("SH9", "pub fn f(a: u32) -> u32 { a.r#pow(2) }"),
        ("SH6", "pub fn f(a: [u8; 2]) { a.r#iter(); }"),
        ("SH13", "pub mod r#ghost { }"),
        ("SH13", "pub fn r#alpha(x: u8) -> u8 { x }"),
        ("SH18", "pub fn f() { r#vos_ghost_x(); }"),
        ("SH15", "pub fn f() { r#println!(\"x\"); }"),
        ("SH15", "r#macro_rules! m { () => {} }"),
        ("SH2", "pub fn f() { r#asm!(\"nop\"); }"),
        ("SH18", "#[cfg(r#feature = \"other\")] pub fn f() {}"),
    ];
    for (id, src) in table {
        src_has(&format!("{src}\n"), id);
    }
    // a raw KEYWORD stays an identifier: `r#unsafe` is a name, not `unsafe`
    clean("pub fn r#unsafe(r#fn: u8) -> u8 { r#fn }\n");
}

// 3 ---------------------------------------------- SH17 closed export list

const BITMAP_HEAD: &str =
    "use core::marker::PhantomData;\npub struct Bitmap { mask: u8 }\npub struct BitmapReady;\n";

fn ghost_with(inner: &str) -> CrateInput {
    with_features(&format!(
        "pub trait ReadySet {{ fn insert(s: Bitmap, p: u8) -> (u8, Bitmap); }}\n{BITMAP_HEAD}#[cfg(feature = \"ghost\")]\npub mod ghost {{\n    use super::*;\n    pub struct Image {{ pub mask: u8 }}\n{inner}\n}}\n"
    ))
}

#[test]
fn item3_a_ghost_function_without_a_ufcs_call_is_a_finding() {
    // the operation reimplemented inline: evidence would cover a lookalike
    let inline = "    pub fn vos_ghost_insert(s: Image, p: u8) -> (u8, Image) { (1, Image { mask: s.mask | p }) }";
    has_msg(&ghost_with(inline), "SH17", "no UFCS call");
    // the call made via `Type::f(..)` or `x.f()`, then edited
    let typed = "    pub fn vos_ghost_insert(s: Image, p: u8) -> (u8, Image) { let (o, Bitmap { mask }) = BitmapReady::insert(Bitmap { mask: s.mask }, p); (o, Image { mask }) }";
    has_msg(&ghost_with(typed), "SH17", "no UFCS call");
    let method = "    pub fn vos_ghost_insert(s: Image, p: u8) -> (u8, Image) { let r = Bitmap { mask: s.mask }.insert(p); (r.0, Image { mask: 0 }) }";
    has_msg(&ghost_with(method), "SH17", "no UFCS call");
    // an export name not on the closed list is a wrapper and must have the call
    has_msg(
        &ghost_with("    pub fn vos_ghost_helper(s: Image) -> u8 { s.mask }"),
        "SH17",
        "no UFCS call",
    );
    // the closed list itself is allowed a free body
    for ok in [
        "    pub fn vos_ghost_alpha(s: Image) -> u8 { s.mask }",
        "    pub fn vos_ghost_alpha_w(s: Image) -> u8 { s.mask }",
        "    pub fn vos_ghost_cfg_n() -> u8 { 8 }",
    ] {
        let r = run(&ghost_with(ok));
        assert!(
            !r.findings
                .iter()
                .any(|f| f.message.contains("no UFCS call")),
            "{ok}\n{}",
            r.render()
        );
    }
}

// 4 ------------------------------------------------ crate-level attributes

#[test]
fn item4_root_attributes_must_be_crate_level() {
    // inside a module they apply to the module
    let r = run(&lib(
        "mod m { #![no_std] #![forbid(unsafe_code)] }\npub fn f() {}\n",
    ));
    assert!(
        rules(&r).contains(&"SH1") && rules(&r).contains(&"SH2"),
        "{}",
        r.render()
    );
    // after an item they are no longer crate-level inner attributes
    let r = run(&lib("pub fn f() {}\n#![no_std]\n#![forbid(unsafe_code)]\n"));
    assert!(rules(&r).contains(&"SH1"), "{}", r.render());
    // a leading run of inner attributes (and comments) is fine
    let r = run(&lib(
        "// c\n#![allow(dead_code)]\n#![no_std]\n/* c */ #![forbid(unsafe_code)]\n",
    ));
    assert!(r.passed(), "{}", r.render());
}

// 5 --------------------------------------------------------- crate roots

fn manifest_input(cargo: &str) -> CrateInput {
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(cargo.to_string());
    i
}

#[test]
fn item5_lib_path_bins_and_other_targets_are_pinned() {
    let head = "[package]\nname = \"vos-demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    has(
        &manifest_input(&format!("{head}[lib]\npath = \"src/other.rs\"\n")),
        "SH15",
    );
    has(
        &manifest_input(&format!("{head}[lib]\npath = \"src/x/../lib.rs\"\n")),
        "SH15",
    );
    has(
        &manifest_input(&format!(
            "{head}[[bin]]\nname = \"b\"\npath = \"src/b.rs\"\n"
        )),
        "SH15",
    );
    has(
        &manifest_input(&format!(
            "{head}[[test]]\nname = \"t\"\npath = \"src/t.rs\"\n"
        )),
        "SH15",
    );
    has(
        &manifest_input(&format!(
            "{head}[[example]]\nname = \"e\"\npath = \"src/e.rs\"\n"
        )),
        "SH15",
    );
    has(
        &manifest_input(&format!(
            "{head}[[bench]]\nname = \"b\"\npath = \"src/b.rs\"\n"
        )),
        "SH15",
    );
    // the proper homes are accepted
    let r = run(&manifest_input(&format!("{head}[lib]\npath = \"src/lib.rs\"\n[[test]]\nname = \"t\"\npath = \"tests/t.rs\"\n[[example]]\nname = \"e\"\npath = \"examples/e.rs\"\n[[bench]]\nname = \"b\"\npath = \"benches/b.rs\"\n")));
    assert!(r.passed(), "{}", r.render());
    // `[project]` is the old spelling of `[package]`
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml =
        Some("[project]\nname = \"vos-demo\"\nedition = \"2021\"\nbuild = \"b.rs\"\n".into());
    has(&i, "SH15");
    // binary target FILES are roots the root checks would not cover
    let mut i = body("pub fn f() {}\n");
    i.sources.push(SourceFile {
        path: "src/bin/evil.rs".into(),
        text: "fn main() { unsafe {} }\n".into(),
    });
    has(&i, "SH15");
}

// 6 ------------------------------------------------------- index forms

#[test]
fn item6_every_index_form_is_reported() {
    for src in [
        "pub struct W(pub [u8; 4]);\npub fn f(w: W, i: usize) -> u8 { w.0[i] }",
        "pub fn f(r: Result<[u8; 4], u8>, i: usize) -> Option<u8> { Some(r.ok()?[i]) }",
        "pub fn f(i: usize) -> u8 { { [1u8, 2] }[i] }",
        "pub struct S([u8; 2]);\nimpl core::ops::Index<usize> for S { type Output = u8; fn index(&self, i: usize) -> &u8 { &self.0[i] } }\npub fn f(s: S) -> u8 { s[0] }\nimpl S { fn g(&self) -> u8 { self[1] } }",
        "pub fn f(n: usize) -> u8 { b\"abc\"[n] }",
        "pub fn f(a: [[u8; 2]; 2], i: usize) -> u8 { a[i][i] }",
        "pub fn f(a: [u8; 2]) -> u8 { a[0] }",
    ] {
        src_has(&format!("{src}\n"), "SH8");
    }
    // array literals and types are not indexing
    clean("pub fn f() -> [u8; 2] { let a: [u8; 2] = [1, 2]; let _b = &[3u8, 4]; a }\n");
    clean("pub fn f(x: [u8; 2]) -> [u8; 2] { match x { [a, b] => [b, a] } }\n");
}

// 7 ---------------------------------------------------- SH9, SH8, SH4

#[test]
fn item7_shift_evasions() {
    for src in [
        "pub fn f(x: u32, n: &u32) -> u32 { x >> *n }",
        "pub fn f(x: u32, y: u32) -> u32 { x >> !y }",
        "pub fn f(x: u32, y: i32) -> u32 { x >> -y }",
        "pub fn f(x: u32, n: u32) -> u32 { x >> { n } }",
        "pub fn f(x: u32, n: u32) -> u32 { x << 1 + n }",
        "pub fn f(x: u32, n: u32) -> u32 { x << 1.max(n) }",
        "pub fn f(x: u32, n: u32) -> u32 { x << 8 * n }",
        "pub fn f(x: u32, n: u8) -> u32 { x << n as u32 }",
        "pub fn f(x: u32, n: u8) -> u32 { x >> n as u32 }",
    ] {
        src_has(&format!("{src}\n"), "SH9");
    }
    // literal amounts that are the whole operand, and generics, stay clean
    clean("pub fn f(x: u32) -> u32 { (x >> 3) | (x << 1) | ((x >> 2) & 1) }\n");
    clean("pub fn f(x: u32) -> bool { x >> 1 == 0 || x << 1 != 3 }\n");
    src_lacks(
        "pub fn f(_a: Option<Vec<Vec<u8>>>) {}\npub fn g<T: Into<Vec<Vec<u8>>>>(_t: T) {}\n",
        "SH9",
    );
    // the overflowing methods
    for m in ["pow", "shl", "shr"] {
        src_has(&format!("pub fn f(a: u32) -> u32 {{ a.{m}(2) }}\n"), "SH9");
    }
}

#[test]
fn item7_panic_and_iterator_paths_after_double_colon() {
    src_has(
        "pub fn f(o: Option<u8>) -> u8 { Option::unwrap(o) }\n",
        "SH8",
    );
    src_has(
        "pub fn f(r: Result<u8, u8>) -> u8 { Result::expect(r, \"x\") }\n",
        "SH8",
    );
    src_has(
        "pub fn f(a: [u8; 2]) { Iterator::for_each(a.into_iter(), drop); }\n",
        "SH6",
    );
    src_has(
        "pub fn f(a: [u8; 2]) -> u8 { Iterator::fold(a.into_iter(), 0, |x, y| x) }\n",
        "SH6",
    );
    for m in ["count", "reduce", "find"] {
        src_has(&format!("pub fn f(a: [u8; 2]) {{ a.{m}(); }}\n"), "SH6");
    }
}

#[test]
fn item7_more_closure_positions() {
    for src in [
        "pub fn c() -> u8 { let f = &|x: u8| x; f(1) }",
        "pub fn c() -> u8 { let f = *|x: u8| x; 0 }",
        "pub fn c(a: bool) -> u8 { let f = if a { 1 } else |x: u8| x; 0 }",
        "pub fn c() { for _i in 0..1 { let _g = break |x: u8| x; } }",
        "pub fn c() -> bool { !|x: u8| x == 0 }",
    ] {
        src_has(&format!("{src}\n"), "SH4");
    }
}

// 8 --------------------------------------------------------- SH6 for loops

#[test]
fn item8_a_for_that_is_not_provably_an_impl_header_is_reported() {
    src_has("pub struct S { pub a: u8 }\npub fn f() -> u8 { let mut t = 0; for S { a } in core::iter::repeat(S { a: 1 }) { t = a; } t }\n", "SH6");
    src_has("pub fn f(z: [(u8, u8); 2]) { for (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, zz) in z { } }\n", "SH6");
    // a `for` with no `in` and no impl header
    src_has("pub fn f() { for x { } }\n", "SH6");
    // impl headers (including where clauses and generics) are still not loops
    clean("pub trait T {}\npub struct S;\nimpl<'a> T for S where S: 'a {}\n");
}

// 9 ----------------------------------------------------------- SH5 graph

#[test]
fn item9_turbofish_qualifiers_and_impls_on_unnamed_types() {
    src_has(
        "pub struct Foo<T>(T);\nimpl<T> Foo<T> { pub fn go(n: u8) -> u8 { Foo::<u8>::go(n) } }\n",
        "SH5",
    );
    src_has(
        "pub struct Foo<T>(T);\nimpl<T> Foo<T> { pub fn go(n: u8) -> u8 { <Foo<u8>>::go(n) } }\n",
        "SH5",
    );
    src_has("pub trait X { fn go2(self, n: u8) -> u8; }\nimpl X for [u8; 4] { fn go2(self, n: u8) -> u8 { self.go2(n) } }\n", "SH5");
    src_has("pub trait X { fn go2(self) -> u8; }\nimpl X for (u8, u8) { fn go2(self) -> u8 { self.go2() } }\nimpl X for () { fn go2(self) -> u8 { 0 } }\n", "SH5");
}

// 10 ----------------------------------------------------- cfg predicates

#[test]
fn item10_every_cfg_other_than_the_ghost_gate_is_a_finding() {
    for src in [
        "#[cfg(target_arch = \"riscv32\")] pub fn f() -> u8 { 1 }\n#[cfg(not(target_arch = \"riscv32\"))] pub fn f() -> u8 { 2 }",
        "#[cfg(debug_assertions)] pub fn f() {}",
        "#[cfg_attr(target_arch = \"x86\", derive(Clone))] pub struct S;",
        "pub fn f() -> bool { cfg!(debug_assertions) }",
        "#[cfg(all(test, unix))] pub fn f() {}",
    ] {
        src_has(&format!("{src}\n"), "SH18");
    }
    // the same on every class that binds SH18
    for class in ["C", "S", "M"] {
        let mut i = body("#[cfg(target_arch = \"riscv32\")] pub fn f() {}\n");
        i.vos_toml = Some(format!("class = \"{class}\"\n"));
        has(&i, "SH18");
    }
    // `cfg!` is not a pure core macro either
    src_has("pub fn f() -> bool { cfg!(debug_assertions) }\n", "SH15");
    // the gate and the cfg(test) mask are the only exceptions
    let r = run(&with_features("#[cfg(feature = \"ghost\")]\npub mod ghost { pub fn vos_ghost_alpha() -> u8 { 1 } }\n#[cfg(test)]\nmod t { #[cfg(unix)] fn x() {} }\n"));
    assert!(r.passed(), "{}", r.render());
}

// 11, 12 -------------------------------------------------- performance

#[test]
fn item11_item_scanning_is_linear() {
    within(Duration::from_secs(5), "100k `type a`", || {
        let src = format!("{HEADER}{}", "type a ".repeat(100_000));
        let _ = run(&lib(&src));
    });
    within(Duration::from_secs(5), "100k `struct S<`", || {
        let src = format!("{HEADER}{}", "struct S< ".repeat(50_000));
        let _ = run(&lib(&src));
    });
    within(Duration::from_secs(5), "50k `#[cfg(test)] a`", || {
        let src = format!("{HEADER}{}", "#[cfg(test)] a ".repeat(50_000));
        let _ = run(&lib(&src));
    });
    within(Duration::from_secs(5), "40k nested `#[`", || {
        let src = format!("{HEADER}{}{}", "#[".repeat(40_000), "]".repeat(40_000));
        let _ = run(&lib(&src));
    });
    for (name, unit) in [
        ("const _:", "const _: u8 "),
        ("gated use", "#[cfg(feature = \"ghost\")] pub use ghost::x "),
        ("fn", "fn f "),
        ("impl", "impl X "),
        ("trait", "trait T "),
        ("mod ghost", "mod ghost { "),
    ] {
        within(Duration::from_secs(8), name, || {
            let src = format!("{HEADER}{}", unit.repeat(15_000));
            let _ = run(&lib(&src));
        });
    }
    within(Duration::from_secs(5), "50k `for`", || {
        let src = format!("{HEADER}fn f() {{ {} }}", "for x ".repeat(50_000));
        let _ = run(&lib(&src));
    });
    // wrapper structs chained 3,000 deep resolve without a quadratic sweep
    within(Duration::from_secs(5), "3,000 chained wrappers", || {
        let mut s = String::from(HEADER);
        s.push_str("struct W0 { c: Cell<u8> }\n");
        for n in 1..3000 {
            s.push_str(&format!("struct W{n} {{ w: W{} }}\n", n - 1));
        }
        s.push_str("const D: W2999 = loop_free();\n");
        let r = run(&lib(&s));
        assert!(
            r.findings.iter().any(|f| f.rule.id() == "SH3"),
            "{}",
            r.render()
        );
    });
}

#[test]
fn item12_call_graph_is_linear_and_fails_closed_on_edge_explosions() {
    within(
        Duration::from_secs(5),
        "4,000 colliding method names",
        || {
            let mut s = String::from(HEADER);
            for n in 0..4000 {
                s.push_str(&format!(
                    "pub struct T{n}; impl T{n} {{ pub fn f(self) {{ self.f() }} }}\n"
                ));
            }
            let r = run(&lib(&s));
            assert!(rules(&r).contains(&"VT03"), "edge cap must fail closed");
        },
    );
    // a long single cycle is found in linear time, with one finding per node
    within(Duration::from_secs(5), "3,000-node cycle", || {
        let mut s = String::from(HEADER);
        for n in 0..3000 {
            s.push_str(&format!("fn f{n}() {{ f{}() }}\n", (n + 1) % 3000));
        }
        let r = run(&lib(&s));
        // findings are capped at 1,000 printed; the rest are counted
        let n = rules(&r).iter().filter(|x| **x == "SH5").count() + r.dropped;
        assert_eq!(n, 3000);
    });
    // two cycles and a self-loop are all reported
    let r = run(&body("fn a() { b() }\nfn b() { a() }\nfn c() { d() }\nfn d() { c() }\nfn e() { e() }\nfn ok() { a2() }\nfn a2() {}\n"));
    assert_eq!(
        rules(&r).iter().filter(|x| **x == "SH5").count(),
        5,
        "{}",
        r.render()
    );
}

// 14 ----------------------------------------------- root entry names

#[test]
fn item14_build_scripts_and_cargo_config_by_any_case() {
    for name in ["build.rs", "Build.rs", "BUILD.RS", "bUiLd.rS"] {
        let mut i = body("pub fn f() {}\n");
        i.root_entries.push(name.into());
        let r = run(&i);
        assert!(
            r.findings
                .iter()
                .any(|f| f.rule.id() == "SH15" && f.file == name),
            "{name}\n{}",
            r.render()
        );
    }
    for name in [
        ".cargo",
        "rust-toolchain",
        "rust-toolchain.toml",
        "Rust-Toolchain",
    ] {
        let mut i = body("pub fn f() {}\n");
        i.root_entries.push(name.into());
        let r = run(&i);
        assert!(
            r.findings
                .iter()
                .any(|f| f.rule.id() == "SH15" && f.file == name),
            "{name}\n{}",
            r.render()
        );
    }
}

// 15, 25 ---------------------------------------------------- SH14

#[test]
fn item15_unspecified_kind_in_class_k_is_treated_as_production() {
    src_has("pub struct S { pub a: u8 }\n", "SH14");
    // but a declared spine, contract, hal or profile may have public fields, and so may other classes
    for k in ["spine", "contract", "hal", "profile"] {
        let mut i = body("pub struct S { pub a: u8 }\n");
        i.vos_toml = Some(format!("kind = \"{k}\"\n"));
        assert!(run(&i).passed(), "{k}");
    }
    let mut i = body("pub struct S { pub a: u8 }\n");
    i.vos_toml = Some("class = \"C\"\n".into());
    assert!(run(&i).passed());
}

#[test]
fn item25_tuple_structs_and_unions_are_covered_and_enum_variants_are_not_fields() {
    src_has("pub struct S(pub u8);\n", "SH14");
    src_has("pub struct S(u8, pub [u8; 2]);\n", "SH14");
    src_has("pub union U { pub a: u8, b: u8 }\n", "SH14");
    clean("pub struct S(u8, pub(crate) u8);\n");
    // an enum's variants carry no visibility; `pub enum` itself is allowed
    clean("pub enum E { A(u8), B { x: u8 } }\n");
}

// 16 ------------------------------------------------- TOML key spellings

#[test]
fn item16_every_spelling_of_build_is_found_or_the_parse_fails() {
    let head = "name = \"vos-demo\"\nedition = \"2021\"\n";
    for cargo in [
        format!("[package]\n{head}\"build\" = \"x\"\n"),
        format!("[\"package\"]\n{head}build = \"x\"\n"),
        format!("[package]\n{head}package . build = \"x\"\n").replace("package . build", "build"),
        String::from("package . name = \"vos-demo\"\npackage . edition = \"2021\"\npackage . build = \"x\"\n"),
        String::from("'package'.'name' = \"vos-demo\"\n'package'.'edition' = \"2021\"\n'package'.'build' = 'x'\n"),
        String::from("\"pack\\u0061ge\".name = \"vos-demo\"\n\"pack\\u0061ge\".edition = \"2021\"\n\"pack\\u0061ge\".build = \"x\"\n"),
        String::from("package = { name = \"vos-demo\", edition = \"2021\", \"build\" = \"x\" }\n"),
        format!("[ package ]\n{head}build\t=\t\"x\"\n"),
        format!("[package]\n{head}build = '''\nx'''\n"),
        format!("[project]\n{head}build = false\n"),
    ] {
        let mut i = body("pub fn f() {}\n");
        i.cargo_toml = Some(cargo.clone());
        let r = run(&i);
        assert!(
            rules(&r).contains(&"SH15") || rules(&r).contains(&"VT03"),
            "spelling must be caught:\n{cargo}\n{}",
            r.render()
        );
        assert!(rules(&r).contains(&"SH15"), "and be recognised as SH15:\n{cargo}\n{}", r.render());
    }
}

// 17 ---------------------------------------------------- lexer table

#[test]
fn item17_strings_chars_and_comments_never_hide_the_next_violation() {
    let preludes = [
        "let _c = '\"';",
        "let _c = '\\\\';",
        "let _c = '/';",
        "let _c = '{';",
        "let _c = '}';",
        "let _c = '\\'';",
        "let _s = \"//\";",
        "let _s = \"/*\";",
        "let _s = \"\\\"//\";",
        "let _s = r#\"//\"#;",
        "let _s = r\"/*\";",
        "let _s = b\"//\";",
        "// \" unbalanced\n",
        "// ' unbalanced\n",
        "/* \" ' */",
        "/* // */",
        "/// doc \"\nlet _x = 1;",
        "let _l: &'static str = \"a\";",
    ];
    for p in preludes {
        for (id, v) in [
            ("SH2", "let _u = unsafe { 1 };"),
            ("SH7", "let _d = 4u8 / 2;"),
            ("SH3", "static T: u8 = 0;"),
        ] {
            let src = format!("pub fn f() {{ {p} {v} }}\n");
            let src = if id == "SH3" {
                format!("{p}\n{v}\n")
            } else {
                src
            };
            let r = run(&body(&src));
            assert!(
                rules(&r).contains(&id),
                "{id} after {p:?}\n{src}\n{}",
                r.render()
            );
        }
    }
    // floats that end in a dot or use an underscore before the suffix
    src_has("pub const A: u8 = { let _x = 1.; 0 };\n", "SH11");
    src_has("pub const A: u8 = { let _x = 1_f64; 0 };\n", "SH11");
    src_has("pub const A: u8 = { let _x = 1.0_f32; 0 };\n", "SH11");
    src_has("pub const A: u8 = { let _x = (1.); 0 };\n", "SH11");
    // `1..2` and `1.max(2)` are not floats
    clean("pub fn f() -> u8 { let mut s = 0u8; for _i in 0..2 { s = s.wrapping_add(1.max(2)); } s }\n");
}

// 18 --------------------------------------------- cfg(test) masking

#[test]
fn item18_masking_stops_exactly_at_the_masked_item() {
    for masked in [
        "#[cfg(test)]\nuse core::cmp::min;\n",
        "#[cfg(test)]\nconst X: [u8; 2] = { let a = [1, 2]; a };\n",
        "#[cfg(test)]\nstruct S;\n",
        "#[cfg(test)]\nstruct G<const N: usize = { 3 }> where [u8; N]: Sized { a: [u8; N] }\n",
        "#[cfg(test)]\nfn f() { let _ = { 1 }; }\n",
        "#[cfg(test)]\nimpl X { fn f() { loop { } } }\n",
        "#[cfg(test)]\nmod t { fn f() {} }\n",
        "#[cfg(test)]\ntype T = Vec<u8>;\n",
    ] {
        // the masked item itself may violate anything...
        // ...but what FOLLOWS it is still judged
        for (id, tail) in [
            ("SH2", "pub fn g() { unsafe {} }"),
            ("SH7", "pub fn g(a: u8) -> u8 { a / 2 }"),
            ("SH3", "static X: u8 = 0;"),
        ] {
            let src = format!("{masked}{tail}\n");
            let r = run(&body(&src));
            assert!(
                rules(&r).contains(&id),
                "{id} after masked item\n{src}\n{}",
                r.render()
            );
        }
        // and the masked item alone is not reported
        let r = run(&body(masked));
        assert!(
            !rules(&r).contains(&"SH2") && !rules(&r).contains(&"SH6"),
            "masked item leaked\n{masked}\n{}",
            r.render()
        );
    }
}

// 19 ------------------------------------------- const _ assert exemption

#[test]
fn item19_a_const_underscore_that_defines_code_gets_no_assert_exemption() {
    src_has(
        "const _: () = { struct X; impl X { pub fn f() { assert!(false) } } };\n",
        "SH8",
    );
    src_has("const _: () = { fn f() { assert!(false) } };\n", "SH8");
    src_has("const _: () = { fn f() {} assert!(true) };\n", "SH8");
    // the genuine compile-time check is still allowed
    clean("pub const N: u8 = 4;\nconst _: () = assert!(N == 4);\n");
}

// 20 --------------------------------------------------- dev-dependencies

#[test]
fn item20_dev_dependencies_are_an_allowlist_of_in_repo_path_crates() {
    for extra in [
        "[dev-dependencies]\nserde = \"1\"\n",
        "[dev-dependencies]\nrand = { version = \"0.8\" }\n",
        "[dev-dependencies]\ncoding_adventures_sha256 = \"1\"\n",
        "[dev-dependencies]\ncoding_adventures_sha256 = { path = \"../s\", features = [\"x\"] }\n",
        "[target.'cfg(unix)'.dev-dependencies]\nlibc = \"0.2\"\n",
        "[dev-dependencies.evil]\ngit = \"https://example.com/e\"\n",
    ] {
        let mut i = body("pub fn f() {}\n");
        i.cargo_toml = Some(format!("{MANIFEST}\n{extra}"));
        let r = run(&i);
        assert!(rules(&r).contains(&"SH12"), "{extra}\n{}", r.render());
    }
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(format!("{MANIFEST}\n[dev-dependencies]\ncoding-adventures-sha256 = {{ path = \"../s\" }}\n[target.'cfg(unix)'.dev-dependencies]\ncoding_adventures_test_bootstrap = {{ path = \"../t\" }}\n"));
    assert!(run(&i).passed());
}

// 21, 22, 23 -------------------------------------------- attribute details

#[test]
fn item21_a_path_after_a_keyword_is_rooted_at_the_crate_graph() {
    src_has("use ::std::collections::BTreeMap;\n", "SH1");
    src_has(
        "pub fn f() -> usize { return ::std::mem::size_of::<u8>() as usize; }\n",
        "SH1",
    );
    src_has(
        "pub fn f(a: u8) -> u8 { if a > 0 { ::std::process::exit(1) } else { 0 } }\n",
        "SH1",
    );
    // a later segment of a core path is still fine
    clean("use core::alloc::Layout;\npub fn f(_l: u8) {}\n");
}

#[test]
fn item22_forbid_must_be_exactly_unsafe_code() {
    for attr in [
        "#![forbid(dead_code)]",
        "#![forbid(unsafe_code, dead_code)]",
        "#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]",
        "#![forbid()]",
    ] {
        let r = run(&lib(&format!("#![no_std]\n{attr}\n")));
        assert!(rules(&r).contains(&"SH2"), "{attr}\n{}", r.render());
    }
    let r = run(&lib("#![no_std]\n#![forbid(unsafe_code)]\n"));
    assert!(r.passed());
}

#[test]
fn item23_link_and_link_name_are_sh2_not_sh15() {
    for attr in ["#[link(name = \"c\")]", "#[link_name = \"c\"]"] {
        let r = run(&body(&format!("{attr} pub fn f() {{}}\n")));
        assert!(rules(&r).contains(&"SH2"), "{attr}\n{}", r.render());
        assert!(!rules(&r).contains(&"SH15"), "{attr}\n{}", r.render());
    }
    // and the SH15 attribute bans keep SH15
    src_has("#[no_mangle] pub fn f() {}\n", "SH15");
}

// 24 ------------------------------------------------ loop index shadowing

#[test]
fn item24_a_let_that_rebinds_the_loop_variable_ends_the_exemption() {
    src_has("pub fn f(a: [u8; 4], x: usize) -> u8 { let mut s = 0; for i in 0..4 { let i = x; s = a[i]; } s }\n", "SH8");
    src_has("pub fn f(a: [u8; 4], x: (usize, usize)) -> u8 { let mut s = 0; for i in 0..4 { let (i, _j) = x; s = a[i]; } s }\n", "SH8");
    src_has("pub fn f(a: [u8; 4], x: Option<usize>) -> u8 { let mut s = 0; for i in 0..4 { if let Some(i) = x { s = a[i]; } } s }\n", "SH8");
    // an index BEFORE the rebinding is still by the loop variable; unrelated lets are fine
    src_lacks(
        "pub fn f(a: [u8; 4]) -> u8 { let mut s = 0; for i in 0..4 { s = a[i]; let _k = 1; } s }\n",
        "SH8",
    );
}

// 26 ------------------------------------------------ cap line rule id

#[test]
fn item26_the_dropped_findings_line_carries_rule_vt03() {
    let mut s = String::from(HEADER);
    for n in 0..1100 {
        s.push_str(&format!("pub fn g{n}(x: u8) -> u8 {{ x / 2 }}\n"));
    }
    let text = run(&lib(&s)).render();
    assert!(
        text.lines()
            .any(|l| l.starts_with("VT03 -:0 100 further findings not printed")),
        "{}",
        &text[text.len().saturating_sub(300)..]
    );
}

// headers too long to scan fail closed (the bound that keeps scans linear)

#[test]
fn an_overlong_header_is_vt03_for_functions_and_sh17_for_ghost_wrappers() {
    let long = "+ a ".repeat(1_200);
    let r = run(&body(&format!(
        "pub fn f(a: u8) -> u8 {{ a }}\npub fn g() -> u8 where u8: Sized {long} {{ 0 }}\n"
    )));
    assert!(
        r.findings
            .iter()
            .any(|f| f.rule.id() == "VT03" && f.message.contains("header of `fn g`")),
        "{}",
        r.render()
    );
    let r = run(&ghost_with(&format!(
        "    pub fn vos_ghost_alpha(s: Image) -> u8 where u8: Sized {long} {{ s.mask }}"
    )));
    assert!(
        r.findings
            .iter()
            .any(|f| f.rule.id() == "SH17" && f.message.contains("header too long")),
        "{}",
        r.render()
    );
}

#[test]
fn a_literal_shift_amount_may_be_followed_by_a_cast() {
    clean("pub fn f(x: u32) -> u32 { x << 1 as u32 }\n");
    // a nested block comment between `#!` and `[` is still an attribute
    let r = run(&lib(
        "#!/* a /* b */ c */[allow(dead_code)] unsafe fn evil() {}\n",
    ));
    assert!(
        r.findings.iter().any(|f| f.message.contains("`unsafe`")),
        "{}",
        r.render()
    );
}
