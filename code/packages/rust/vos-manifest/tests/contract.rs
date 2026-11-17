//! `contract.vos`: parse, caps, word classes, ordering, canonical form, digest.

use vos_manifest::contract::{
    is_ident, is_key, is_opname, parse_contract, ContractErrorKind as K, Metric, Tier,
};

const GOOD: &str = "vos-contract 1\nname vos-sched-contract\niface sched\ntier any\ntier canonical\ntier policy\ntier safety\nconfig N 8\nceiling dispatch depth 2\nceiling dispatch heap 0\nceiling dispatch steps 160\nceiling make_ready steps 9\nend\n";

fn kind(text: &str) -> K {
    parse_contract(text.as_bytes()).expect_err(text).kind
}

#[test]
fn the_good_contract_parses_and_re_emits_byte_equal() {
    let c = parse_contract(GOOD.as_bytes()).expect("parses");
    assert_eq!(c.package, "vos-sched-contract");
    assert_eq!(c.iface, "sched");
    assert_eq!(c.tiers, Tier::ALL.to_vec());
    assert_eq!(c.config_value("N"), Some(8));
    assert_eq!(c.config_value("Z"), None);
    assert_eq!(c.ceilings.len(), 4);
    assert_eq!(
        c.ceilings[3].op, "make_ready",
        "underscored operation names (VOS-D051)"
    );
    assert_eq!(c.ceilings[2].metric, Metric::Steps);
    assert_eq!(c.emit(), GOOD);
    assert!(c.has_tier(Tier::Canonical));
    assert_eq!(Tier::parse("canonical"), Some(Tier::Canonical));
    assert_eq!(Tier::parse("Canonical"), None);
    assert_eq!(
        c.digest(),
        vos_manifest::digest::sha256_hex(GOOD.as_bytes())
    );
}

#[test]
fn the_real_scheduler_contract_parses() {
    let bytes = include_bytes!("../../vos-sched-contract/contract.vos");
    let c = parse_contract(bytes).expect("the real contract.vos is canonical");
    assert_eq!(c.package, "vos-sched-contract");
    assert_eq!(c.config_value("N"), Some(8));
    assert_eq!(c.tiers.len(), 4);
}

#[test]
fn an_empty_tier_list_and_no_rows_are_a_contract() {
    let c = parse_contract(b"vos-contract 1\nname a\niface b\nend\n").unwrap();
    assert!(c.tiers.is_empty() && c.config.is_empty() && c.ceilings.is_empty());
}

#[test]
fn caps_are_checked_before_lexing() {
    let big = vec![b'a'; 64 * 1024 + 1];
    let e = parse_contract(&big).unwrap_err();
    assert_eq!(e.kind, K::TooLarge);
    assert!(e.is_cap());
    // A long line inside the file cap.
    let long = format!("vos-contract 1\nname {}\nend\n", "a".repeat(1_100));
    let e = parse_contract(long.as_bytes()).unwrap_err();
    assert_eq!(e.kind, K::LineTooLong);
    assert_eq!(e.line, 2);
    assert!(e.is_cap());
    // A cap is not a content error.
    assert!(!parse_contract(b"nonsense\n").unwrap_err().is_cap());
}

#[test]
fn bytes_outside_ascii_and_tabs_and_crs_are_refused() {
    assert_eq!(parse_contract(&[0xff, 0xfe]).unwrap_err().kind, K::NotAscii);
    assert_eq!(
        kind("vos-contract 1\nname caf\u{e9}\niface b\nend\n"),
        K::NotAscii
    );
    assert_eq!(
        kind("vos-contract 1\nname a\tb\niface b\nend\n"),
        K::Lexical
    );
    assert_eq!(kind("vos-contract 1\r\nname a\niface b\nend\n"), K::Lexical);
}

#[test]
fn the_grammar_refuses_malformed_structure() {
    assert_eq!(kind(""), K::Syntax);
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nend"),
        K::Syntax,
        "no final LF"
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nend\nend\n"),
        K::Syntax
    );
    assert_eq!(kind("name a\niface b\nend\n"), K::Syntax, "no header");
    assert_eq!(
        kind("vos-contract 1\niface b\nname a\nend\n"),
        K::Syntax,
        "rows out of order"
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nconfig N 08\nend\n"),
        K::Syntax,
        "leading zero"
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling d depth\nend\n"),
        K::Syntax
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling d steps 1\nconfig N 1\nend\n"),
        K::Syntax,
        "config rows come before ceiling rows"
    );
}

#[test]
fn the_version_is_one() {
    assert_eq!(kind("vos-contract 2\nname a\niface b\nend\n"), K::Version);
    assert_eq!(kind("vos-contract 0\nname a\niface b\nend\n"), K::Version);
}

#[test]
fn word_classes_are_checked_per_row() {
    // Package and interface: hyphens, no underscore, no capital, no digit first.
    assert_eq!(kind("vos-contract 1\nname a_b\niface b\nend\n"), K::BadName);
    assert_eq!(kind("vos-contract 1\nname A\niface b\nend\n"), K::BadName);
    assert_eq!(kind("vos-contract 1\nname a\niface b_c\nend\n"), K::BadName);
    let long = "a".repeat(33);
    assert_eq!(
        kind(&format!("vos-contract 1\nname {long}\niface b\nend\n")),
        K::BadName
    );
    // Operation names: underscores yes, hyphens no.
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling a-b depth 1\nend\n"),
        K::BadName
    );
    // Keys: letters, digits, underscore, capitals allowed; hyphen not.
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nconfig a-b 1\nend\n"),
        K::BadName
    );
    // The class functions themselves.
    assert!(is_ident("vos-a1") && !is_ident("1a") && !is_ident("a_b") && !is_ident(""));
    assert!(is_opname("make_ready") && !is_opname("a-b") && !is_opname("A"));
    assert!(is_key("N") && is_key("Max_threads") && !is_key("1N") && !is_key("a-b"));
    assert!(is_ident(&"a".repeat(32)) && !is_ident(&"a".repeat(33)));
}

#[test]
fn numbers_must_fit_u64() {
    let ok = "vos-contract 1\nname a\niface b\nconfig N 18446744073709551615\nend\n";
    assert_eq!(
        parse_contract(ok.as_bytes()).unwrap().config[0].value,
        u64::MAX
    );
    assert_eq!(kind(&ok.replace("615", "616")), K::BadNumber);
    // Twenty digits above u64::MAX: one DEC token, refused by the range check.
    assert_eq!(
        kind(&ok.replace("18446744073709551615", "99999999999999999999")),
        K::BadNumber
    );
    // Twenty-one digits: two DEC tokens, refused by the grammar.
    assert_eq!(
        kind(&ok.replace("18446744073709551615", "100000000000000000000")),
        K::Syntax
    );
    assert_eq!(
        kind(&ok.replace("18446744073709551615", "123456789012345678901")),
        K::Syntax
    );
}

#[test]
fn tiers_metrics_and_row_order_are_enforced() {
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\ntier fast\nend\n"),
        K::BadTier
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\ntier safety\ntier any\nend\n"),
        K::Unsorted
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\ntier any\ntier any\nend\n"),
        K::Duplicate
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nconfig N 1\nconfig A 2\nend\n"),
        K::Unsorted
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nconfig N 1\nconfig N 2\nend\n"),
        K::Duplicate
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling d speed 1\nend\n"),
        K::BadMetric
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling d steps 1\nceiling d depth 1\nend\n"),
        K::Unsorted
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling e depth 1\nceiling d depth 1\nend\n"),
        K::Unsorted
    );
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\nceiling d depth 1\nceiling d depth 2\nend\n"),
        K::Duplicate
    );
}

#[test]
fn row_count_caps() {
    let mut cfg = String::from("vos-contract 1\nname a\niface b\n");
    for i in 0..65 {
        cfg.push_str(&format!("config K{i:02} 1\n"));
    }
    cfg.push_str("end\n");
    assert_eq!(kind(&cfg), K::TooManyRows);
    let mut ceil = String::from("vos-contract 1\nname a\niface b\n");
    for i in 0..1_025 {
        ceil.push_str(&format!("ceiling o{i:04} depth 1\n"));
    }
    ceil.push_str("end\n");
    assert_eq!(kind(&ceil), K::TooManyRows);
    // Exactly at the caps is fine.
    let mut ok = String::from("vos-contract 1\nname a\niface b\n");
    for i in 0..64 {
        ok.push_str(&format!("config K{i:02} 1\n"));
    }
    for i in 0..1_024 {
        ok.push_str(&format!("ceiling o{i:04} depth 1\n"));
    }
    ok.push_str("end\n");
    assert!(ok.len() <= 64 * 1024);
    parse_contract(ok.as_bytes()).expect("at the caps");
    // Five tiers cannot be written sorted and unique, but the cap still guards.
    assert_eq!(
        kind("vos-contract 1\nname a\niface b\ntier any\ntier canonical\ntier policy\ntier safety\ntier safety\nend\n"),
        K::TooManyRows
    );
}

#[test]
fn two_spellings_of_one_contract_are_not_both_accepted() {
    // The grammar tolerates these; the canonical comparison does not (NC15's discipline).
    for variant in [
        GOOD.replace("name vos", "name  vos"),
        GOOD.replace("iface sched\n", "iface sched \n"),
        format!("{GOOD}\n"),
        GOOD.replace("config N 8", "config N  8"),
    ] {
        assert_eq!(kind(&variant), K::NotCanonical, "{variant:?}");
    }
    // A blank line between rows is refused earlier, by the grammar itself.
    assert_eq!(kind(&GOOD.replace("tier any\n", "tier any\n\n")), K::Syntax);
    let e = parse_contract(GOOD.replace("name vos", "name  vos").as_bytes()).unwrap_err();
    assert_eq!(e.line, 2, "the line of the first difference");
}

#[test]
fn errors_display_with_their_line() {
    let e = parse_contract(b"vos-contract 2\nname a\niface b\nend\n").unwrap_err();
    assert_eq!(e.to_string(), "contract.vos:1: the version is 1");
}
