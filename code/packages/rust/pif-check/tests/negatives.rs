//! Negative corpus for the CTE parser and the printed-syntax parser: every malformed input is an
//! error, never a panic and never a silent acceptance.

use pif_check::cte::{parse_name, parse_name_list, CteError, Dterm};
use pif_check::ppparse::{parse_term, PpError};

#[test]
fn the_cte_parser_refuses_these_names_and_numbers() {
    // More than 32 components.
    let many = format!("C<{}>[]", vec!["\"a\""; 33].join(","));
    assert_eq!(
        Dterm::parse(&many)
            .err()
            .map(|e| matches!(e, CteError::Syntax("name too long", _))),
        Some(true)
    );
    // A name of more than 4096 decoded bytes.
    let long = format!("C<\"{}\">[]", "a".repeat(4097));
    assert!(Dterm::parse(&long).is_err());
    // A nat literal of more than 10,000 digits.
    let big = format!("N{}", "9".repeat(10_001));
    assert!(matches!(
        Dterm::parse(&big),
        Err(CteError::Syntax("nat literal too long", _))
    ));
    assert!(Dterm::parse(&format!("N{}", "9".repeat(10_000))).is_ok());
    // Separators.
    assert!(Dterm::parse("C<\"a\"\"b\">[]").is_err());
    assert!(Dterm::parse("C<\"a\">[zz]").is_err());
    assert!(Dterm::parse("C<\"a\">[z;z]").is_err());
    // Stack discipline: arities that do not add up cannot be written as text.
    for bad in [
        "Pd(<>,v0)",
        "Zd",
        "Ld(<>,v0,v0",
        "A(v0)",
        "S",
        "Sm(z)",
        "Ss()",
        "J(<>,0)",
        "Ld<>",
    ] {
        assert!(Dterm::parse(bad).is_err(), "{bad}");
    }
    // Names on their own.
    assert!(parse_name("<\"a\">").is_ok());
    assert!(parse_name("<\"a\">,<\"b\">").is_err()); // one name expected
    assert!(parse_name("<\"a\">x").is_err());
    assert_eq!(parse_name_list("<\"a\">,<\"b\">,<>").unwrap().len(), 3);
    assert!(parse_name_list("<\"a\">,").is_err());
    assert!(parse_name_list("").is_err());
    assert!(parse_name_list("<1,2>").is_ok());
    assert!(parse_name_list("<01>").is_err());
}

#[test]
fn the_printed_syntax_parser_refuses_these() {
    for bad in [
        "forall (a : T)",
        "forall (a : T) , ",
        "forall ( : T), T",
        "fun {a : T} => ",
        "let a : T := v;",
        "let a := v; b",
        "f (",
        "f )",
        "f.{1,}",
        "f.{,1}",
        "Sort max(1 2)",
        "Sort max(",
        "Sort imax(1,2",
        "Sort (1)",
        "T -> -> T",
        "{a : T}",
        "forall [a : T}, T",
        "forall {|a : T}, a",
        "forall (a : T] a",
        "#",
        "#-1",
        "a b #",
        "\"s\"",
        "Sort \"s\"",
    ] {
        assert!(parse_term(bad).is_err(), "{bad:?}");
    }
    // The error variants carry a position.
    assert!(matches!(parse_term("f )"), Err(PpError::Syntax(..))));
    // A very long flat text is refused before it is scanned into tokens.
    let huge = "a ".repeat(9 * 1024 * 1024);
    assert_eq!(parse_term(&huge).err(), Some(PpError::TooLarge));
}
