//! Structural edge cases: the shapes of `impl` headers, traits, attributes and
//! manifests that the other suites do not reach.  Each asserts a *behaviour*
//! (a finding appears or does not), not just that nothing panics.

mod common;
use common::*;
use vos_shape_check::SourceFile;

fn fires(src: &str, id: &str) {
    check(body(src), Expect::Fires(id.into()), src);
}

fn clean(src: &str) {
    check(body(src), Expect::Clean, src);
}

#[test]
fn impl_headers_of_every_shape_resolve_to_their_type() {
    let _batch = Batch::begin();
    // path type, generics on the impl and the type
    fires("mod a { pub struct B; }\npub trait Tr { fn f(); }\nimpl<T> Tr for a::B { fn f() { Self::f() } }\n", "SH5");
    // generics containing brackets
    fires("pub trait Tr<T> { fn g(); }\npub struct S<T>(T);\nimpl<T: Tr<[u8; 2]>> S<T> { fn g() { Self::g() } }\n", "SH5");
    // a reference self type
    fires("pub trait Tr { fn f(); }\npub struct S;\nimpl Tr for &'static mut S { fn f() { Self::f() } }\n", "SH5");
    // a where clause with a tuple argument
    fires("pub trait Tr3<T> {}\npub struct S<T>(T);\nimpl<T> S<T> where T: Tr3<(u8, u8)> { fn m() { Self::m() } }\n", "SH5");
    // an array self type has no name: it gets a synthetic type key, so `self.f()` still has an edge
    fires(
        "pub trait Tr { fn f(self); }\nimpl Tr for [u8; 2] { fn f(self) { self.f() } }\n",
        "SH5",
    );
    // a header with no body is skipped without a panic
    clean("pub struct S;\nimpl S;\n");
    // clean: the same method name on two different types is not recursion
    clean("pub struct A;\npub struct B;\nimpl A { pub fn f() {} }\nimpl B { pub fn f() { A::f() } }\n");
}

#[test]
fn trait_headers_and_default_methods() {
    let _batch = Batch::begin();
    fires("pub(crate) trait T2 { fn h() { Self::h() } }\n", "SH5");
    fires(
        "pub trait Tr<T> {}\npub trait V: Tr<(u8,)> { fn v() { Self::v() } }\n",
        "SH5",
    );
    fires("pub unsafe trait U { fn u() { Self::u() } }\n", "SH2");
    clean("pub trait W { fn w(); }\n");
}

#[test]
fn call_qualifiers_that_name_nothing() {
    let _batch = Batch::begin();
    // a free function using `Self::` outside an impl is a free call
    fires("fn z() { Self::z() }\n", "SH5");
    // qualifiers that are neither a path nor a qualified type name no edge
    // lexical shapes with a `>` before `::name(` and no matching `<` must not panic
    check(
        body("pub fn w(a: u8) -> bool { a > ::g(1) }\npub fn g(_x: u8) -> u8 { 0 }\n"),
        Expect::Silent("SH5".into()),
        "",
    );
    check(
        body("pub fn w(a: u8) -> bool { (a)::g(1); true }\n"),
        Expect::Silent("SH5".into()),
        "",
    );
}

#[test]
fn the_ufcs_call_graph_follows_qualified_self_types() {
    let _batch = Batch::begin();
    fires(
        "pub struct S;\npub trait T { fn f(); }\nimpl T for S { fn f() { <Self as T>::f() } }\n",
        "SH5",
    );
    clean("pub trait T { fn f(); }\npub struct S;\nimpl T for S { fn f() { <P as T>::f() } }\n");
}

#[test]
fn struct_bodies_and_generics_are_read_correctly() {
    let _batch = Batch::begin();
    // generic struct with a where clause: fields still found, refs still caught
    fires(
        "pub struct S<T> where T: Copy { pub a: &'static u8, pub b: T }\n",
        "SH10",
    );
    // a field named like a keyword-ish ident, and a method called `union`
    clean("pub struct S { union: u8 }\npub fn f(s: S) -> u8 { s.union }\n");
    clean("pub fn union(a: u8) -> u8 { a }\npub fn g() -> u8 { union(1) }\n");
    // attributes on fields and a tuple-like body do not confuse the field reader
    let mut i = body("pub struct S { #[doc = \"x\"] pub a: u8, (u8), pub b: u8 }\n");
    i.vos_toml = Some("kind = \"production\"\n".into());
    check(i.clone(), Expect::Fires("SH14".into()), "");
    // a truncated item at end of file cannot be read to its end: VT03 (fail closed)
    check(body("pub struct S"), Expect::Fires("VT03".into()), "");
    // a union with generics is parsed like a struct
    fires(
        "pub union U<T: Copy> { pub a: &'static u8, pub b: T }\n",
        "SH10",
    );
}

#[test]
fn cfg_test_masking_in_odd_positions() {
    let _batch = Batch::begin();
    // an attribute stack after cfg(test)
    clean("#[cfg(test)]\n#[derive(Clone)]\nstruct T { a: f32 }\npub fn f() {}\n");
    // a cfg(test) item with no terminator, and a trailing attribute with no item
    clean("pub fn f() {}\n#[cfg(test)]\nconst X: f32 = 1.0");
    clean("pub fn f() {}\n#[cfg(test)]");
    // cfg(test) on a field and on a statement
    clean(
        "pub struct S { #[cfg(test)] a: f32, b: u8 }\npub fn f() { #[cfg(test)] let _x = 1.0; }\n",
    );
    // an attribute that is not exactly cfg(test) is not a mask
    fires("#[cfg(not(test))]\nstruct T { a: f32 }\n", "SH11");
}

#[test]
fn attributes_between_the_gate_and_the_ghost_item() {
    let _batch = Batch::begin();
    let mk = |attr_line: &str, vis: &str| {
        let mut i = body(&format!("#[cfg(feature = \"ghost\")]\n{attr_line}{vis}mod ghost {{ pub fn vos_ghost_alpha() -> u8 {{ 1 }} }}\n"));
        i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
        run(&i)
    };
    for (attr, vis) in [
        ("", "pub "),
        ("#[allow(dead_code)]\n", "pub "),
        ("", "pub(crate) "),
        ("", ""),
        ("#[doc = \"x\"]\n", "pub(crate) "),
    ] {
        let r = mk(attr, vis);
        assert!(r.findings.is_empty(), "{attr:?} {vis:?}\n{}", r.render());
    }
    // `pub use self::ghost::..` is accepted when gated
    let mut i = body("#[cfg(feature = \"ghost\")]\npub mod ghost { pub fn vos_ghost_alpha() -> u8 { 1 } }\n#[cfg(feature = \"ghost\")]\npub use self::ghost::vos_ghost_alpha;\n");
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
    check(i.clone(), Expect::Clean, "");
    // a gated `pub fn`/`use` that is not a ghost item is not a gate for it
    let mut i = body("#[cfg(feature = \"ghost\")]\npub fn other() {}\n");
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
    check(i.clone(), Expect::Fires("SH18".into()), "");
    let mut i = body("#[cfg(feature = \"ghost\")]\nuse core::cmp::min;\n");
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
    check(i.clone(), Expect::Fires("SH18".into()), "");
}

#[test]
fn ghost_scope_shapes_for_sh17() {
    let _batch = Batch::begin();
    let ghost = |inner: &str| {
        let mut i = body(&format!(
            "#[cfg(feature = \"ghost\")]\npub mod ghost {{\n{inner}\n}}\n"
        ));
        i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
        run(&i)
    };
    // a function-pointer type and a bodiless declaration inside ghost
    let r = ghost("pub fn vos_ghost_alpha(_f: fn(u8)) {}\npub trait X { fn vos_ghost_cfg_n(); }");
    assert!(rules(&r).contains(&"SH4"), "{}", r.render());
    assert!(!rules(&r).contains(&"SH17"), "{}", r.render());
    // a stray `> ::f(` with no `<` is not a UFCS call, so the fn is an export
    let r = ghost("pub fn vos_ghost_alpha() -> bool { 1 > ::core::convert::identity(1) }");
    assert!(!rules(&r).contains(&"SH17"), "{}", r.render());
    // a nested group in a destructure
    let r = ghost("pub struct S { a: (u8, u8) }\npub fn vos_ghost_alpha() -> u8 { let S { a: (x, _y) } = <S as T>::make(); x }");
    assert!(!rules(&r).contains(&"SH17"), "{}", r.render());
    // a `<` in a UFCS qualifier with nested generics
    let r = ghost("pub fn vos_ghost_alpha() -> u8 { <Foo<Bar<u8>> as T>::make() }");
    assert!(!rules(&r).contains(&"SH17"), "{}", r.render());
    // an `as` outside the call is computation
    let r = ghost("pub fn vos_ghost_f() -> u8 { let x = <Foo as T>::make(); x as u8 }");
    assert!(rules(&r).contains(&"SH17"), "{}", r.render());
}

#[test]
fn default_feature_cycles_do_not_hang() {
    let _batch = Batch::begin();
    let mut i = body("pub fn f() {}\n");
    i.cargo_toml = Some(format!(
        "{MANIFEST}\n[features]\ndefault = [\"a\"]\na = [\"b\"]\nb = [\"a\", \"dep:x\", \"y/z\"]\n"
    ));
    check(i.clone(), Expect::Clean, "");
}

#[test]
fn tokens_at_the_very_start_of_a_file() {
    let _batch = Batch::begin();
    // a `|` or `[` with no previous token
    check(lib("|x| x"), Expect::Fires("SH4".into()), "");
    check(lib("[0u8; 2];"), Expect::Silent("SH8".into()), "");
}

#[test]
fn ghost_files_in_subdirectories_are_ghost_scope() {
    let _batch = Batch::begin();
    let mut i = body("#[cfg(feature = \"ghost\")]\npub mod ghost;\n");
    i.cargo_toml = Some(format!("{MANIFEST}\n[features]\nghost = []\n"));
    i.sources.push(SourceFile {
        path: "src/ghost/mod.rs".into(),
        text: "pub mod inner;\n".into(),
    });
    i.sources.push(SourceFile {
        path: "src/ghost/inner.rs".into(),
        text: "pub fn not_prefixed() {}\n".into(),
    });
    check(i.clone(), Expect::Fires("SH17".into()), "");
}
