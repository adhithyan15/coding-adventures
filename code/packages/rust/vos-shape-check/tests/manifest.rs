//! The TOML-subset reader, the `Cargo.toml` facts and the `vos.toml` grammar.

use std::collections::BTreeSet;
use vos_shape_check::manifest::{parse_cargo, parse_vos_toml, Class, Kind};
use vos_shape_check::toml::{parse, Value};

fn paths(text: &str) -> Vec<(String, Value)> {
    parse(text)
        .unwrap()
        .into_iter()
        .map(|e| (e.path.join("."), e.value))
        .collect()
}

fn value_of(text: &str, path: &str) -> Value {
    paths(text)
        .into_iter()
        .find(|(p, _)| p == path)
        .unwrap_or_else(|| panic!("no {path} in {text:?}"))
        .1
}

#[test]
fn every_spelling_of_a_key_flattens_to_one_path() {
    let want = Value::Str("x".into());
    for t in [
        "[package]\nbuild = \"x\"",
        "package.build = \"x\"",
        "package = { build = \"x\" }",
        "[\"package\"]\n\"build\" = \"x\"",
        "['package']\n'build' = 'x'",
        "[package]\nbuild=\"x\"# c",
        "package = {build=\"x\"}",
        "[ package ]\n  build  =  \"x\"  ",
        "[package]\nbuild = \"\"\"x\"\"\"",
        "[package]\nbuild = '''x'''",
        "[package]\nbuild = '''\nx'''",
        "[package]\nbuild = \"\"\"\nx\"\"\"",
        "[package]\nbuild = \"\"\"x\\\n   \"\"\"",
    ] {
        assert_eq!(value_of(t, "package.build"), want, "{t:?}");
    }
}

#[test]
fn values_arrays_tables_and_arrays_of_tables() {
    let t = "# top\n[a]\nb = true\nc = false\nd = 12\ne = 1.5e3\nf = 2024-01-01T00:00:00Z\ng = [1, 2,\n  # inside\n  3,]\nh = []\n[[bin]]\nname = \"x\"\n[[bin]]\nname = \"y\"\ni = { j = 1, k = { l = \"m\" } }\nn = [{ o = \"p\" }, { o = \"q\" }]\n";
    assert_eq!(value_of(t, "a.b"), Value::Bool(true));
    assert_eq!(value_of(t, "a.c"), Value::Bool(false));
    assert_eq!(value_of(t, "a.d"), Value::Other("12".into()));
    assert_eq!(value_of(t, "a.e"), Value::Other("1.5e3".into()));
    assert_eq!(
        value_of(t, "a.f"),
        Value::Other("2024-01-01T00:00:00Z".into())
    );
    assert_eq!(
        value_of(t, "a.g"),
        Value::Array(vec![
            Value::Other("1".into()),
            Value::Other("2".into()),
            Value::Other("3".into())
        ])
    );
    assert_eq!(value_of(t, "a.h"), Value::Array(vec![]));
    assert_eq!(value_of(t, "bin.i.k.l"), Value::Str("m".into()));
    assert_eq!(value_of(t, "bin.i"), Value::Table);
    assert_eq!(value_of(t, "bin.n.[].o"), Value::Str("p".into()));
    assert_eq!(
        parse(t)
            .unwrap()
            .iter()
            .filter(|e| e.path == ["bin"])
            .count(),
        2
    );
}

#[test]
fn string_escapes_and_lines() {
    assert_eq!(
        value_of("k = \"a\\tb\\n\\\\\\\"\\u00e9\\U0001F600\\r\\b\\f\"", "k"),
        Value::Str("a\tb\n\\\"\u{e9}\u{1F600}\r\u{8}\u{c}".into())
    );
    let e = parse("a = 1\nb = 2\n[t]\nc = \"x\"\n").unwrap();
    assert_eq!(
        e.iter().map(|x| x.line).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert!(parse("").unwrap().is_empty());
    assert!(parse("\r\na = 1\r\n").is_ok());
}

#[test]
fn toml_errors_fail_closed() {
    for t in [
        "a",
        "a =",
        "a = @",
        "a = \"open",
        "a = \"open\nnext\"",
        "a = 'open",
        "a = \"\"\"open",
        "a = '''open",
        "a = \"\\q\"",
        "a = \"\\u12\"",
        "a = \"\\uD800\"",
        "a = \"\\",
        "a = [1, 2",
        "a = [1 2]",
        "a = { b = 1",
        "a = { b 1 }",
        "a = { b = 1 c = 2 }",
        "[a",
        "[[a]",
        "[a] junk",
        "a = 1 junk",
        "= 1",
        "[a.]\n",
        "a.b. = 1",
        "a = \"x\" \"y\"",
    ] {
        assert!(parse(t).is_err(), "{t:?} should not parse");
    }
    let deep = format!("a = {}{}", "[".repeat(40), "]".repeat(40));
    assert!(parse(&deep).unwrap_err().message.contains("deep"));
    assert_eq!(parse("a = 1\nb = @").unwrap_err().line, 2);
}

#[test]
fn cargo_facts_name_edition_and_features() {
    let c = parse_cargo("[package]\nname = \"n\"\nedition = \"2021\"\n[features]\nghost = []\ndefault = [\"ghost\", 3]\nother = \"notalist\"\n").unwrap();
    assert_eq!(c.name.as_deref(), Some("n"));
    assert_eq!(c.edition.as_deref(), Some("2021"));
    assert_eq!(c.features["ghost"].1, Vec::<String>::new());
    assert_eq!(c.features["default"].1, vec!["ghost".to_string()]);
    assert!(c.features["other"].1.is_empty());
    let c = parse_cargo("[package]\nname = 3\nedition.workspace = true\n").unwrap();
    assert_eq!(c.name, None);
    assert_eq!(c.edition, None);
    let c = parse_cargo("[package]\nedition = 2021\n").unwrap();
    assert_eq!(c.edition, None);
    assert!(parse_cargo("[package").is_err());
}

#[test]
fn cargo_facts_dependencies_in_every_table_shape() {
    let c = parse_cargo(
        "[dependencies]\na = \"1\"\nb = { path = \"../b\" }\nc.workspace = true\nd = true\n[dependencies.e]\npath = \"../e\"\nfeatures = [\"x\"]\n[build-dependencies]\nf = \"1\"\n[dev-dependencies]\ng = \"1\"\n[target.'cfg(unix)'.dependencies]\nh = \"1\"\n[target.'cfg(unix)'.build-dependencies.i]\nversion = \"1\"\n",
    )
    .unwrap();
    let find = |n: &str| {
        c.deps
            .iter()
            .find(|d| d.name == n)
            .unwrap_or_else(|| panic!("{n}"))
    };
    let keys = |n: &str| find(n).keys.iter().cloned().collect::<Vec<_>>();
    assert_eq!(keys("a"), ["version"]);
    assert_eq!(keys("b"), ["path"]);
    assert_eq!(keys("c"), ["workspace"]);
    assert_eq!(keys("d"), ["other"]);
    assert_eq!(keys("e"), ["features", "path"]);
    assert_eq!(find("f").table, "build-dependencies");
    assert_eq!(find("g").table, "dev-dependencies");
    assert_eq!(find("h").table, "dependencies");
    assert_eq!(keys("i"), ["version"]);
    let names: BTreeSet<_> = c.deps.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names.len(), 9);
    // a table header with no keys still declares the dependency
    let c = parse_cargo("[dependencies.z]\n").unwrap();
    assert!(c.deps[0].keys.is_empty());
}

#[test]
fn cargo_facts_build_links_proc_macro_and_paths() {
    let c = parse_cargo("[package]\nbuild = false\nlinks = \"z\"\n[lib]\nproc-macro = true\ncrate-type = [\"cdylib\", \"proc-macro\"]\npath = \"lib.rs\"\n[[bin]]\npath = \"src/ok.rs\"\n").unwrap();
    assert_eq!(c.build_keys.len(), 1);
    assert_eq!(c.links_keys.len(), 1);
    assert_eq!(c.proc_macro.len(), 2);
    assert_eq!(c.bad_paths.len(), 1);
    let c = parse_cargo("[lib]\nproc-macro = false\ncrate-type = 3\npath = \"a\\\\..\\\\b.rs\"\n")
        .unwrap();
    assert!(c.proc_macro.is_empty());
    assert_eq!(c.bad_paths.len(), 1);
    let c = parse_cargo("[lib]\npath = \"C:/x/src/lib.rs\"\n").unwrap();
    assert_eq!(c.bad_paths.len(), 1);
    // `[[test]]` and `[dependencies] path` are not target paths
    let c =
        parse_cargo("[[test]]\npath = \"tests/x.rs\"\n[dependencies]\nq = { path = \"../q\" }\n")
            .unwrap();
    assert!(c.bad_paths.is_empty());
}

#[test]
fn vos_toml_grammar() {
    let v = parse_vos_toml(
        "# hi\nclass = \"M\"\nkind = \"reference\"\nevidence-features = [\"ghost\", \"cov\"]\n",
    )
    .unwrap();
    assert_eq!(v.class, Some(Class::M));
    assert_eq!(v.kind, Some(Kind::Reference));
    assert_eq!(v.evidence_features, ["ghost", "cov"]);
    for (s, c) in [
        ("K", Class::K),
        ("C", Class::C),
        ("S", Class::S),
        ("M", Class::M),
        ("P", Class::P),
        ("H", Class::H),
    ] {
        assert_eq!(
            parse_vos_toml(&format!("class = \"{s}\"")).unwrap().class,
            Some(c)
        );
        assert_eq!(c.letter(), s);
    }
    for (s, k) in [
        ("contract", Kind::Contract),
        ("production", Kind::Production),
        ("reference", Kind::Reference),
        ("any", Kind::Any),
        ("spine", Kind::Spine),
        ("hal", Kind::Hal),
        ("profile", Kind::Profile),
    ] {
        let v = parse_vos_toml(&format!("kind = \"{s}\"")).unwrap();
        assert_eq!(v.kind, Some(k));
        assert_eq!(k.name(), s);
    }
    let v = parse_vos_toml("").unwrap();
    assert_eq!((v.class, v.kind), (None, None));
    let e = parse_vos_toml("class = \"K\"\nkind = \"nope\"").unwrap_err();
    assert_eq!(e.0, 2);
}
