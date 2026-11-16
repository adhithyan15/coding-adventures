//! One clean case and one violating case for every rule of VOS03 section 4.
//!
//! The helpers `clean` and `fires` build a one-file class K crate with the
//! standard `#![no_std] #![forbid(unsafe_code)]` header.  A "clean" test asserts
//! the report is EMPTY (so a rule that over-fires is caught too); a "fires" test
//! asserts the named rule appears.  Removing a rule makes its `fires` test fail.

mod common;
use common::*;
use vos_shape_check::{CrateInput, SourceFile};

fn clean(src: &str) {
    let r = run(&body(src));
    assert!(
        r.findings.is_empty(),
        "expected clean, got:\n{}",
        r.render()
    );
}

fn fires(src: &str, id: &str) {
    let r = run(&body(src));
    assert!(
        rules(&r).contains(&id),
        "expected {id}, got:\n{}",
        r.render()
    );
}

fn silent(src: &str, id: &str) {
    let r = run(&body(src));
    assert!(
        !rules(&r).contains(&id),
        "did not expect {id}, got:\n{}",
        r.render()
    );
}

fn with_manifest(extra: &str, src: &str) -> CrateInput {
    let mut i = body(src);
    i.cargo_toml = Some(format!("{MANIFEST}{extra}"));
    i
}

fn manifest_fires(extra: &str, id: &str) {
    let r = run(&with_manifest(extra, "pub fn f() {}\n"));
    assert!(
        rules(&r).contains(&id),
        "expected {id} for\n{extra}\ngot:\n{}",
        r.render()
    );
}

fn manifest_clean(extra: &str) {
    let r = run(&with_manifest(extra, "pub fn f() {}\n"));
    assert!(
        r.findings.is_empty(),
        "expected clean for\n{extra}\ngot:\n{}",
        r.render()
    );
}

// ---------------------------------------------------------------- SH1

#[test]
fn sh1_clean_core_only() {
    clean("use core::alloc::Layout;\npub fn f(_l: u8) -> u8 { 1 }\n");
}

#[test]
fn sh1_missing_no_std_attribute() {
    let r = run(&lib("#![forbid(unsafe_code)]\npub fn f() {}\n"));
    assert!(rules(&r).contains(&"SH1"), "{}", r.render());
}

#[test]
fn sh1_extern_crate_and_std_alloc_paths() {
    fires("extern crate alloc;\n", "SH1");
    fires("use alloc::vec::Vec;\n", "SH1");
    fires(
        "pub fn f() -> usize { std::mem::size_of::<u8>() as usize }\n",
        "SH1",
    );
    fires("use std;\n", "SH1");
    fires("pub fn f() -> u8 { ::std::primitive::u8::MAX }\n", "SH1");
}

#[test]
fn sh1_cfg_attr_no_std_is_not_accepted() {
    let r = run(&lib(
        "#![cfg_attr(not(test), no_std)]\n#![forbid(unsafe_code)]\n",
    ));
    assert!(rules(&r).contains(&"SH1"));
}

// ---------------------------------------------------------------- SH2

#[test]
fn sh2_clean_unsafe_code_name_raw_ident_comment_string() {
    clean("// unsafe { }\n/* unsafe */\npub fn f(r#unsafe: u8) -> u8 { let _s = \"unsafe\"; r#unsafe }\n");
}

#[test]
fn sh2_unsafe_keyword() {
    fires("pub unsafe fn f() {}\n", "SH2");
    fires("pub fn f() { unsafe { } }\n", "SH2");
}

#[test]
fn sh2_foreign_linkage_and_asm_and_link_attr() {
    fires("extern \"C\" { fn g(); }\n", "SH2");
    fires("pub fn f() { asm!(\"nop\"); }\n", "SH2");
    fires("pub fn f() { core::arch::global_asm!(\"nop\"); }\n", "SH2");
    fires("#[link(name = \"c\")] pub fn f() {}\n", "SH2");
}

#[test]
fn sh2_requires_forbid_and_refuses_waivers() {
    let r = run(&lib("#![no_std]\npub fn f() {}\n"));
    assert!(rules(&r).contains(&"SH2"));
    fires("#[allow(unsafe_code)] pub fn f() {}\n", "SH2");
    fires("#![warn(unsafe_code)]\n", "SH2");
    fires(
        "#[cfg_attr(test, expect(unsafe_code))] pub fn f() {}\n",
        "SH2",
    );
    let r = run(&lib("#![no_std]\n#![deny(unsafe_code)]\n"));
    assert!(rules(&r).contains(&"SH2"), "deny is not forbid");
}

// ---------------------------------------------------------------- SH3

#[test]
fn sh3_clean_consts_and_static_lifetime() {
    clean("pub const A: u8 = 1;\npub const B: [u8; 2] = [1, 2];\n");
    silent("pub const S: &'static str = \"a\";\n", "SH3");
}

#[test]
fn sh3_static_and_thread_local() {
    fires("pub static COUNT: u8 = 0;\n", "SH3");
    fires("static mut X: u8 = 0;\n", "SH3");
    fires("thread_local!(static X: u8 = 1;);\n", "SH3");
}

#[test]
fn sh3_const_of_interior_mutable_type_including_wrappers() {
    fires("const C: Cell<u8> = Cell::new(0);\n", "SH3");
    fires("const C: AtomicU8 = AtomicU8::new(0);\n", "SH3");
    fires(
        "struct W { c: Cell<u8> }\nconst D: W = W { c: Cell::new(1) };\n",
        "SH3",
    );
    fires(
        "type Alias = RefCell<u8>;\nconst D: Alias = RefCell::new(1);\n",
        "SH3",
    );
    // two levels of wrapper: the closure is a fixed point
    fires(
        "struct W { c: Cell<u8> }\nstruct V { w: W }\nconst D: V = V { w: W { c: Cell::new(1) } };\n",
        "SH3",
    );
}

#[test]
fn sh3_const_item_forms_that_are_not_data_items() {
    silent("pub const fn f() -> u8 { 1 }\nconst G: u8 = 1;\n", "SH3");
    silent(
        "pub fn f<const N: usize>(a: [u8; N]) -> u8 { a.len() as u8 }\n",
        "SH3",
    );
}

// ---------------------------------------------------------------- SH4

#[test]
fn sh4_clean_or_patterns_and_logical_or() {
    clean("pub fn f(a: bool, b: bool, x: u8) -> bool { (a || b) | matches!(x, 1 | 2) }\n");
}

#[test]
fn sh4_dyn_fn_bounds_pointers_closures() {
    fires("pub fn f(x: &dyn core::fmt::Debug) {}\n", "SH4");
    fires("pub fn f<F: FnMut(u8)>(g: F) {}\n", "SH4");
    fires("pub fn f<F: Fn(u8)>(g: F) {}\n", "SH4");
    fires("pub fn f<F: FnOnce(u8)>(g: F) {}\n", "SH4");
    fires("pub type P = fn(u8) -> u8;\n", "SH4");
    fires("pub fn f() -> u8 { let c = |a: u8| a; c(1) }\n", "SH4");
    fires("pub fn f() { let c = || (); c(); }\n", "SH4");
    fires("pub fn f() { let c = move || (); c(); }\n", "SH4");
    fires("pub fn f() { let c = move |a: u8| a; }\n", "SH4");
}

#[test]
fn sh4_edition_must_be_2021_or_later() {
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(MANIFEST.replace("2021", "2018"));
    assert!(rules(&run(&i)).contains(&"SH4"));
    i.cargo_toml = Some(MANIFEST.replace("edition = \"2021\"\n", "edition.workspace = true\n"));
    assert!(rules(&run(&i)).contains(&"SH4"));
    i.cargo_toml = Some(MANIFEST.replace("edition = \"2021\"\n", ""));
    assert!(rules(&run(&i)).contains(&"SH4"));
    i.cargo_toml = Some(MANIFEST.replace("2021", "2024"));
    assert!(run(&i).findings.is_empty());
}

// ---------------------------------------------------------------- SH5

#[test]
fn sh5_clean_call_chain_and_cross_crate_ufcs() {
    clean("fn a(x: u8) -> u8 { b(x) }\nfn b(x: u8) -> u8 { x }\n");
    clean(
        "pub struct K;\npub trait T { fn insert(s: u8) -> u8; }\nimpl K { pub fn insert(s: u8) -> u8 { <P as T>::insert(s) } }\n",
    );
}

#[test]
fn sh5_direct_and_mutual_recursion() {
    fires("fn r(n: u8) -> u8 { r(n) }\n", "SH5");
    let r = run(&body("fn a() { b() }\nfn b() { c() }\nfn c() { a() }\n"));
    assert_eq!(
        rules(&r).iter().filter(|x| **x == "SH5").count(),
        3,
        "{}",
        r.render()
    );
    assert!(r.render().contains("a -> b -> c -> a"), "{}", r.render());
}

#[test]
fn sh5_method_and_self_path_and_type_path_recursion() {
    fires("struct S;\nimpl S { fn f(&self) { self.f() } }\n", "SH5");
    fires("struct S;\nimpl S { fn f() { Self::f() } }\n", "SH5");
    fires(
        "struct S;\nstruct U;\nimpl S { fn f() { U::g() } }\nimpl U { fn g() { S::f() } }\n",
        "SH5",
    );
    fires("struct S;\nimpl S { fn f() { <S as Tr>::f() } }\n", "SH5");
    fires("trait Tr { fn f(); fn g() { Tr::f() } }\nstruct S;\nimpl Tr for S { fn f() { Tr::g() } }\n", "SH5");
    fires("mod m { pub fn f() { self::f() } }\n", "SH5");
}

#[test]
fn sh5_turbofish_call_and_nested_fn() {
    fires("fn f() { f::<u8>() }\n", "SH5");
    fires("fn outer() { fn inner() { outer() } inner() }\n", "SH5");
}

// ---------------------------------------------------------------- SH6

#[test]
fn sh6_clean_for_forms() {
    clean(
        "pub const LEN: usize = 8;\npub struct S;\nimpl S { pub const N: u8 = 8; }\npub fn f(mut a: u8) -> u8 {\n    for _ in 0..4 { a = a.wrapping_add(1); }\n    for _i in 0..LEN { a = a.wrapping_add(1); }\n    for _j in 0..S::N { a = a.wrapping_add(1); }\n    for _k in 0u8..8 { a = a.wrapping_add(1); }\n    a\n}\n",
    );
    // `impl Trait for Type` and a higher-ranked bound are not loops
    clean("pub trait T {}\npub struct S;\nimpl T for S {}\n");
    // a higher-ranked bound `for<'a>` is not a loop either
    silent("pub fn f<F>(_g: F) where for<'a> F: Sized {}\n", "SH6");
}

#[test]
fn sh6_while_loop_and_other_for_shapes() {
    fires("pub fn f() { while true { } }\n", "SH6");
    fires("pub fn f() { loop { } }\n", "SH6");
    fires("pub fn f(n: u8) { for _i in 0..n { } }\n", "SH6");
    fires("pub fn f() { for _i in 1..4 { } }\n", "SH6");
    fires("pub fn f() { for _i in 0..=4 { } }\n", "SH6");
    fires("pub fn f() { for _i in 0x0..4 { } }\n", "SH6");
    fires("pub fn f(a: [u8; 2]) { for _x in a { } }\n", "SH6");
    fires(
        "pub fn f(n: u8) { for _i in 0..n.wrapping_add(1) { } }\n",
        "SH6",
    );
    fires("pub fn f() { for _i in 0..Self::n { } }\n", "SH6");
    fires("pub fn f() { for _i in 0..-1 { } }\n", "SH6");
    // a `for` with no `in` before the brace is `impl .. for` and is not judged
    silent("pub trait T {}\npub struct S;\nimpl T for S {}\n", "SH6");
}

#[test]
fn sh6_iterator_adaptors() {
    for m in [
        "iter",
        "map",
        "filter",
        "zip",
        "enumerate",
        "rev",
        "fold",
        "collect",
        "sum",
        "any",
        "all",
        "windows",
        "nth",
    ] {
        fires(&format!("pub fn f(a: [u8; 2]) {{ a.{m}(); }}\n"), "SH6");
    }
    fires("pub fn f(a: [u8; 2]) { a.iter::<u8>(); }\n", "SH6");
    // not a method call: a field or a local named like an adaptor
    silent(
        "pub struct S { pub map: u8 }\npub fn f(s: S) -> u8 { s.map }\n",
        "SH6",
    );
}

// ---------------------------------------------------------------- SH7

#[test]
fn sh7_clean_comments_strings_and_non_division() {
    clean("// a / b % c\n/* a / b */\npub fn f(a: u8) -> u8 { let _s = \"a / b\"; a.wrapping_add(1) }\n");
}

#[test]
fn sh7_division_operators_and_methods() {
    fires("pub fn f(a: u8, b: u8) -> u8 { a / b }\n", "SH7");
    fires("pub fn f(a: u8, b: u8) -> u8 { a % b }\n", "SH7");
    fires("pub fn f(mut a: u8) { a /= 2; }\n", "SH7");
    fires("pub fn f(mut a: u8) { a %= 2; }\n", "SH7");
    for m in [
        "div_euclid",
        "rem_euclid",
        "checked_div",
        "checked_rem",
        "wrapping_div",
        "div_ceil",
        "div",
    ] {
        fires(&format!("pub fn f(a: u8) -> u8 {{ a.{m}(2) }}\n"), "SH7");
    }
}

// ---------------------------------------------------------------- SH8

#[test]
fn sh8_clean_const_assert_get_and_loop_index() {
    clean(
        "pub const N: usize = 4;\nconst _: () = assert!(N == 4);\npub fn f(a: [u8; 4]) -> u8 {\n    let mut s = 0u8;\n    for i in 0..N { s = s.wrapping_add(a[i]); }\n    match a.get(1) { Some(x) => s.wrapping_add(*x), None => s }\n}\n",
    );
    clean("pub fn f() -> [u8; 4] { let a: [u8; 4] = [0; 4]; a }\n");
}

#[test]
fn sh8_panic_methods_and_macros() {
    for m in [
        "unwrap",
        "expect",
        "unwrap_err",
        "expect_err",
        "unwrap_unchecked",
    ] {
        fires(&format!("pub fn f(o: Option<u8>) {{ o.{m}(); }}\n"), "SH8");
    }
    for m in [
        "panic",
        "unreachable",
        "todo",
        "unimplemented",
        "assert",
        "assert_eq",
        "assert_ne",
        "debug_assert",
        "debug_assert_eq",
        "debug_assert_ne",
    ] {
        fires(&format!("pub fn f() {{ {m}!(true); }}\n"), "SH8");
    }
    // only `assert!` in a `const _` item is compile time
    fires("const _: () = panic!(\"x\");\n", "SH8");
}

#[test]
fn sh8_indexing_must_be_by_an_enclosing_for_variable() {
    fires("pub fn f(a: [u8; 4]) -> u8 { a[2] }\n", "SH8");
    fires("pub fn f(a: [u8; 4], i: usize) -> u8 { a[i] }\n", "SH8");
    fires(
        "pub fn f(a: [u8; 4]) -> u8 { let mut s = 0; for i in 0..4 { s = a[i + 1]; } s }\n",
        "SH8",
    );
    fires(
        "pub fn f(a: [[u8; 4]; 4]) -> u8 { for i in 0..4 { return a[i][0]; } 0 }\n",
        "SH8",
    );
    fires(
        "pub fn f(a: [u8; 4]) -> u8 { for i in 0..4 { let _ = a[i]; } a[0] }\n",
        "SH8",
    );
    fires(
        "pub fn f(a: [u8; 4]) -> u8 { g(a)[0] }\nfn g(a: [u8; 4]) -> [u8; 4] { a }\n",
        "SH8",
    );
    // the loop variable of a loop that does not enclose the index
    fires(
        "pub fn f(a: [u8; 4]) -> u8 { let mut s = 0; for i in 0..4 { s += 1; } a[i] }\n",
        "SH8",
    );
}

// ---------------------------------------------------------------- SH9

#[test]
fn sh9_clean_literal_and_wrapped_shifts_and_generics() {
    clean("pub fn f(x: u8) -> u8 { (x >> 3) | (x << 1) }\n");
    clean("pub fn f(x: u8, n: u32) -> Option<u8> { x.checked_shr(n) }\npub fn g(x: u8, n: u32) -> u8 { x.wrapping_shl(n) }\n");
    clean("pub fn f(a: Option<Option<u8>>) -> Option<Option<u8>> { a }\n");
    clean("pub fn f(x: u8) -> u8 { let mut y = x; y <<= 1; y >>= 2; y }\n");
    clean("pub trait Tr<T> {}\npub fn f<T: Tr<Option<u8>>>(_x: T) {}\n");
    clean("pub fn f<T>() where T: Tr<Option<Option<u8>>> {}\npub trait Tr<T> {}\n");
}

#[test]
fn sh9_non_literal_shift_amounts() {
    fires("pub fn f(x: u8, n: u32) -> u8 { x << n }\n", "SH9");
    fires("pub fn f(x: u8, n: u32) -> u8 { x >> n }\n", "SH9");
    fires("pub fn f(x: u8) -> u8 { x >> (1 + 1) }\n", "SH9");
    fires("pub fn f(mut x: u8, n: u32) { x <<= n; }\n", "SH9");
    fires("pub fn f(mut x: u8, n: u32) { x >>= n; }\n", "SH9");
    fires("pub fn f(x: u8) -> u8 { x >> 1.0 as u8 }\n", "SH9");
}

#[test]
fn sh9_bare_arithmetic_is_deliberately_not_checked() {
    // VOS03 section 4 SH9: `+ - *` need clippy::arithmetic_side_effects, not a lexer.
    clean("pub fn f(a: u8, b: u8) -> u8 { a + b - a * b }\n");
}

// ---------------------------------------------------------------- SH10

#[test]
fn sh10_clean_fixed_width() {
    clean("pub struct S { a: u8, b: [u16; 4], c: bool }\npub fn f(s: S) -> (u32, i64, S) { (1u32, 2i64, s) }\n");
    clean("pub const LEN: usize = 4;\npub fn f(i: u8) -> u8 { let _j = i as usize; i }\n");
    clean("pub fn f<const N: usize>(_a: [u8; N]) {}\n");
}

#[test]
fn sh10_heap_and_host_width_types() {
    for ty in [
        "Vec",
        "String",
        "Box",
        "Rc",
        "Arc",
        "HashMap",
        "HashSet",
        "BTreeMap",
        "BTreeSet",
        "VecDeque",
        "BinaryHeap",
    ] {
        fires(&format!("pub fn f(_x: {ty}) {{}}\n"), "SH10");
    }
    fires("pub struct S { pub n: usize }\n", "SH10");
    fires("pub fn f(a: isize) {}\n", "SH10");
    fires("pub fn f(a: u8) -> isize { a as isize }\n", "SH10");
    fires("pub fn f(a: u8) { let _x: usize = a as usize; }\n", "SH10");
}

#[test]
fn sh10_reference_types() {
    fires("pub struct S; impl S { pub fn f(&self) {} }\n", "SH10");
    fires("pub struct S; impl S { pub fn f(&mut self) {} }\n", "SH10");
    fires("pub fn f(a: &u8) {}\n", "SH10");
    fires("pub fn f() -> &'static u8 { &1 }\n", "SH10");
    fires(
        "pub fn f(a: [u8; 2]) -> Option<&'static u8> { None }\n",
        "SH10",
    );
    fires("pub struct S<'a> { pub r: &'a u8 }\n", "SH10");
    fires("pub enum E<'a> { A(&'a u8) }\n", "SH10");
    fires("pub struct T<'a>(&'a u8);\n", "SH10");
    fires("pub fn f(a: u8) { let _r: &'static u8 = &1; }\n", "SH10");
    // `&` as an operator in a body is not a type
    silent("pub fn f(a: u8, b: u8) -> u8 { a & b }\n", "SH10");
}

// ---------------------------------------------------------------- SH11

#[test]
fn sh11_clean_ranges_and_tuple_fields() {
    clean("pub fn f(t: ((u8, u8), u8)) -> u8 { t.0.1 }\n");
    clean("pub fn f() -> u32 { 1_000u32 + 0xffu32 + 0b11u32 }\n");
}

#[test]
fn sh11_float_types_and_literals() {
    fires("pub fn f(a: f32) {}\n", "SH11");
    fires("pub fn f(a: f64) {}\n", "SH11");
    fires("pub const X: u8 = { let _a = 1.0; 1 };\n", "SH11");
    fires("pub const X: u8 = { let _a = 1e3; 1 };\n", "SH11");
    fires("pub const X: u8 = { let _a = 2f32; 1 };\n", "SH11");
    fires("pub const X: u8 = { let _a = 1.5E-3; 1 };\n", "SH11");
}

// ---------------------------------------------------------------- SH12

#[test]
fn sh12_clean_contract_core_and_dev_dependencies() {
    manifest_clean("\n[dependencies]\nvos-ready-contract = { path = \"../vos-ready-contract\" }\nvos_timer_contract = { path = \"../t\" }\ncore = {}\n");
    manifest_clean("\n[dev-dependencies]\ncoding_adventures_sha256 = { path = \"../sha256\" }\ncoding_adventures_test_bootstrap = { path = \"../tb\" }\nvos-ready-contract = { path = \"../c\" }\n");
    manifest_clean("\n[dependencies.vos-ready-contract]\npath = \"../c\"\n");
}

#[test]
fn sh12_third_party_and_malformed_contract_dependencies() {
    manifest_fires("\n[dependencies]\nserde = \"1\"\n", "SH12");
    manifest_fires("\n[dependencies]\nlibc = { version = \"0.2\" }\n", "SH12");
    manifest_fires("\n[build-dependencies]\ncc = \"1\"\n", "SH12");
    manifest_fires(
        "\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n",
        "SH12",
    );
    manifest_fires("\n[dependencies]\nvos-ready-contract = \"1\"\n", "SH12");
    manifest_fires(
        "\n[dependencies]\nvos-ready-contract = { path = \"../c\", package = \"evil\" }\n",
        "SH12",
    );
    manifest_fires(
        "\n[dependencies]\nvos-ready-contract = { path = \"../c\", features = [\"x\"] }\n",
        "SH12",
    );
    manifest_fires(
        "\n[dependencies]\nvos-ready-contract = { git = \"https://example.com/x\" }\n",
        "SH12",
    );
    manifest_fires("\n[dependencies]\ncore = \"1\"\n", "SH12");
    manifest_fires(
        "\n[dependencies]\nvos-contract = { path = \"../c\" }\n",
        "SH12",
    );
    manifest_fires(
        "\n[dependencies]\nVOS-READY-CONTRACT = { path = \"../c\" }\n",
        "SH12",
    );
    manifest_fires("\n[dependencies]\nflag = true\n", "SH12");
}

// ---------------------------------------------------------------- SH13

const GHOSTLESS_FEATURES: &str = "\n[features]\nghost = []\n";

#[test]
fn sh13_clean_gated_ghost_module() {
    let mut i = body(
        "#[cfg(feature = \"ghost\")]\npub mod ghost { pub fn vos_ghost_alpha() -> u8 { 1 } }\n",
    );
    i.cargo_toml = Some(format!("{MANIFEST}{GHOSTLESS_FEATURES}"));
    let r = run(&i);
    assert!(r.findings.is_empty(), "{}", r.render());
}

#[test]
fn sh13_ungated_ghost_module_stray_alpha_default_feature() {
    fires("pub mod ghost { }\n", "SH13");
    fires("#[cfg(test)]\nmod t {}\npub mod ghost;\n", "SH13");
    fires("pub fn alpha(x: u8) -> u8 { x }\n", "SH13");
    fires("pub fn alpha_w(x: u8) -> u8 { x }\n", "SH13");
    fires("pub fn vos_ghost_alpha(x: u8) -> u8 { x }\n", "SH13");
    manifest_fires("\n[features]\nghost = []\ndefault = [\"ghost\"]\n", "SH13");
    manifest_fires(
        "\n[features]\nghost = []\nx = [\"ghost\"]\ndefault = [\"x\"]\n",
        "SH13",
    );
    manifest_clean("\n[features]\nghost = []\nx = []\ndefault = [\"x\"]\n");
}

#[test]
fn sh13_ghost_file_without_a_module_declaration() {
    let mut i = body("pub fn f() {}\n");
    i.sources.push(SourceFile {
        path: "src/ghost.rs".into(),
        text: "pub fn vos_ghost_alpha() {}\n".into(),
    });
    assert!(rules(&run(&i)).contains(&"SH13"));
}

#[test]
fn sh13_gated_out_of_line_ghost_module_with_ghost_file_is_clean() {
    let mut i = body("#[cfg(feature = \"ghost\")]\npub mod ghost;\n");
    i.cargo_toml = Some(format!("{MANIFEST}{GHOSTLESS_FEATURES}"));
    i.sources.push(SourceFile {
        path: "src/ghost.rs".into(),
        text: "pub fn vos_ghost_alpha() -> u8 { 1 }\n".into(),
    });
    let r = run(&i);
    assert!(r.findings.is_empty(), "{}", r.render());
}

// ---------------------------------------------------------------- SH14

fn kind_input(kind: &str, src: &str) -> CrateInput {
    let mut i = body(src);
    i.vos_toml = Some(format!("kind = \"{kind}\"\n"));
    i
}

#[test]
fn sh14_private_fields_clean_pub_fields_fire_for_components() {
    let clean_src = "pub struct S { a: u8, pub(crate) b: u8 }\n";
    for k in ["production", "reference"] {
        assert!(run(&kind_input(k, clean_src)).findings.is_empty());
    }
    for k in ["production", "reference"] {
        let r = run(&kind_input(k, "pub struct S { pub a: u8 }\n"));
        assert!(rules(&r).contains(&"SH14"), "{}", r.render());
    }
    let mut any = kind_input("any", "pub struct S { pub a: u8 }\n");
    any.cargo_toml = Some(MANIFEST.replace("vos-demo", "vos-demo-any"));
    assert!(rules(&run(&any)).contains(&"SH14"));
}

#[test]
fn sh14_spine_contract_and_unknown_kind_may_have_pub_fields() {
    for k in ["spine", "contract", "hal", "profile"] {
        assert!(
            run(&kind_input(k, "pub struct S { pub a: u8 }\n"))
                .findings
                .is_empty(),
            "{k}"
        );
    }
    // an UNSPECIFIED kind in class K is treated as production (fail closed)
    assert!(rules(&run(&body("pub struct S { pub a: u8 }\n"))).contains(&"SH14"));
}

// ---------------------------------------------------------------- SH15

#[test]
fn sh15_clean_core_macros() {
    clean("pub fn f(x: u8) -> bool { matches!(x, 1) }\n");
    // `cfg!` varies the build between the evidence host and the image
    fires("pub fn g() -> bool { cfg!(test) }\n", "SH15");
    fires("pub fn g() -> bool { cfg!(test) }\n", "SH18");
    clean("pub const S: u8 = { let _a = concat!(\"a\", \"b\"); let _b = stringify!(x); let _c = file!(); let _d = column!(); let _e = module_path!(); 1 };\n");
}

#[test]
fn sh15_macros_includes_env_and_foreign_macros() {
    fires("macro_rules! m { () => {} }\n", "SH15");
    fires("pub fn f() { include!(\"x.rs\"); }\n", "SH15");
    fires("pub const S: &str = include_str!(\"x\");\n", "SH15");
    fires("pub const S: &[u8] = include_bytes!(\"x\");\n", "SH15");
    fires("pub fn f() { println!(\"x\"); }\n", "SH15");
    fires("pub const S: &str = env!(\"X\");\n", "SH15");
    fires("pub fn f() { let _v = vec![1]; }\n", "SH15");
    fires("pub fn f() { write!(w, \"x\"); }\n", "SH15");
}

#[test]
fn sh15_attributes_that_relocate_code_or_symbols() {
    fires("#[path = \"x.rs\"] mod x;\n", "SH15");
    fires("#[link_section = \".x\"] pub static X: u8 = 0;\n", "SH15");
    fires("#[no_mangle] pub fn f() {}\n", "SH15");
    fires("#[export_name = \"f\"] pub fn f() {}\n", "SH15");
    fires("#[cfg_attr(test, path = \"x.rs\")] mod x;\n", "SH15");
    fires("#[unsafe(no_mangle)] pub fn f() {}\n", "SH15");
    fires("#[link_name = \"x\"] pub fn f() {}\n", "SH2");
}

#[test]
fn sh15_build_script_file_and_manifest_keys() {
    let mut i = body("pub fn f() {}\n");
    i.root_entries.push("build.rs".into());
    assert!(rules(&run(&i)).contains(&"SH15"));
    manifest_fires("build = \"build.rs\"\n", "SH15"); // appended under [package]
    manifest_fires("build = false\n", "SH15");
    manifest_fires("links = \"z\"\n", "SH15");
    // other spellings of the same key
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(
        "package.name = \"vos-demo\"\npackage.edition = \"2021\"\npackage.build = false\n".into(),
    );
    assert!(rules(&run(&i)).contains(&"SH15"));
    i.cargo_toml =
        Some("package = { name = \"vos-demo\", edition = \"2021\", build = \"b.rs\" }\n".into());
    assert!(rules(&run(&i)).contains(&"SH15"));
    i.cargo_toml = Some(
        "[\"package\"]\nname = \"vos-demo\"\nedition = \"2021\"\n\"build\" = \"b.rs\"\n".into(),
    );
    assert!(rules(&run(&i)).contains(&"SH15"));
}

#[test]
fn sh15_proc_macro_and_target_paths() {
    manifest_fires("\n[lib]\nproc-macro = true\n", "SH15");
    manifest_fires("\n[lib]\nproc_macro = true\n", "SH15");
    manifest_fires("\n[lib]\nproc-macro = \"yes\"\n", "SH15");
    manifest_fires("\n[lib]\ncrate-type = [\"proc-macro\"]\n", "SH15");
    manifest_fires("\n[lib]\ncrate-type = \"proc-macro\"\n", "SH15");
    manifest_clean("\n[lib]\nproc-macro = false\ncrate-type = [\"rlib\"]\npath = \"src/lib.rs\"\n");
    manifest_fires("\n[lib]\npath = \"lib.rs\"\n", "SH15");
    manifest_fires("\n[lib]\npath = \"../x/src/lib.rs\"\n", "SH15");
    manifest_fires("\n[lib]\npath = \"/abs/src/lib.rs\"\n", "SH15");
    manifest_fires("\n[[bin]]\nname = \"b\"\npath = \"tools/b.rs\"\n", "SH15");
    manifest_fires("\n[lib]\npath = 3\n", "SH15");
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(format!(
        "bin = [{{ name = \"b\", path = \"b.rs\" }}]\n{MANIFEST}"
    ));
    assert!(rules(&run(&i)).contains(&"SH15"));
}

// ---------------------------------------------------------------- SH17 and SH18 (toy-based)

fn bitmap() -> CrateInput {
    load("pass-ready-bitmap")
}

#[test]
fn sh17_the_toy_ghost_face_is_clean() {
    let r = run(&bitmap());
    assert!(r.findings.is_empty(), "{}", r.render());
}

#[test]
fn sh17_prefix_on_functions_and_reexports() {
    let r = run(&mutate(
        bitmap(),
        "pub fn vos_ghost_alpha(",
        "pub fn alpha_export(",
    ));
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
    let r = run(&mutate(
        bitmap(),
        "pub use ghost::{vos_ghost_alpha,",
        "pub use ghost::{alpha_gone, vos_ghost_alpha,",
    ));
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
    let r = run(&mutate(
        bitmap(),
        "pub use ghost::{vos_ghost_alpha,",
        "pub use ghost::{vos_ghost_alpha as plain,",
    ));
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
    let r = run(&mutate(bitmap(), "pub use ghost::{vos_ghost_alpha, vos_ghost_cfg_n, vos_ghost_init, vos_ghost_insert, vos_ghost_pick, vos_ghost_remove};", "pub use ghost::*;"));
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
    let r = run(&mutate(
        bitmap(),
        "pub use ghost::{vos_ghost_alpha,",
        "pub use ghost::{vos_ghost_alpha as vos_ghost_a2,",
    ));
    assert!(
        r.findings.is_empty(),
        "an aliased, prefixed re-export is fine:\n{}",
        r.render()
    );
}

#[test]
fn sh17_wrapper_must_have_exactly_one_ufcs_call_and_compute_nothing() {
    let two = "let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);\n        (o, Image { mask })                      // exhaustive";
    let m = mutate(bitmap(), two, "let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);\n        let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::remove(Bitmap { mask }, p);\n        (o, Image { mask })                      // exhaustive");
    assert!(rules(&run(&m)).contains(&"SH17"));
    let branch = mutate(bitmap(), "(o, Image { mask })                      // exhaustive", "if p > 3 { (o, Image { mask }) } else { (o, Image { mask }) }                      // exhaustive");
    assert!(rules(&run(&branch)).contains(&"SH17"));
    let op = mutate(
        bitmap(),
        "(o, Image { mask })                      // exhaustive",
        "(o, Image { mask: mask | 1 })                      // exhaustive",
    );
    assert!(rules(&run(&op)).contains(&"SH17"));
    let extra_call = mutate(
        bitmap(),
        "<BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);",
        "<BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask.wrapping_add(0) }, p);",
    );
    assert!(rules(&run(&extra_call)).contains(&"SH17"));
    let cast = mutate(bitmap(), "(Bitmap { mask: s.mask }, p);\n        (o, Image { mask })\n    }\n    pub fn vos_ghost_pick", "(Bitmap { mask: s.mask }, p);\n        (o, Image { mask: mask as u8 })\n    }\n    pub fn vos_ghost_pick");
    assert!(rules(&run(&cast)).contains(&"SH17"));
    let other_call = mutate(
        bitmap(),
        "(o, Image { mask })                      // exhaustive",
        "(o, helper(Image { mask }))                      // exhaustive",
    );
    assert!(rules(&run(&other_call)).contains(&"SH17"));
    let method = mutate(bitmap(), "pub fn vos_ghost_init() -> Image {\n        let Bitmap { mask } = <BitmapReady as ReadySet>::init();\n        Image { mask }", "pub fn vos_ghost_init() -> Image {\n        let Bitmap { mask } = <BitmapReady as ReadySet>::init();\n        let _z = mask.count_ones();\n        Image { mask }");
    assert!(rules(&run(&method)).contains(&"SH17"));
}

#[test]
fn sh17_destructure_must_be_exhaustive_and_list_every_field() {
    let rest = mutate(
        bitmap(),
        "let Bitmap { mask } = <BitmapReady as ReadySet>::init();",
        "let Bitmap { .. } = <BitmapReady as ReadySet>::init();\n        let mask = 0;",
    );
    assert!(rules(&run(&rest)).contains(&"SH17"));
    // a State with a second field that the destructure forgets
    let grown = mutate(
        mutate(
            bitmap(),
            "pub struct Bitmap { mask: u8 }",
            "pub struct Bitmap { mask: u8, extra: u8 }",
        ),
        "fn init() -> Bitmap { Bitmap { mask: 0 } }",
        "fn init() -> Bitmap { Bitmap { mask: 0, extra: 0 } }",
    );
    let r = run(&grown);
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
    assert!(r.render().contains("missing: extra"), "{}", r.render());
    // an unknown field in the pattern
    let unknown = mutate(
        bitmap(),
        "let Bitmap { mask } = <BitmapReady as ReadySet>::init();",
        "let Bitmap { mask, ghosty } = <BitmapReady as ReadySet>::init();",
    );
    assert!(rules(&run(&unknown)).contains(&"SH17"));
}

#[test]
fn sh17_image_must_mirror_state_in_names_types_and_order() {
    let extra = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub mask: u8, pub more: u8 }",
    );
    assert!(rules(&run(&extra)).contains(&"SH17"));
    let typed = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub mask: u16 }",
    );
    assert!(rules(&run(&typed)).contains(&"SH17"));
    let renamed = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub bits: u8 }",
    );
    assert!(rules(&run(&renamed)).contains(&"SH17"));
    // two fields in a different order
    let two = mutate(
        mutate(
            bitmap(),
            "pub struct Bitmap { mask: u8 }",
            "pub struct Bitmap { mask: u8, other: u8 }",
        ),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub other: u8, pub mask: u8 }",
    );
    let r = run(&two);
    assert!(
        r.findings
            .iter()
            .any(|f| f.rule.id() == "SH17" && f.message.contains("mirror")),
        "{}",
        r.render()
    );
}

#[test]
fn sh17_an_image_no_wrapper_pairs_is_a_finding() {
    let orphan = mutate(
        bitmap(),
        "// The abstraction and the configuration",
        "pub struct Other { pub q: u8 }\n    // The abstraction and the configuration",
    );
    assert!(
        run(&orphan).findings.is_empty(),
        "only a struct named Image is checked for pairing"
    );
    let unpaired = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub mask: u8 }\n    pub struct Spare { pub z: u8 }",
    );
    assert!(run(&unpaired).findings.is_empty());
    // delete every wrapper: Image remains but nothing pairs it
    let mut i = bitmap();
    let f = i
        .sources
        .iter_mut()
        .find(|s| s.path == "src/lib.rs")
        .unwrap();
    let start = f.text.find("    pub fn vos_ghost_init").unwrap();
    let end = f.text.find("    // The abstraction").unwrap();
    f.text.replace_range(start..end, "");
    f.text = f.text.replace("pub use ghost::{vos_ghost_alpha, vos_ghost_cfg_n, vos_ghost_init, vos_ghost_insert, vos_ghost_pick, vos_ghost_remove};", "pub use ghost::{vos_ghost_alpha, vos_ghost_cfg_n};");
    let r = run(&i);
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
}

#[test]
fn sh18_clean_toy_and_every_leak_shape() {
    let leaked = mutate(
        bitmap(),
        "impl ReadySet for BitmapReady {",
        "pub fn leak() -> u8 { ghost::vos_ghost_cfg_n() }\nimpl ReadySet for BitmapReady {",
    );
    assert!(rules(&run(&leaked)).contains(&"SH18"));
    let uses = mutate(
        bitmap(),
        "use vos_ready_contract::{Pick, ReadySet, Upd};",
        "use vos_ready_contract::{Pick, ReadySet, Upd};\nuse ghost::Image;",
    );
    assert!(rules(&run(&uses)).contains(&"SH18"));
    let ungated_reexport = mutate(
        bitmap(),
        "#[cfg(feature = \"ghost\")]\npub use ghost::",
        "pub use ghost::",
    );
    let r = run(&ungated_reexport);
    assert!(rules(&r).contains(&"SH18"), "{}", r.render());
    let private_reexport = mutate(
        bitmap(),
        "pub use ghost::{vos_ghost_alpha,",
        "use ghost::{vos_ghost_alpha,",
    );
    assert!(rules(&run(&private_reexport)).contains(&"SH18"));
    let by_name = mutate(
        bitmap(),
        "impl ReadySet for BitmapReady {",
        "pub fn leak2() { let _ = vos_ghost_alpha; }\nimpl ReadySet for BitmapReady {",
    );
    assert!(rules(&run(&by_name)).contains(&"SH18"));
}

#[test]
fn sh18_cfg_forms_of_evidence_features() {
    for (from, to) in [
        ("#[cfg(feature = \"ghost\")]", "#[cfg(not(feature = \"ghost\"))]"),
        ("#[cfg(feature = \"ghost\")]", "#[cfg(all(feature = \"ghost\", test))]"),
        ("#[cfg(feature = \"ghost\")]", "#[cfg_attr(feature = \"ghost\", derive(Clone))]"),
        ("fn init() -> Bitmap { Bitmap { mask: 0 } }", "fn init() -> Bitmap { if cfg!(feature = \"ghost\") { Bitmap { mask: 1 } } else { Bitmap { mask: 0 } } }"),
        ("pub struct BitmapReady;", "#[cfg(feature = \"ghost\")]\npub struct BitmapReady;"),
        ("pub struct BitmapReady;", "#[cfg(feature = \"cov\")]\npub struct BitmapReady;"),
    ] {
        let r = run(&mutate(bitmap(), from, to));
        assert!(rules(&r).contains(&"SH18"), "{to}\n{}", r.render());
    }
}

#[test]
fn sh18_feature_must_be_declared_and_non_evidence_features_do_not_gate_code() {
    let mut i = bitmap();
    i.cargo_toml = Some(
        MANIFEST.replace("vos-demo", "vos-ready-bitmap")
            + "\n[dependencies]\nvos-ready-contract = { path = \"../c\" }\n",
    );
    let r = run(&i);
    assert!(
        r.findings
            .iter()
            .any(|f| f.message.contains("not declared")),
        "{}",
        r.render()
    );
    let std_gate = mutate(
        bitmap(),
        "pub struct BitmapReady;",
        "#[cfg(feature = \"std\")]\npub struct BitmapReady;",
    );
    let r = run(&std_gate);
    assert!(
        r.findings
            .iter()
            .any(|f| f.message.contains("not an evidence feature")),
        "{}",
        r.render()
    );
    // `cfg(test)` items are masked entirely
    let tested = mutate(
        bitmap(),
        "pub struct BitmapReady;",
        "pub struct BitmapReady;\n#[cfg(test)]\nmod tests { fn t() { assert!(true); [1][0]; } }",
    );
    assert!(run(&tested).findings.is_empty());
}

#[test]
fn sh18_declared_evidence_features_extend_the_central_list() {
    // a feature named in vos.toml is an evidence feature for containment
    let mut i = body("#[cfg(feature = \"trace\")]\npub fn f() {}\n");
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\ntrace = []\n"));
    i.vos_toml = Some("evidence-features = [\"trace\"]\n".into());
    let r = run(&i);
    // `trace` is not in the central list: VT01, and as evidence it may gate only mod ghost
    assert!(rules(&r).contains(&"VT01"), "{}", r.render());
    assert!(rules(&r).contains(&"SH18"), "{}", r.render());
    let central = vos_shape_check::Policy {
        evidence_features: vec!["ghost".into(), "trace".into()],
    };
    let r = vos_shape_check::check_crate(&i, &central);
    assert!(!rules(&r).contains(&"VT01"));
    assert!(rules(&r).contains(&"SH18"), "{}", r.render());
}

// ---------------------------------------------------------------- VT01 to VT03

#[test]
fn vt01_vos_toml_errors_are_findings() {
    for text in [
        "class = \"Z\"\n",
        "kind = \"widget\"\n",
        "colour = \"red\"\n",
        "class = \"K\"\nclass = \"C\"\n",
        "class = K\n",
        "class = \"K\" junk\n",
        "kind = \"any\" junk\n",
        "class \"K\"\n",
        "class = \"K\\n\"\n",
        "class = \"unterminated\n",
        "evidence-features = \"ghost\"\n",
        "evidence-features = [\"ghost\" junk]\n",
        "evidence-features = [\"ghost\"] junk\n",
        "evidence-features = [\"\"]\n",
        "evidence-features = [\"ghost\", \"nope\"]\n",
        "evidence-features = [ghost]\n",
        "class = \"\u{e9}\"\n",
    ] {
        let mut i = body("pub fn f() {}\n");
        i.vos_toml = Some(text.to_string());
        let r = run(&i);
        assert!(rules(&r).contains(&"VT01"), "{text:?}\n{}", r.render());
    }
    // too long a file, too long a line, too many keys, too many names
    let mut i = body("pub fn f() {}\n");
    i.vos_toml = Some("# ".to_string() + &"x".repeat(5000));
    assert!(rules(&run(&i)).contains(&"VT01"));
    i.vos_toml = Some(format!("# {}\n", "x".repeat(1100)));
    assert!(rules(&run(&i)).contains(&"VT01"));
    i.vos_toml = Some((0..17).map(|n| format!("k{n} = \"v\"\n")).collect());
    assert!(run(&i)
        .findings
        .iter()
        .any(|f| f.message.contains("16 keys")));
    i.vos_toml = Some(format!(
        "evidence-features = [{}]\n",
        (0..70)
            .map(|n| format!("\"f{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    assert!(run(&i).findings.iter().any(|f| f.message.contains("64")));
    // a file of many comment lines is fine; many key lines are not, whatever they say
    i.vos_toml = Some("# c\n".repeat(100));
    assert!(run(&i).findings.is_empty());
    i.vos_toml = Some(format!("evidence-features = [\"{}\"]\n", "x".repeat(40)));
    assert!(rules(&run(&i)).contains(&"VT01"));
}

#[test]
fn vt01_valid_vos_toml_is_accepted() {
    let mut i = body("pub fn f() {}\n");
    i.vos_toml = Some("# c\n\nclass = \"K\" # trailing\nkind = \"production\"\nevidence-features = [ \"ghost\" , \"cov\" ] # x\n".into());
    assert!(run(&i).findings.is_empty());
    i.vos_toml = Some("evidence-features = []\n".into());
    assert!(run(&i).findings.is_empty());
    i.vos_toml = Some("evidence-features = [\"ghost\",]\n".into());
    assert!(run(&i).findings.is_empty());
}

#[test]
fn vt02_any_name_and_kind_must_agree() {
    let mk = |name: &str, kind: Option<&str>| {
        let mut i = body("pub fn f() {}\n");
        i.cargo_toml = Some(MANIFEST.replace("vos-demo", name));
        i.vos_toml = kind.map(|k| format!("kind = \"{k}\"\n"));
        run(&i)
    };
    assert!(rules(&mk("vos-ready-any", Some("production"))).contains(&"VT02"));
    assert!(rules(&mk("vos_ready_any", None)).contains(&"VT02"));
    assert!(rules(&mk("vos-ready-bitmap", Some("any"))).contains(&"VT02"));
    assert!(mk("vos-ready-any", Some("any")).findings.is_empty());
    assert!(mk("vos-ready-bitmap", Some("production"))
        .findings
        .is_empty());
    assert!(mk("vos-ready-bitmap", None).findings.is_empty());
}

#[test]
fn vt03_fail_closed_on_unreadable_input() {
    for src in [
        "pub fn f() { \"unterminated }\n",
        "/* unterminated\n",
        "pub fn f() { ( }\n",
        "pub fn f() { ) }\n",
        "pub fn f() { ( ] }\n",
        "pub fn f() { '1 }\n",
        "pub fn f() { r#\"never closed }\n",
        "pub fn f() { r#### }\n",
        "pub fn f() { b'x }\n",
        "pub fn f() { \u{2603} }\n",
        "pub fn f() { '' }\n",
    ] {
        let r = run(&lib(src));
        assert!(rules(&r).contains(&"VT03"), "{src:?}\n{}", r.render());
    }
}

#[test]
fn vt03_manifest_and_crate_root_problems() {
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = None;
    assert!(rules(&run(&i)).contains(&"VT03"));
    i.cargo_toml = Some("[package\nname = 1".into());
    assert!(rules(&run(&i)).contains(&"VT03"));
    i.cargo_toml = Some("x".repeat(70_000));
    assert!(rules(&run(&i)).contains(&"VT03"));
    let mut i = body("pub fn f() {}\n");
    i.sources.clear();
    assert!(rules(&run(&i)).contains(&"VT03"));
}

#[test]
fn vt03_size_caps() {
    let mut i = body("pub fn f() {}\n");
    i.sources[0].text = format!("{}// {}\n", HEADER, "x".repeat(1_100_000));
    assert!(rules(&run(&i)).contains(&"VT03"));
    let mut i = body("pub fn f() {}\n");
    for n in 0..520 {
        i.sources.push(SourceFile {
            path: format!("src/m{n}.rs"),
            text: String::new(),
        });
    }
    assert!(rules(&run(&i)).contains(&"VT03"));
    let mut i = body("pub fn f() {}\n");
    for n in 0..9 {
        i.sources.push(SourceFile {
            path: format!("src/m{n}.rs"),
            text: format!("// {}\n", "x".repeat(1_000_000)),
        });
    }
    assert!(rules(&run(&i)).contains(&"VT03"));
    // more than 4096 functions: the call graph is refused, fail closed
    let mut s = String::from(HEADER);
    for n in 0..4100 {
        s.push_str(&format!("fn f{n}() {{}}\n"));
    }
    assert!(rules(&run(&lib(&s))).contains(&"VT03"));
    // too many tokens in one file
    let mut s = String::from(HEADER);
    s.push_str(&"a ".repeat(262_150));
    assert!(rules(&run(&lib(&s))).contains(&"VT03"));
}

#[test]
fn a_binary_target_file_is_a_finding_and_not_a_crate_root() {
    let mut i = body("fn main() {}\n");
    i.sources[0].path = "src/main.rs".into();
    let r = run(&i);
    assert!(
        rules(&r).contains(&"SH15") && rules(&r).contains(&"VT03"),
        "{}",
        r.render()
    );
}

// ---------------------------------------------------------------- classes

#[test]
fn classes_select_the_rules_of_vos01_section_1_6() {
    let heap = "pub fn f(v: Vec<u8>) -> usize { v.len() }\n"; // SH10, no_std absent in the lib() form
    let third_party = "\n[dependencies]\nserde = \"1\"\n";
    let probe = |class: &str, src: &str, manifest: &str| {
        let mut i = lib(src);
        i.cargo_toml = Some(format!("{MANIFEST}{manifest}"));
        i.vos_toml = Some(format!("class = \"{class}\"\n"));
        run(&i)
    };
    // K binds everything: no_std, forbid, SH10, SH12
    let k = probe("K", heap, third_party);
    for id in ["SH1", "SH2", "SH10", "SH12"] {
        assert!(rules(&k).contains(&id), "K {id}\n{}", k.render());
    }
    // C and S: SH2-SH11 and SH13-SH15, not SH1 or SH12
    for c in ["C", "S"] {
        let r = probe(c, heap, third_party);
        assert!(!rules(&r).contains(&"SH1"), "{c}\n{}", r.render());
        assert!(!rules(&r).contains(&"SH12"));
        assert!(rules(&r).contains(&"SH10") && rules(&r).contains(&"SH2"));
    }
    // M: also exempt from SH10; still SH2, SH6, SH7, SH8, SH11
    let m = probe(
        "M",
        "pub fn f(v: Vec<u8>, x: u8) -> u8 { x / 2 }\n",
        third_party,
    );
    assert!(
        !rules(&m).contains(&"SH10") && !rules(&m).contains(&"SH1") && !rules(&m).contains(&"SH12")
    );
    assert!(rules(&m).contains(&"SH7") && rules(&m).contains(&"SH2"));
    // P and H: no shape rule
    for c in ["P", "H"] {
        let r = probe(
            c,
            "pub fn f(v: Vec<u8>, x: u8) -> u8 { unsafe { let _ = x / 2; } 0 }\n",
            third_party,
        );
        assert!(r.findings.is_empty(), "{c}\n{}", r.render());
    }
    // ...but vos.toml itself and unreadable input are checked for every class
    let mut i = lib("pub fn f() { ( }\n");
    i.vos_toml = Some("class = \"H\"\n".into());
    assert!(rules(&run(&i)).contains(&"VT03"));
}

#[test]
fn reports_are_sorted_deterministic_and_capped() {
    let a = run(&body(
        "pub fn f(x: u8) -> u8 { x / 2 }\npub fn g(x: u8) -> u8 { x % 2 }\n",
    ));
    let b = run(&body(
        "pub fn f(x: u8) -> u8 { x / 2 }\npub fn g(x: u8) -> u8 { x % 2 }\n",
    ));
    assert_eq!(a.render(), b.render());
    assert_eq!(a.exit_code(), 1);
    let text = a.render();
    assert!(
        text.starts_with("vos-shape-check: crate vos-demo, class K (default), kind unspecified\n")
    );
    assert!(text.contains("SH7 src/lib.rs:3 "), "{text}");
    assert!(text.ends_with("result: FAIL, 2 findings\n"));
    let ok = run(&body("pub fn f() {}\n"));
    assert_eq!(ok.exit_code(), 0);
    assert!(ok.render().ends_with("result: PASS, 0 findings\n"));
    // more than 1000 findings: printed 1000, the rest counted
    let mut s = String::from(HEADER);
    for n in 0..1100 {
        s.push_str(&format!("pub fn g{n}(x: u8) -> u8 {{ x / 2 }}\n"));
    }
    let big = run(&lib(&s));
    assert_eq!(big.findings.len(), 1000);
    assert_eq!(big.dropped, 100);
    assert!(!big.passed());
    assert!(big.render().contains("100 further findings not printed"));
    assert!(big.render().ends_with("result: FAIL, 1100 findings\n"));
}
