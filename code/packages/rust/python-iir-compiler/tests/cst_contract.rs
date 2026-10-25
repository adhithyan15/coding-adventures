//! LANG84 C8: the CST contract per release.
//!
//! The release-path compiler dispatches on rule names (`arith`, `factor`, …)
//! and effective token types (`INT`, `FLOOR_DIV`, …). A release whose grammar
//! renamed one of them would strand the compiler silently: the program would
//! just stop matching. So for every release that claims an IIR subset, every
//! name the compiler dispatches on must exist in that release's compiled
//! tables.

use std::collections::BTreeSet;

use coding_adventures_python_lexer::releases::token_grammar;
use coding_adventures_python_parser::release_parser_grammar;
use grammar_tools::parser_grammar::GrammarElement;
use python_iir_compiler::profiles::profile_for;
use python_iir_compiler::profiles::PROFILE_RULES;
use python_iir_compiler::{supported_releases, DISPATCHED_RULES, DISPATCHED_TOKENS};

/// Every token name a grammar element references.
fn referenced_tokens(element: &GrammarElement, out: &mut BTreeSet<String>) {
    match element {
        GrammarElement::TokenReference { name } => {
            out.insert(name.clone());
        }
        // `@asi(T, C…)` names the token it inserts and its closers.
        GrammarElement::AutoInsert { token, closers } => {
            out.insert(token.clone());
            out.extend(closers.iter().cloned());
        }
        GrammarElement::Sequence { elements } | GrammarElement::Alternation { choices: elements } => {
            for inner in elements {
                referenced_tokens(inner, out);
            }
        }
        GrammarElement::Repetition { element }
        | GrammarElement::Optional { element }
        | GrammarElement::Group { element }
        | GrammarElement::OneOrMore { element }
        | GrammarElement::NegativeLookahead { element }
        | GrammarElement::PositiveLookahead { element } => referenced_tokens(element, out),
        GrammarElement::SeparatedRepetition { element, separator, .. } => {
            referenced_tokens(element, out);
            referenced_tokens(separator, out);
        }
        // An adjacency predicate (`@no_space_before` …) tests the trivia
        // between tokens and names no token.
        GrammarElement::RuleReference { .. }
        | GrammarElement::Literal { .. }
        | GrammarElement::AdjacencyPredicate { .. } => {}
    }
}

#[test]
fn every_dispatched_name_exists_in_every_claiming_release() {
    let claiming: Vec<_> = supported_releases().iter().filter(|r| profile_for(r).is_some()).collect();
    assert_eq!(claiming.len(), 7);
    for release in claiming {
        let grammar = release_parser_grammar(release).unwrap();
        let rules: BTreeSet<_> = grammar.rules.iter().map(|r| r.name.as_str()).collect();
        for rule in DISPATCHED_RULES {
            assert!(rules.contains(rule), "{release}: the grammar has no rule `{rule}`");
        }
        // A rule one profile adds (Python 2's `print_stmt`) exists in every
        // release of that profile and in no other: the compiler chooses the
        // statement form by this shape, so it must not appear by accident.
        let profile = profile_for(release).unwrap().name;
        for (owner, extra) in PROFILE_RULES {
            for rule in *extra {
                assert_eq!(rules.contains(rule), *owner == profile, "{release} ({profile}): rule `{rule}`");
            }
        }
        let mut referenced = BTreeSet::new();
        for rule in &grammar.rules {
            referenced_tokens(&rule.body, &mut referenced);
        }
        let tokens = token_grammar(release).unwrap();
        let defined: BTreeSet<_> = tokens
            .definitions
            .iter()
            .map(|d| d.alias.clone().unwrap_or_else(|| d.name.clone()))
            .collect();
        for token in DISPATCHED_TOKENS {
            assert!(referenced.contains(*token), "{release}: the grammar never references {token}");
            // NEWLINE is the lexer engine's built-in token, not a definition.
            assert!(
                *token == "NEWLINE" || defined.contains(*token),
                "{release}: the .tokens file defines no {token}"
            );
        }
    }
}
