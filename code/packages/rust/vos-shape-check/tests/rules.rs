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
    check(body(src), Expect::Clean, src);
}

fn fires(src: &str, id: &str) {
    check(body(src), Expect::Fires(id.into()), src);
}

fn silent(src: &str, id: &str) {
    check(body(src), Expect::Silent(id.into()), src);
}

fn with_manifest(extra: &str, src: &str) -> CrateInput {
    let mut i = body(src);
    i.cargo_toml = Some(format!("{MANIFEST}{extra}"));
    i
}

fn manifest_fires(extra: &str, id: &str) {
    check(
        with_manifest(extra, "pub fn f() {}\n"),
        Expect::Fires(id.into()),
        extra,
    );
}

fn manifest_clean(extra: &str) {
    check(
        with_manifest(extra, "pub fn f() {}\n"),
        Expect::Clean,
        extra,
    );
}

// ---------------------------------------------------------------- SH1

#[test]
fn sh1_clean_core_only() {
    let _batch = Batch::begin();
    clean("use core::alloc::Layout;\npub fn f(_l: u8) -> u8 { 1 }\n");
}

#[test]
fn sh1_missing_no_std_attribute() {
    let _batch = Batch::begin();
    check(
        lib("#![forbid(unsafe_code)]\npub fn f() {}\n"),
        Expect::Fires("SH1".into()),
        "",
    );
}

#[test]
fn sh1_extern_crate_and_std_alloc_paths() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    check(
        lib("#![cfg_attr(not(test), no_std)]\n#![forbid(unsafe_code)]\n"),
        Expect::Fires("SH1".into()),
        "",
    );
}

// ---------------------------------------------------------------- SH2

#[test]
fn sh2_clean_unsafe_code_name_raw_ident_comment_string() {
    let _batch = Batch::begin();
    clean("// unsafe { }\n/* unsafe */\npub fn f(r#unsafe: u8) -> u8 { let _s = \"unsafe\"; r#unsafe }\n");
}

#[test]
fn sh2_unsafe_keyword() {
    let _batch = Batch::begin();
    fires("pub unsafe fn f() {}\n", "SH2");
    fires("pub fn f() { unsafe { } }\n", "SH2");
}

#[test]
fn sh2_foreign_linkage_and_asm_and_link_attr() {
    let _batch = Batch::begin();
    fires("extern \"C\" { fn g(); }\n", "SH2");
    fires("pub fn f() { asm!(\"nop\"); }\n", "SH2");
    fires("pub fn f() { core::arch::global_asm!(\"nop\"); }\n", "SH2");
    fires("#[link(name = \"c\")] pub fn f() {}\n", "SH2");
}

#[test]
fn sh2_requires_forbid_and_refuses_waivers() {
    let _batch = Batch::begin();
    check(
        lib("#![no_std]\npub fn f() {}\n"),
        Expect::Fires("SH2".into()),
        "",
    );
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
    let _batch = Batch::begin();
    clean("pub const A: u8 = 1;\npub const B: [u8; 2] = [1, 2];\n");
    silent("pub const S: &'static str = \"a\";\n", "SH3");
}

#[test]
fn sh3_static_and_thread_local() {
    let _batch = Batch::begin();
    fires("pub static COUNT: u8 = 0;\n", "SH3");
    fires("static mut X: u8 = 0;\n", "SH3");
    fires("thread_local!(static X: u8 = 1;);\n", "SH3");
}

#[test]
fn sh3_const_of_interior_mutable_type_including_wrappers() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    silent("pub const fn f() -> u8 { 1 }\nconst G: u8 = 1;\n", "SH3");
    silent(
        "pub fn f<const N: usize>(a: [u8; N]) -> u8 { a.len() as u8 }\n",
        "SH3",
    );
}

// ---------------------------------------------------------------- SH4

#[test]
fn sh4_clean_or_patterns_and_logical_or() {
    let _batch = Batch::begin();
    clean("pub fn f(a: bool, b: bool, x: u8) -> bool { (a || b) | matches!(x, 1 | 2) }\n");
}

#[test]
fn sh4_dyn_fn_bounds_pointers_closures() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(MANIFEST.replace("2021", "2018"));
    check(i.clone(), Expect::Fires("SH4".into()), "");
    i.cargo_toml = Some(MANIFEST.replace("edition = \"2021\"\n", "edition.workspace = true\n"));
    check(i.clone(), Expect::Fires("SH4".into()), "");
    i.cargo_toml = Some(MANIFEST.replace("edition = \"2021\"\n", ""));
    check(i.clone(), Expect::Fires("SH4".into()), "");
    i.cargo_toml = Some(MANIFEST.replace("2021", "2024"));
    check(i.clone(), Expect::Clean, "");
}

// ---------------------------------------------------------------- SH5

#[test]
fn sh5_clean_call_chain_and_cross_crate_ufcs() {
    let _batch = Batch::begin();
    clean("fn a(x: u8) -> u8 { b(x) }\nfn b(x: u8) -> u8 { x }\n");
    clean(
        "pub struct K;\npub trait T { fn insert(s: u8) -> u8; }\nimpl K { pub fn insert(s: u8) -> u8 { <P as T>::insert(s) } }\n",
    );
}

#[test]
fn sh5_direct_and_mutual_recursion() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    fires("fn f() { f::<u8>() }\n", "SH5");
    fires("fn outer() { fn inner() { outer() } inner() }\n", "SH5");
}

// ---------------------------------------------------------------- SH6

#[test]
fn sh6_clean_for_forms() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    clean("// a / b % c\n/* a / b */\npub fn f(a: u8) -> u8 { let _s = \"a / b\"; a.wrapping_add(1) }\n");
}

#[test]
fn sh7_division_operators_and_methods() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    clean(
        "pub const N: usize = 4;\nconst _: () = assert!(N == 4);\npub fn f(a: [u8; 4]) -> u8 {\n    let mut s = 0u8;\n    for i in 0..N { s = s.wrapping_add(a[i]); }\n    match a.get(1) { Some(x) => s.wrapping_add(*x), None => s }\n}\n",
    );
    clean("pub fn f() -> [u8; 4] { let a: [u8; 4] = [0; 4]; a }\n");
}

#[test]
fn sh8_panic_methods_and_macros() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    clean("pub fn f(x: u8) -> u8 { (x >> 3) | (x << 1) }\n");
    clean("pub fn f(x: u8, n: u32) -> Option<u8> { x.checked_shr(n) }\npub fn g(x: u8, n: u32) -> u8 { x.wrapping_shl(n) }\n");
    clean("pub fn f(a: Option<Option<u8>>) -> Option<Option<u8>> { a }\n");
    clean("pub fn f(x: u8) -> u8 { let mut y = x; y <<= 1; y >>= 2; y }\n");
    clean("pub trait Tr<T> {}\npub fn f<T: Tr<Option<u8>>>(_x: T) {}\n");
    clean("pub fn f<T>() where T: Tr<Option<Option<u8>>> {}\npub trait Tr<T> {}\n");
}

#[test]
fn sh9_non_literal_shift_amounts() {
    let _batch = Batch::begin();
    fires("pub fn f(x: u8, n: u32) -> u8 { x << n }\n", "SH9");
    fires("pub fn f(x: u8, n: u32) -> u8 { x >> n }\n", "SH9");
    fires("pub fn f(x: u8) -> u8 { x >> (1 + 1) }\n", "SH9");
    fires("pub fn f(mut x: u8, n: u32) { x <<= n; }\n", "SH9");
    fires("pub fn f(mut x: u8, n: u32) { x >>= n; }\n", "SH9");
    fires("pub fn f(x: u8) -> u8 { x >> 1.0 as u8 }\n", "SH9");
}

#[test]
fn sh9_bare_arithmetic_is_deliberately_not_checked() {
    let _batch = Batch::begin();
    // VOS03 section 4 SH9: `+ - *` need clippy::arithmetic_side_effects, not a lexer.
    clean("pub fn f(a: u8, b: u8) -> u8 { a + b - a * b }\n");
}

// ---------------------------------------------------------------- SH10

#[test]
fn sh10_clean_fixed_width() {
    let _batch = Batch::begin();
    clean("pub struct S { a: u8, b: [u16; 4], c: bool }\npub fn f(s: S) -> (u32, i64, S) { (1u32, 2i64, s) }\n");
    clean("pub const LEN: usize = 4;\npub fn f(i: u8) -> u8 { let _j = i as usize; i }\n");
    clean("pub fn f<const N: usize>(_a: [u8; N]) {}\n");
}

#[test]
fn sh10_heap_and_host_width_types() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    clean("pub fn f(t: ((u8, u8), u8)) -> u8 { t.0.1 }\n");
    clean("pub fn f() -> u32 { 1_000u32 + 0xffu32 + 0b11u32 }\n");
}

#[test]
fn sh11_float_types_and_literals() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    manifest_clean("\n[dependencies]\nvos-ready-contract = { path = \"../vos-ready-contract\" }\nvos_timer_contract = { path = \"../t\" }\ncore = {}\n");
    manifest_clean("\n[dev-dependencies]\ncoding_adventures_sha256 = { path = \"../sha256\" }\ncoding_adventures_test_bootstrap = { path = \"../tb\" }\nvos-ready-contract = { path = \"../c\" }\n");
    manifest_clean("\n[dependencies.vos-ready-contract]\npath = \"../c\"\n");
}

#[test]
fn sh12_third_party_and_malformed_contract_dependencies() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    let mut i = body(
        "#[cfg(feature = \"ghost\")]\npub mod ghost { pub fn vos_ghost_alpha() -> u8 { 1 } }\n",
    );
    i.cargo_toml = Some(format!("{MANIFEST}{GHOSTLESS_FEATURES}"));
    check(i.clone(), Expect::Clean, "");
}

#[test]
fn sh13_ungated_ghost_module_stray_alpha_default_feature() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.sources.push(SourceFile {
        path: "src/ghost.rs".into(),
        text: "pub fn vos_ghost_alpha() {}\n".into(),
    });
    check(i.clone(), Expect::Fires("SH13".into()), "");
}

#[test]
fn sh13_gated_out_of_line_ghost_module_with_ghost_file_is_clean() {
    let _batch = Batch::begin();
    let mut i = body("#[cfg(feature = \"ghost\")]\npub mod ghost;\n");
    i.cargo_toml = Some(format!("{MANIFEST}{GHOSTLESS_FEATURES}"));
    i.sources.push(SourceFile {
        path: "src/ghost.rs".into(),
        text: "pub fn vos_ghost_alpha() -> u8 { 1 }\n".into(),
    });
    check(i.clone(), Expect::Clean, "");
}

// ---------------------------------------------------------------- SH14

fn kind_input(kind: &str, src: &str) -> CrateInput {
    let mut i = body(src);
    i.vos_toml = Some(format!("kind = \"{kind}\"\n"));
    i
}

#[test]
fn sh14_private_fields_clean_pub_fields_fire_for_components() {
    let _batch = Batch::begin();
    let clean_src = "pub struct S { a: u8, pub(crate) b: u8 }\n";
    for k in ["production", "reference"] {
        check(kind_input(k, clean_src), Expect::Clean, "");
    }
    for k in ["production", "reference"] {
        check(
            kind_input(k, "pub struct S { pub a: u8 }\n"),
            Expect::Fires("SH14".into()),
            "",
        );
    }
    let mut any = kind_input("any", "pub struct S { pub a: u8 }\n");
    any.cargo_toml = Some(MANIFEST.replace("vos-demo", "vos-demo-any"));
    check(any.clone(), Expect::Fires("SH14".into()), "");
}

#[test]
fn sh14_spine_contract_and_unknown_kind_may_have_pub_fields() {
    let _batch = Batch::begin();
    for k in ["spine", "contract", "hal", "profile"] {
        check(
            kind_input(k, "pub struct S { pub a: u8 }\n"),
            Expect::Clean,
            "",
        );
    }
    // an UNSPECIFIED kind in class K is treated as production (fail closed)
    check(
        body("pub struct S { pub a: u8 }\n"),
        Expect::Fires("SH14".into()),
        "",
    );
}

// ---------------------------------------------------------------- SH15

#[test]
fn sh15_clean_core_macros() {
    let _batch = Batch::begin();
    clean("pub fn f(x: u8) -> bool { matches!(x, 1) }\n");
    // `cfg!` varies the build between the evidence host and the image
    fires("pub fn g() -> bool { cfg!(test) }\n", "SH15");
    fires("pub fn g() -> bool { cfg!(test) }\n", "SH18");
    clean("pub const S: u8 = { let _a = concat!(\"a\", \"b\"); let _b = stringify!(x); let _c = file!(); let _d = column!(); let _e = module_path!(); 1 };\n");
}

#[test]
fn sh15_macros_includes_env_and_foreign_macros() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.root_entries.push("build.rs".into());
    check(i.clone(), Expect::Fires("SH15".into()), "");
    manifest_fires("build = \"build.rs\"\n", "SH15"); // appended under [package]
    manifest_fires("build = false\n", "SH15");
    manifest_fires("links = \"z\"\n", "SH15");
    // other spellings of the same key
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(
        "package.name = \"vos-demo\"\npackage.edition = \"2021\"\npackage.build = false\n".into(),
    );
    check(i.clone(), Expect::Fires("SH15".into()), "");
    i.cargo_toml =
        Some("package = { name = \"vos-demo\", edition = \"2021\", build = \"b.rs\" }\n".into());
    check(i.clone(), Expect::Fires("SH15".into()), "");
    i.cargo_toml = Some(
        "[\"package\"]\nname = \"vos-demo\"\nedition = \"2021\"\n\"build\" = \"b.rs\"\n".into(),
    );
    check(i.clone(), Expect::Fires("SH15".into()), "");
}

#[test]
fn sh15_proc_macro_and_target_paths() {
    let _batch = Batch::begin();
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
    check(i.clone(), Expect::Fires("SH15".into()), "");
}

// ---------------------------------------------------------------- SH17 and SH18 (toy-based)

fn bitmap() -> CrateInput {
    load("pass-ready-bitmap")
}

#[test]
fn sh17_the_toy_ghost_face_is_clean() {
    let _batch = Batch::begin();
    check(bitmap(), Expect::Clean, "");
}

#[test]
fn sh17_prefix_on_functions_and_reexports() {
    let _batch = Batch::begin();
    check(
        mutate(bitmap(), "pub fn vos_ghost_alpha(", "pub fn alpha_export("),
        Expect::Fires("SH17".into()),
        "",
    );
    check(
        mutate(
            bitmap(),
            "pub use ghost::{vos_ghost_alpha,",
            "pub use ghost::{alpha_gone, vos_ghost_alpha,",
        ),
        Expect::Fires("SH17".into()),
        "",
    );
    check(
        mutate(
            bitmap(),
            "pub use ghost::{vos_ghost_alpha,",
            "pub use ghost::{vos_ghost_alpha as plain,",
        ),
        Expect::Fires("SH17".into()),
        "",
    );
    check(mutate(bitmap(), "pub use ghost::{vos_ghost_alpha, vos_ghost_cfg_n, vos_ghost_init, vos_ghost_insert, vos_ghost_pick, vos_ghost_remove};", "pub use ghost::*;"), Expect::Fires("SH17".into()), "");
    check(
        mutate(
            bitmap(),
            "pub use ghost::{vos_ghost_alpha,",
            "pub use ghost::{vos_ghost_alpha as vos_ghost_a2,",
        ),
        Expect::Clean,
        "",
    );
}

#[test]
fn sh17_wrapper_must_have_exactly_one_ufcs_call_and_compute_nothing() {
    let _batch = Batch::begin();
    let two = "let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);\n        (o, Image { mask })                      // exhaustive";
    let m = mutate(bitmap(), two, "let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);\n        let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::remove(Bitmap { mask }, p);\n        (o, Image { mask })                      // exhaustive");
    check(m, Expect::Fires("SH17".into()), "");
    let branch = mutate(bitmap(), "(o, Image { mask })                      // exhaustive", "if p > 3 { (o, Image { mask }) } else { (o, Image { mask }) }                      // exhaustive");
    check(branch, Expect::Fires("SH17".into()), "");
    let op = mutate(
        bitmap(),
        "(o, Image { mask })                      // exhaustive",
        "(o, Image { mask: mask | 1 })                      // exhaustive",
    );
    check(op, Expect::Fires("SH17".into()), "");
    let extra_call = mutate(
        bitmap(),
        "<BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);",
        "<BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask.wrapping_add(0) }, p);",
    );
    check(extra_call, Expect::Fires("SH17".into()), "");
    let cast = mutate(bitmap(), "(Bitmap { mask: s.mask }, p);\n        (o, Image { mask })\n    }\n    pub fn vos_ghost_pick", "(Bitmap { mask: s.mask }, p);\n        (o, Image { mask: mask as u8 })\n    }\n    pub fn vos_ghost_pick");
    check(cast, Expect::Fires("SH17".into()), "");
    let other_call = mutate(
        bitmap(),
        "(o, Image { mask })                      // exhaustive",
        "(o, helper(Image { mask }))                      // exhaustive",
    );
    check(other_call, Expect::Fires("SH17".into()), "");
    let method = mutate(bitmap(), "pub fn vos_ghost_init() -> Image {\n        let Bitmap { mask } = <BitmapReady as ReadySet>::init();\n        Image { mask }", "pub fn vos_ghost_init() -> Image {\n        let Bitmap { mask } = <BitmapReady as ReadySet>::init();\n        let _z = mask.count_ones();\n        Image { mask }");
    check(method, Expect::Fires("SH17".into()), "");
}

#[test]
fn sh17_destructure_must_be_exhaustive_and_list_every_field() {
    let _batch = Batch::begin();
    let rest = mutate(
        bitmap(),
        "let Bitmap { mask } = <BitmapReady as ReadySet>::init();",
        "let Bitmap { .. } = <BitmapReady as ReadySet>::init();\n        let mask = 0;",
    );
    check(rest, Expect::Fires("SH17".into()), "");
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
    check(
        grown,
        Expect::Msg("SH17".into(), "missing: extra".into()),
        "",
    );
    // an unknown field in the pattern
    let unknown = mutate(
        bitmap(),
        "let Bitmap { mask } = <BitmapReady as ReadySet>::init();",
        "let Bitmap { mask, ghosty } = <BitmapReady as ReadySet>::init();",
    );
    check(unknown, Expect::Fires("SH17".into()), "");
}

#[test]
fn sh17_image_must_mirror_state_in_names_types_and_order() {
    let _batch = Batch::begin();
    let extra = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub mask: u8, pub more: u8 }",
    );
    check(extra, Expect::Fires("SH17".into()), "");
    let typed = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub mask: u16 }",
    );
    check(typed, Expect::Fires("SH17".into()), "");
    let renamed = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub bits: u8 }",
    );
    check(renamed, Expect::Fires("SH17".into()), "");
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
    let _batch = Batch::begin();
    let orphan = mutate(
        bitmap(),
        "// The abstraction and the configuration",
        "pub struct Other { pub q: u8 }\n    // The abstraction and the configuration",
    );
    check(orphan, Expect::Clean, "");
    let unpaired = mutate(
        bitmap(),
        "pub struct Image { pub mask: u8 }",
        "pub struct Image { pub mask: u8 }\n    pub struct Spare { pub z: u8 }",
    );
    check(unpaired, Expect::Clean, "");
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
    check(i.clone(), Expect::Fires("SH17".into()), "");
}

#[test]
fn sh18_clean_toy_and_every_leak_shape() {
    let _batch = Batch::begin();
    let leaked = mutate(
        bitmap(),
        "impl ReadySet for BitmapReady {",
        "pub fn leak() -> u8 { ghost::vos_ghost_cfg_n() }\nimpl ReadySet for BitmapReady {",
    );
    check(leaked, Expect::Fires("SH18".into()), "");
    let uses = mutate(
        bitmap(),
        "use vos_ready_contract::{Pick, ReadySet, Upd};",
        "use vos_ready_contract::{Pick, ReadySet, Upd};\nuse ghost::Image;",
    );
    check(uses, Expect::Fires("SH18".into()), "");
    let ungated_reexport = mutate(
        bitmap(),
        "#[cfg(feature = \"ghost\")]\npub use ghost::",
        "pub use ghost::",
    );
    check(ungated_reexport, Expect::Fires("SH18".into()), "");
    let private_reexport = mutate(
        bitmap(),
        "pub use ghost::{vos_ghost_alpha,",
        "use ghost::{vos_ghost_alpha,",
    );
    check(private_reexport, Expect::Fires("SH18".into()), "");
    let by_name = mutate(
        bitmap(),
        "impl ReadySet for BitmapReady {",
        "pub fn leak2() { let _ = vos_ghost_alpha; }\nimpl ReadySet for BitmapReady {",
    );
    check(by_name, Expect::Fires("SH18".into()), "");
}

#[test]
fn sh18_cfg_forms_of_evidence_features() {
    let _batch = Batch::begin();
    for (from, to) in [
        ("#[cfg(feature = \"ghost\")]", "#[cfg(not(feature = \"ghost\"))]"),
        ("#[cfg(feature = \"ghost\")]", "#[cfg(all(feature = \"ghost\", test))]"),
        ("#[cfg(feature = \"ghost\")]", "#[cfg_attr(feature = \"ghost\", derive(Clone))]"),
        ("fn init() -> Bitmap { Bitmap { mask: 0 } }", "fn init() -> Bitmap { if cfg!(feature = \"ghost\") { Bitmap { mask: 1 } } else { Bitmap { mask: 0 } } }"),
        ("pub struct BitmapReady;", "#[cfg(feature = \"ghost\")]\npub struct BitmapReady;"),
        ("pub struct BitmapReady;", "#[cfg(feature = \"cov\")]\npub struct BitmapReady;"),
    ] {
        check(mutate(bitmap(), from, to), Expect::Fires("SH18".into()), "");
    }
}

#[test]
fn sh18_feature_must_be_declared_and_non_evidence_features_do_not_gate_code() {
    let _batch = Batch::begin();
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
    check(tested, Expect::Clean, "");
}

#[test]
fn sh18_declared_evidence_features_extend_the_central_list() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
        "a.b = 1\n",
        "[table]\n",
        "class = 1\n",
        "evidence-features = [1]\n",
    ] {
        let mut i = body("pub fn f() {}\n");
        i.vos_toml = Some(text.to_string());
        check(i.clone(), Expect::Fires("VT01".into()), "");
    }
    // too long a file, too long a line, too many keys, too many names
    let mut i = body("pub fn f() {}\n");
    i.vos_toml = Some("# ".to_string() + &"x".repeat(5000));
    check(i.clone(), Expect::Fires("VT01".into()), "");
    i.vos_toml = Some(format!("# {}\n", "x".repeat(1100)));
    check(i.clone(), Expect::Fires("VT01".into()), "");
    i.vos_toml = Some((0..17).map(|n| format!("k{n} = \"v\"\n")).collect());
    check(i.clone(), Expect::Msg("VT01".into(), "16 keys".into()), "");
    i.vos_toml = Some(format!(
        "evidence-features = [{}]\n",
        (0..70)
            .map(|n| format!("\"f{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    check(i.clone(), Expect::Msg("VT01".into(), "64".into()), "");
    // a file of many comment lines is fine; many key lines are not, whatever they say
    i.vos_toml = Some("# c\n".repeat(100));
    check(i.clone(), Expect::Clean, "");
    i.vos_toml = Some(format!("evidence-features = [\"{}\"]\n", "x".repeat(40)));
    check(i.clone(), Expect::Fires("VT01".into()), "");
}

#[test]
fn vt01_valid_vos_toml_is_accepted() {
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.vos_toml = Some("# c\n\nclass = \"K\" # trailing\nkind = \"production\"\nevidence-features = [ \"ghost\" , \"cov\" ] # x\n".into());
    check(i.clone(), Expect::Clean, "");
    i.vos_toml = Some("evidence-features = []\n".into());
    check(i.clone(), Expect::Clean, "");
    i.vos_toml = Some("evidence-features = [\"ghost\",]\n".into());
    check(i.clone(), Expect::Clean, "");
}

#[test]
fn vt02_any_name_and_kind_must_agree() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
    for src in [
        "pub fn f() { \"unterminated }\n",
        "/* unterminated\n",
        "pub fn f() { ( }\n",
        "pub fn f() { ) }\n",
        "pub fn f() { ( ] }\n",
        "pub fn f() { r#\"never closed }\n",
        "pub fn f() { r#### }\n",
        "pub fn f() { b'x }\n",
        "pub fn f() { \u{2603} }\n",
    ] {
        check(lib(src), Expect::Fires("VT03".into()), "");
    }
}

#[test]
fn vt03_manifest_and_crate_root_problems() {
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = None;
    check(i.clone(), Expect::Fires("VT03".into()), "");
    i.cargo_toml = Some("[package\nname = 1".into());
    check(i.clone(), Expect::Fires("VT03".into()), "");
    i.cargo_toml = Some("x".repeat(70_000));
    check(i.clone(), Expect::Fires("VT03".into()), "");
    let mut i = body("pub fn f() {}\n");
    i.sources.clear();
    check(i.clone(), Expect::Fires("VT03".into()), "");
}

#[test]
fn vt03_size_caps() {
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.sources[0].text = format!("{}// {}\n", HEADER, "x".repeat(1_100_000));
    check(i.clone(), Expect::Fires("VT03".into()), "");
    let mut i = body("pub fn f() {}\n");
    for n in 0..520 {
        i.sources.push(SourceFile {
            path: format!("src/m{n}.rs"),
            text: String::new(),
        });
    }
    check(i.clone(), Expect::Fires("VT03".into()), "");
    let mut i = body("pub fn f() {}\n");
    for n in 0..9 {
        i.sources.push(SourceFile {
            path: format!("src/m{n}.rs"),
            text: format!("// {}\n", "x".repeat(1_000_000)),
        });
    }
    check(i.clone(), Expect::Fires("VT03".into()), "");
    // more than 4096 functions: the call graph is refused, fail closed
    let mut s = String::from(HEADER);
    for n in 0..4100 {
        s.push_str(&format!("fn f{n}() {{}}\n"));
    }
    check(lib(&s), Expect::Fires("VT03".into()), "");
    // too many tokens in one file
    let mut s = String::from(HEADER);
    s.push_str(&"a ".repeat(262_150));
    check(lib(&s), Expect::Fires("VT03".into()), "");
}

#[test]
fn a_binary_target_file_is_a_finding_and_not_a_crate_root() {
    let _batch = Batch::begin();
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
    let _batch = Batch::begin();
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
    check(i.clone(), Expect::Fires("VT03".into()), "");
}

#[test]
fn reports_are_sorted_deterministic_and_capped() {
    let _batch = Batch::begin();
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

// ---------------------------------------------------------------- SH19
//
// The command-type check (VOS-039b-2a, VOS03 section 4).  Every control asserts
// the rule id AND words of the message, because a control that fires for the
// wrong reason proves nothing.

const CONTRACT_TOML: &str = "class = \"K\"\nkind = \"contract\"\n";

/// A contract crate (`kind = "contract"`) holding `src` after the standard header.
fn contract(src: &str) -> CrateInput {
    let mut i = body(src);
    i.vos_toml = Some(CONTRACT_TOML.to_string());
    i
}

/// `src` in a contract crate must raise SH19 with `needle` in the message.
fn rejected(src: &str, needle: &str) {
    check(
        contract(src),
        Expect::Msg("SH19".into(), needle.into()),
        src,
    );
}

/// `src` in a contract crate must raise nothing at all.
fn accepted(src: &str) {
    check(contract(src), Expect::Clean, src);
}

#[test]
fn sh19_the_two_controls_of_the_spec_are_rejected_and_the_good_ones_pass() {
    let _batch = Batch::begin();
    rejected(
        "pub enum Cmd { Poke(usize) }\n",
        "enum `Cmd` variant `Poke` field `0`: type `usize` is not a command payload",
    );
    rejected(
        "pub enum Cmd { Write { addr: u32 }, Nop }\n",
        "enum `Cmd` variant `Write` field `addr`: type `u32`",
    );
    accepted("pub enum Pick { Idle, Run(u8) }\npub enum Upd { Changed, Unchanged, OutOfRange }\n");
}

#[test]
fn sh19_every_spelling_that_is_not_one_of_the_accepted_four_is_rejected() {
    let _batch = Batch::begin();
    for ty in [
        "usize",
        "isize",
        "u16",
        "u32",
        "u64",
        "i8",
        "i32",
        "char",
        "&'static u8",
        "*const u8",
        "(u8, u8)",
        "Option<u8>",
        "[u8]",
        "[u16; 4]",
        "[u8; N]",
        "[u8; 0x10]",
        "[u8; 4usize]",
        "[u8; 1_0]",
        "[u8; { 4 }]",
        "[[u8; 2]; 2]",
        "core::num::NonZeroU8",
        "Foo",
    ] {
        rejected(
            &format!("pub enum E {{ A({ty}), B }}\n"),
            "is not a command payload",
        );
    }
    // The message names the type as written.
    rejected(
        "pub enum E { A(core::num::NonZeroU8), B }\n",
        "type `core :: num :: NonZeroU8` is not a command payload",
    );
}

#[test]
fn sh19_a_type_alias_is_rejected_not_resolved() {
    let _batch = Batch::begin();
    rejected(
        "pub type Addr = usize;\npub enum E { A(Addr), B }\n",
        "a `type` alias is not resolved",
    );
    // Even an alias of an accepted type: the alias is not followed.
    rejected(
        "pub type Id = u8;\npub enum E { A(Id), B }\n",
        "a `type` alias is not resolved",
    );
    rejected(
        "pub union U { a: u8 }\npub enum E { A(U), B }\n",
        "a `union` is not a command payload",
    );
    rejected(
        "pub enum E { A(Missing), B }\n",
        "no struct or enum of this crate has that name",
    );
}

#[test]
fn sh19_non_exhaustive_and_generic_enums_are_rejected() {
    let _batch = Batch::begin();
    rejected(
        "#[non_exhaustive]\npub enum E { A, B }\n",
        "enum `E` is `#[non_exhaustive]`",
    );
    rejected(
        "pub enum E { A, #[non_exhaustive] B }\n",
        "enum `E` variant `B` is `#[non_exhaustive]`",
    );
    rejected("pub enum Cmd<T> { A(T), B }\n", "enum `Cmd` is generic");
    rejected(
        "pub enum Cmd<const N: usize> { A([u8; 4]), B }\n",
        "enum `Cmd` is generic",
    );
}

#[test]
fn sh19_a_nested_struct_or_enum_is_judged_by_what_it_holds() {
    let _batch = Batch::begin();
    // The root cause is named, however many hops away.
    rejected(
        "pub struct Inner { pub a: usize }\npub struct Outer { pub i: Inner }\npub enum E { A(Outer), B }\n",
        "type `Outer` is not a command payload (struct `Inner`, field `a`: type `usize`",
    );
    rejected(
        "pub struct S<T>(pub T);\npub enum E { A(S), B }\n",
        "struct `S` is generic",
    );
    rejected(
        "#[non_exhaustive]\npub struct S { pub a: u8 }\npub enum E { A(S), B }\n",
        "struct `S` is `#[non_exhaustive]`",
    );
    rejected(
        "pub enum Inner { X(u16), Y }\npub enum E { A(Inner), B }\n",
        "enum `Inner`, variant `X` field `0`: type `u16`",
    );
    accepted(
        "pub struct Id(pub u8);\npub struct Span { pub at: Id, pub len: u8, pub ok: bool }\npub enum Inner { X(Span), Y }\npub enum E { A(Inner), B([u8; 4]) }\n",
    );
}

#[test]
fn sh19_a_cycle_terminates_and_two_items_of_one_name_must_both_pass() {
    let _batch = Batch::begin();
    // Not compilable Rust, but the check must not loop on it.
    accepted("pub struct A { pub b: B }\npub struct B { pub a: A }\npub enum E { X(A), Y }\n");
    rejected(
        "pub struct A { pub b: B }\npub struct B { pub a: A, pub z: usize }\npub enum E { X(A), Y }\n",
        "struct `B`, field `z`",
    );
    // `S` is a good struct in one module and a bad one in another: both are judged.
    rejected(
        "pub mod a { pub struct S(pub u8); }\npub mod b { pub struct S(pub usize); }\npub enum E { X(S), Y }\n",
        "type `S` is not a command payload",
    );
}

#[test]
fn sh19_accepts_the_shapes_a_command_set_is_made_of() {
    let _batch = Batch::begin();
    accepted("pub enum E { A(bool), B(u8), C([u8; 4]), D([u8; 0]) }\n");
    accepted("pub enum E { A { n: u8, ok: bool }, B }\n");
    accepted("pub enum E { A = 1, B = 2, C }\n");
    // A discriminant on the LAST variant (no trailing comma), and a trailing attribute.
    accepted("pub enum E { A, B = 2 }\n");
    accepted("pub enum E { A, B, #[doc = \"z\"] }\n");
    accepted("pub enum E { #[doc = \"x\"] A(#[doc = \"y\"] u8), B }\n");
    accepted("enum Private { A(u8), B }\n");
    accepted("pub enum Empty {}\n");
    accepted("pub struct Unit;\npub enum E { A(Unit), B }\n");
}

#[test]
fn sh19_checks_private_enums_and_every_enum_of_the_crate() {
    let _batch = Batch::begin();
    // A private enum is in the derived set: the set is "every enum", not "every pub enum".
    rejected(
        "enum Private { A(usize), B }\n",
        "enum `Private` variant `A`",
    );
    // So is one inside a module and one inside a function body.
    rejected(
        "pub mod m { pub enum Inner { A(usize), B } }\n",
        "enum `Inner`",
    );
    rejected(
        "pub fn f() -> u8 { enum Local { A(usize), B } 0 }\n",
        "enum `Local`",
    );
}

#[test]
fn sh19_an_unreadable_variant_fails_closed() {
    let _batch = Batch::begin();
    // Each is a VT03 gap AND an SH19 finding.
    for src in [
        "pub enum E { A { 5 }, B }\n",
        "pub enum E { A B, C }\n",
        "pub enum E { 5, B }\n",
        "pub enum E<T { A, B }\n",
    ] {
        check(contract(src), Expect::Fires("VT03".into()), src);
    }
    for src in [
        "pub enum E { A { 5 }, B }\n",
        "pub enum E { A B, C }\n",
        "pub enum E { 5, B }\n",
    ] {
        rejected(src, "could not read");
    }
    // A tuple struct whose `where` clause runs into the next item is a gap too.
    check(
        contract("pub struct S(u8) where\npub fn g() {}\n"),
        Expect::Fires("VT03".into()),
        "tuple struct where",
    );
    // An empty payload type is not accepted either.
    rejected("pub enum E { A(,), B }\n", "is not a command payload");
}

#[test]
fn sh19_binds_by_kind_not_by_class() {
    let _batch = Batch::begin();
    let bad = "pub enum Cmd { Poke(usize) }\n";
    // The same enum in a production crate (or one with no declared kind) is silent.
    for kind in ["production", "reference", "any"] {
        let mut i = body(bad);
        i.vos_toml = Some(format!("class = \"K\"\nkind = \"{kind}\"\n"));
        check(i, Expect::Silent("SH19".into()), kind);
    }
    check(body(bad), Expect::Silent("SH19".into()), "no vos.toml");
    // A contract crate is judged whatever its class: class P binds no SH rule at all.
    for class in ["K", "C", "S", "M", "P", "H"] {
        let mut i = body(bad);
        i.vos_toml = Some(format!("class = \"{class}\"\nkind = \"contract\"\n"));
        check(i, Expect::Fires("SH19".into()), class);
    }
}

#[test]
fn sh19_the_spec_states_the_accepted_types_and_the_rule_id_prints() {
    let spec = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS03-component-shape-checker.md"),
    )
    .unwrap();
    let row = spec
        .lines()
        .find(|l| l.starts_with("| **SH19**"))
        .expect("VOS03 has an SH19 row");
    for t in vos_shape_check::command::ACCEPTED_TYPES {
        // The last entry is prose; the spec says "same-crate" and "struct" and "enum".
        let key = if t.starts_with("same-crate") {
            "same-crate"
        } else {
            t
        };
        assert!(row.contains(key), "the SH19 row does not state {t:?}");
    }
    assert_eq!(vos_shape_check::Rule::Sh19.id(), "SH19");
    let r = run(&contract("pub enum Cmd { Poke(usize) }\n"));
    let text = r.render();
    assert!(text.contains("kind contract"), "{text}");
    assert!(
        text.contains("SH19 src/lib.rs:3 enum `Cmd` variant `Poke` field `0`: type `usize`"),
        "{text}"
    );
}

#[test]
fn sh19_work_is_linear_in_the_number_of_variants() {
    scales_linearly("enum variants", 400, 4, |n| {
        let mut s = String::from("pub enum E {\n");
        for i in 0..n {
            s.push_str(&format!("    V{i}(u8, bool, [u8; 4]),\n"));
        }
        s.push_str("}\n");
        let (r, steps) = run_counted(&contract(&s));
        assert!(r.passed(), "{}", r.render());
        steps
    });
    // A long chain of structs is walked by a work list, not by recursion.
    scales_linearly("nested structs", 400, 4, |n| {
        let mut s = String::from("pub enum E { A(S0), B }\n");
        for i in 0..n {
            if i + 1 < n {
                s.push_str(&format!("pub struct S{i}(pub S{});\n", i + 1));
            } else {
                s.push_str(&format!("pub struct S{i}(pub usize);\n"));
            }
        }
        let (r, steps) = run_counted(&contract(&s));
        assert!(rules(&r).contains(&"SH19"), "{}", r.render());
        steps
    });
}

#[test]
fn sh19_a_very_deep_chain_does_not_overflow_the_stack() {
    // 20,000 nested structs: a recursive judge would need 20,000 frames.
    let n = 20_000;
    let mut s = String::from("pub enum E { A(S0), B }\n");
    for i in 0..n {
        if i + 1 < n {
            s.push_str(&format!("pub struct S{i}(pub S{});\n", i + 1));
        } else {
            s.push_str(&format!("pub struct S{i}(pub usize);\n"));
        }
    }
    let r = run(&contract(&s));
    assert!(rules(&r).contains(&"SH19"), "deep chain must be judged");
}
