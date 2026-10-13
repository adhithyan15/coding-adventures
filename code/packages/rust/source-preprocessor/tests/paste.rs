//! The generic engine must reject a projected overspend before calling a hook.
//! A counted test dialect makes that ordering observable without allocating a
//! large token. Real C spelling and rooted-input behavior live in its frontend.

use coding_adventures_source_preprocessor::{
    bounds::{Bounds, Spend},
    dialect::{Dialect, Directive},
    fs::MemoryFs,
    hideset::HideSets,
    macros::{expand_with_dialect, MToken, MacroDef, MacroTable},
    source_map::{FileId, Position, SourceMap},
    PpError,
};
use lexer::token::{Token, TokenType};
use std::cell::Cell;

fn token(value: &str) -> Token {
    Token {
        type_: TokenType::Name,
        value: value.into(),
        line: 1,
        column: 1,
        type_name: None,
        flags: None,
        cv: None,
    }
}

#[derive(Default)]
struct CountedPaste {
    calls: Cell<u32>,
    result: Option<Token>,
    decline: bool,
}

impl Dialect for CountedPaste {
    fn classify(&self, _: &[Token]) -> Option<Result<Directive, PpError>> {
        None
    }
    fn eval_condition(&self, _: &[Token]) -> Result<bool, PpError> {
        Ok(false)
    }
    fn lex(&self, _: &str, _: FileId) -> Result<Vec<Token>, PpError> {
        Ok(Vec::new())
    }
    fn paste(&self, left: &Token, right: &Token) -> Option<Token> {
        self.calls.set(self.calls.get() + 1);
        if self.decline {
            return None;
        }
        Some(
            self.result
                .clone()
                .unwrap_or_else(|| token(&format!("{}{}", left.value, right.value))),
        )
    }
}

fn expand(
    body: &[&str],
    bounds: Bounds,
    spend: &mut Spend,
    dialect: &CountedPaste,
) -> Result<Vec<MToken>, PpError> {
    let mut fs = MemoryFs::new();
    let file = fs.insert("<main>", "");
    let mut table = MacroTable::new();
    table.define(
        "P",
        MacroDef {
            params: Some(vec!["x".into()]),
            body: body.iter().map(|s| token(s)).collect(),
            defined_at: Position {
                file,
                line: 1,
                column: 1,
            },
        },
    );
    let input = ["P", "(", "NAME", ")"]
        .iter()
        .map(|s| MToken::bare(token(s), file))
        .collect();
    expand_with_dialect(
        input,
        &table,
        &mut HideSets::new(),
        &mut SourceMap::new(),
        &bounds,
        spend,
        dialect,
    )
}

#[test]
fn prefix_and_suffix_charge_exact_concatenation_before_hook() {
    for body in [["pre", "##", "x"], ["x", "##", "post"]] {
        let bytes = if body[0] == "x" { 8 } else { 7 };
        for bounds in [
            Bounds {
                token_spelling_bytes: bytes - 1,
                ..Bounds::default()
            },
            Bounds {
                synthesised_text_bytes: bytes - 1,
                ..Bounds::default()
            },
            Bounds {
                tokens_produced: 0,
                ..Bounds::default()
            },
        ] {
            let dialect = CountedPaste::default();
            assert!(expand(&body, bounds, &mut Spend::default(), &dialect).is_err());
            assert_eq!(dialect.calls.get(), 0, "precharge must precede the hook");
        }
        let dialect = CountedPaste::default();
        let mut spend = Spend::default();
        let output = expand(
            &body,
            Bounds {
                macro_depth: 0,
                token_spelling_bytes: bytes,
                synthesised_text_bytes: bytes,
                tokens_produced: 1,
                ..Bounds::default()
            },
            &mut spend,
            &dialect,
        )
        .unwrap();
        assert_eq!(
            output[0].token.value,
            if body[0] == "x" {
                "NAMEpost"
            } else {
                "preNAME"
            }
        );
        assert_eq!(spend.synthesised_text_bytes, bytes);
        assert_eq!(spend.tokens_produced, 1);
        assert_eq!(dialect.calls.get(), 1);
    }
}

#[test]
fn suffix_aggregate_budget_counts_earlier_substitutions() {
    let dialect = CountedPaste::default();
    let mut spend = Spend {
        synthesised_text_bytes: 1,
        ..Spend::default()
    };
    assert!(expand(
        &["x", "##", "post"],
        Bounds {
            synthesised_text_bytes: 8,
            ..Bounds::default()
        },
        &mut spend,
        &dialect
    )
    .is_err());
    assert_eq!(dialect.calls.get(), 0);
}

#[test]
fn suffix_rejects_declined_and_malformed_hook_results() {
    let mut wrong_type = token("NAMEpost");
    wrong_type.type_ = TokenType::String;
    for result in [
        token("postNAME"),
        token("NAMEpostextra"),
        token("NAMExxxx"),
        wrong_type,
    ] {
        let dialect = CountedPaste {
            result: Some(result),
            ..CountedPaste::default()
        };
        assert!(expand(
            &["x", "##", "post"],
            Bounds::default(),
            &mut Spend::default(),
            &dialect
        )
        .is_err());
        assert_eq!(dialect.calls.get(), 1);
    }
    let dialect = CountedPaste {
        decline: true,
        ..CountedPaste::default()
    };
    assert!(expand(
        &["x", "##", "post"],
        Bounds::default(),
        &mut Spend::default(),
        &dialect
    )
    .is_err());
}

#[test]
fn generic_paste_requires_exactly_one_parameter_operand() {
    for body in [
        ["x", "##", "x"],
        ["left", "##", "right"],
        ["##", "x", "post"],
    ] {
        let dialect = CountedPaste::default();
        assert!(expand(&body, Bounds::default(), &mut Spend::default(), &dialect).is_err());
        assert_eq!(dialect.calls.get(), 0);
    }
}
