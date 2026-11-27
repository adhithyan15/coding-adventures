//! The TOML-subset reader, the `Cargo.toml` facts and the `vos.toml` grammar.

use std::collections::BTreeSet;
use vos_shape_check::manifest::{parse_cargo, parse_vos_toml, Class, Kind, SatLane};
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
        "[package]\nbuild = \"\"\"x\"\"\"",
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
fn escapes_are_refused_and_lines_are_kept() {
    // The TOML lexer leaves escapes of basic strings undecoded, so a basic or
    // multi-line basic string with a backslash (a key OR a value) is refused
    // (fail closed); literal strings have no escapes and are fine.
    for bad in [
        "k = \"a\\tb\"",
        "k = \"\\u00e9\"",
        "\"pack\\u0061ge\".build = 1",
        "k = \"\"\"x\\\n y\"\"\"",
        "[\"a\\nb\"]",
    ] {
        let e = parse(bad).unwrap_err();
        assert!(e.message.contains("escape"), "{bad:?}: {}", e.message);
    }
    assert_eq!(value_of("k = 'a\\tb'", "k"), Value::Str("a\\tb".into()));
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
    // nesting is bounded by the grammar parser's own recursion cap (about 53
    // levels of arrays), beyond which it refuses
    let ok_depth = format!("a = {}{}", "[".repeat(20), "]".repeat(20));
    assert!(parse(&ok_depth).is_ok());
    for n in [70usize, 300] {
        let deep = format!("a = {}{}", "[".repeat(n), "]".repeat(n));
        assert!(parse(&deep).is_err(), "{n} levels");
    }
    // the grammar crates give no position for a lexical or syntax error
    assert_eq!(parse("a = 1\nb = @").unwrap_err().line, 1);
    // duplicate keys are refused (the grammar crate accepts them), also via dotted
    // keys and inline tables; the same key under two `[[x]]` tables is fine
    for dup in [
        "a = 1\na = 2\n",
        "a.b = 1\na = { b = 2 }\na.b = 3\n",
        "[t]\nx = 1\nx = 2\n",
    ] {
        assert!(
            parse(dup).unwrap_err().message.contains("duplicate"),
            "{dup:?}"
        );
    }
    assert!(parse("[[b]]\nn = 1\n[[b]]\nn = 2\n").is_ok());
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
    let c =
        parse_cargo("[lib]\nproc-macro = false\ncrate-type = 3\npath = 'a\\..\\b.rs'\n").unwrap();
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

#[test]
fn vos_toml_contract_and_obligations_keys() {
    // VOS02 section 10.1 (D-GD2): an implementation names its contract package and the
    // tiers gen-doc embeds.  This reader only bounds them; vos-link judges the values.
    let v = parse_vos_toml(
        "kind = \"production\"\ncontract = \"vos-ready-contract\"\nobligations = [\"canonical\", \"safety\"]\n",
    )
    .unwrap();
    assert_eq!(v.contract.as_deref(), Some("vos-ready-contract"));
    assert_eq!(v.obligations, ["canonical", "safety"]);
    let none = parse_vos_toml("kind = \"contract\"\n").unwrap();
    assert_eq!(none.contract, None);
    assert!(none.obligations.is_empty());
    for (text, why) in [
        ("contract = \"a\"\ncontract = \"b\"", "duplicate key"),
        ("contract = \"\"", "1 to 32 bytes"),
        (
            "contract = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
            "1 to 32 bytes",
        ),
        ("contract = 3", "wrong type"),
        ("obligations = \"safety\"", "wrong type"),
        (
            "obligations = [\"a\"]\nobligations = [\"b\"]",
            "duplicate key",
        ),
        ("obligations = [1]", "holds strings"),
        ("obligations = [\"\"]", "1 to 16 bytes"),
        (
            "obligations = [\"a\", \"b\", \"c\", \"d\", \"e\", \"f\"]",
            "more than 5",
        ),
        ("obligation = [\"a\"]", "unknown key"),
        ("coverage-floor = 101", "integer from 0 to 100"),
        ("coverage-floor = -1", "integer from 0 to 100"),
        ("coverage-floor = 9_5", "integer from 0 to 100"),
        ("coverage-floor = 95.0", "integer from 0 to 100"),
        ("coverage-floor = 0x5f", "integer from 0 to 100"),
        ("coverage-floor = 0095", "integer from 0 to 100"),
        ("coverage-floor = \"95\"", "wrong type"),
        ("coverage-floor = 95\ncoverage-floor = 96", "duplicate key"),
        ("excused-controls = []", "never empty"),
        ("excused-controls = [\"a\"]", "inline tables"),
        ("excused-controls = \"a\"", "wrong type"),
        ("excused-controls = [{ id = \"a\" }]", "exactly one `id` and one `reason`"),
        ("excused-controls = [{ reason = \"VOS-D098\" }]", "exactly one `id` and one `reason`"),
        ("excused-controls = [{ id = \"\", reason = \"VOS-D098\" }]", "1 to 128 bytes"),
        ("excused-controls = [{ id = \"a b\", reason = \"VOS-D098\" }]", "1 to 128 bytes"),
        ("excused-controls = [{ id = \"a\", reason = \"because\" }]", "backlog row"),
        ("excused-controls = [{ id = \"a\", reason = \"VOS-D98\" }]", "backlog row"),
        ("excused-controls = [{ id = \"a\", reason = \"VOS-D0981\" }]", "backlog row"),
        ("excused-controls = [{ id = \"a\", reason = \"\" }]", "backlog row"),
        ("excused-controls = [{ id = \"a\", why = \"x\" }]", "`id` and `reason` only"),
        ("excused-controls = [{ id = 1, reason = \"VOS-D098\" }]", "are strings"),
        ("excused-controls = [{ id = \"a\", reason = \"VOS-D098\" }, { id = \"a\", reason = \"VOS-D098\" }]", "listed twice"),
    ] {
        let e = parse_vos_toml(text).unwrap_err();
        assert!(e.1.contains(why), "{text:?}: {e:?}");
    }
}

#[test]
fn coverage_floor_is_an_integer_percent_and_optional() {
    // VOS-039a: the declared floor of the coverage gate (VOS04 section 5.6).
    assert_eq!(
        parse_vos_toml("kind = \"any\"").unwrap().coverage_floor,
        None
    );
    for (text, want) in [
        ("coverage-floor = 95", 95),
        ("coverage-floor = 0", 0),
        ("coverage-floor = 100", 100),
        ("coverage-floor = 7", 7),
    ] {
        assert_eq!(
            parse_vos_toml(text).unwrap().coverage_floor,
            Some(want),
            "{text}"
        );
    }
}

#[test]
fn excused_controls_are_whole_plain_ids_with_a_backlog_reason() {
    let v = parse_vos_toml(
        "excused-controls = [{ id = \"m/negctl/exhaustive/init/field-flip\", reason = \"VOS-D098\" }, { reason = \"VOS-D065\", id = \"x\" }]",
    )
    .unwrap();
    let got: Vec<(&str, &str)> = v
        .excused_controls
        .iter()
        .map(|e| (e.id.as_str(), e.reason.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("m/negctl/exhaustive/init/field-flip", "VOS-D098"),
            ("x", "VOS-D065")
        ]
    );
    assert!(parse_vos_toml("kind = \"any\"")
        .unwrap()
        .excused_controls
        .is_empty());
    let many: Vec<String> = (0..17)
        .map(|i| format!("{{ id = \"i{i}\", reason = \"VOS-D098\" }}"))
        .collect();
    let e = parse_vos_toml(&format!("excused-controls = [{}]", many.join(","))).unwrap_err();
    assert!(e.1.contains("more than 16"), "{e:?}");
}

#[test]
fn sat_key_is_a_closed_enum_with_one_member() {
    // Absent means off (VOS31 section 10.3: opt-in per contract).
    assert_eq!(parse_vos_toml("kind = \"contract\"").unwrap().sat, None);
    let v = parse_vos_toml("kind = \"contract\"\nsat = \"model\"\n").unwrap();
    assert_eq!(v.sat, Some(SatLane::Model));
    assert_eq!(SatLane::Model.name(), "model");
    for (text, want) in [
        ("sat = \"Model\"", "sat must be"),
        ("sat = \"\"", "sat must be"),
        ("sat = \"model \"", "sat must be"),
        ("sat = \"none\"", "sat must be"),
        ("sat = \"model\"\nsat = \"model\"", "duplicate key"),
        ("sat = true", "wrong type"),
        ("sat = 1", "wrong type"),
        ("sat = [\"model\"]", "wrong type"),
    ] {
        let e = parse_vos_toml(text).unwrap_err();
        assert!(e.1.contains(want), "{text}: {e:?}");
    }
    // The rejection message of an unknown key names the new key.
    let e = parse_vos_toml("bogus = \"x\"").unwrap_err();
    assert!(e.1.contains(", sat)"), "{e:?}");
}
