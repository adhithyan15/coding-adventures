//! # Parser — token stream → AST.
//!
//! Hand-written recursive descent. Each grammar production maps to
//! one function; lookahead is at most one token via [`Parser::peek`].
//!
//! ## Grammar (v0.1)
//!
//! ```text
//! program     := statement*
//! statement   := prior_decl | contrib_decl | interact_decl
//!              | observe_decl | query_decl
//!
//! prior_decl  := 'prior' NUMBER 'for' term annotation*
//! contrib_decl:= 'contributes' NUMBER 'from' term 'to' term annotation*
//! interact_decl := 'interacts' NUMBER 'when' term ('and' term)+ 'for' term annotation*
//! observe_decl:= 'observe' term
//! query_decl  := '?' term
//!
//! annotation  := 'source' STRING
//!              | 'locator' STRING
//!              | 'trust' trust_tier
//!
//! trust_tier  := 'consensus' | 'authoritative' | 'empirical'
//!              | 'inferred' | 'unattributed'
//!
//! term        := IDENT
//!              | IDENT '(' term (',' term)* ')'
//! ```

use crate::ast::{Annotation, Program, Statement, Term, TrustTierName};
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// We expected one of `expected` but found `found` at the given
    /// source position. Includes a string description of the
    /// production context so the error message points the reader at
    /// the right grammar rule.
    Expected {
        expected: String,
        found: String,
        line: usize,
        col: usize,
    },
    /// `interacts` requires at least two evidence terms joined by
    /// `and`. With one or zero, the modeller probably meant
    /// `contributes` — flag explicitly so the diagnostic is helpful.
    InteractNeedsAtLeastTwoEvidence { line: usize, col: usize },
    /// A compound term was nested deeper than [`MAX_TERM_DEPTH`].
    ///
    /// This bound exists to prevent untrusted input from producing
    /// stack-overflow DoS via `f(f(f(...)))`-style nesting. Real
    /// rulebook terms in clinical / legal / financial domains never
    /// approach this depth — single-argument `pmh(hypertension)`
    /// covers the bulk of real usage; depths above ~10 indicate a
    /// modeller error or an adversarial input.
    TooDeeplyNested { depth: usize, line: usize, col: usize },
    UnexpectedEof,
}

/// Maximum depth of nested compound terms the parser accepts.
///
/// Chosen as a sane upper bound: rulebooks in practice nest 1-2
/// levels (`pmh(hypertension)`, occasionally
/// `relation(a, compound(b, c))`); 256 leaves three orders of
/// magnitude of headroom before any plausible legitimate input is
/// rejected, while bounding adversarial recursion well within a
/// typical 2-8 MB stack budget. Each `parse_term` frame is a few
/// hundred bytes; 256 × ~512 bytes ≈ 128 KB.
pub const MAX_TERM_DEPTH: usize = 256;

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    /// Current nesting depth inside `parse_term`. Tracked to bound
    /// recursion at [`MAX_TERM_DEPTH`] so adversarial input cannot
    /// stack-overflow the process.
    term_depth: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            term_depth: 0,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, kind_match: impl Fn(&TokenKind) -> bool, expected: &str)
        -> Result<&Token, ParseError>
    {
        let t = self.peek();
        if kind_match(&t.kind) {
            Ok(self.advance())
        } else {
            Err(ParseError::Expected {
                expected: expected.into(),
                found: format!("{:?}", t.kind),
                line: t.line,
                col: t.col,
            })
        }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while !self.at_eof() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.peek().kind {
            TokenKind::KwPrior => self.parse_prior(),
            TokenKind::KwContributes => self.parse_contributes(),
            TokenKind::KwInteracts => self.parse_interacts(),
            TokenKind::KwObserve => self.parse_observe(),
            TokenKind::Question => self.parse_query(),
            other => {
                let t = self.peek();
                Err(ParseError::Expected {
                    expected: "statement keyword (prior, contributes, interacts, observe, ?)"
                        .into(),
                    found: format!("{other:?}"),
                    line: t.line,
                    col: t.col,
                })
            }
        }
    }

    fn parse_prior(&mut self) -> Result<Statement, ParseError> {
        self.expect(|k| matches!(k, TokenKind::KwPrior), "`prior`")?;
        let probability = self.parse_number()?;
        self.expect(|k| matches!(k, TokenKind::KwFor), "`for`")?;
        let conclusion = self.parse_term()?;
        let annotations = self.parse_annotations()?;
        Ok(Statement::Prior {
            probability,
            conclusion,
            annotations,
        })
    }

    fn parse_contributes(&mut self) -> Result<Statement, ParseError> {
        self.expect(|k| matches!(k, TokenKind::KwContributes), "`contributes`")?;
        let lr = self.parse_number()?;
        self.expect(|k| matches!(k, TokenKind::KwFrom), "`from`")?;
        let evidence = self.parse_term()?;
        self.expect(|k| matches!(k, TokenKind::KwTo), "`to`")?;
        let conclusion = self.parse_term()?;
        let annotations = self.parse_annotations()?;
        Ok(Statement::Contributes {
            lr,
            evidence,
            conclusion,
            annotations,
        })
    }

    fn parse_interacts(&mut self) -> Result<Statement, ParseError> {
        let interact_tok = self.expect(|k| matches!(k, TokenKind::KwInteracts), "`interacts`")?;
        let line = interact_tok.line;
        let col = interact_tok.col;
        let lr = self.parse_number()?;
        self.expect(|k| matches!(k, TokenKind::KwWhen), "`when`")?;
        let mut evidence_set = vec![self.parse_term()?];
        while matches!(self.peek().kind, TokenKind::KwAnd) {
            self.advance();
            evidence_set.push(self.parse_term()?);
        }
        if evidence_set.len() < 2 {
            return Err(ParseError::InteractNeedsAtLeastTwoEvidence { line, col });
        }
        self.expect(|k| matches!(k, TokenKind::KwFor), "`for`")?;
        let conclusion = self.parse_term()?;
        let annotations = self.parse_annotations()?;
        Ok(Statement::Interacts {
            lr,
            evidence_set,
            conclusion,
            annotations,
        })
    }

    fn parse_observe(&mut self) -> Result<Statement, ParseError> {
        self.expect(|k| matches!(k, TokenKind::KwObserve), "`observe`")?;
        let term = self.parse_term()?;
        Ok(Statement::Observe { term })
    }

    fn parse_query(&mut self) -> Result<Statement, ParseError> {
        self.expect(|k| matches!(k, TokenKind::Question), "`?`")?;
        let conclusion = self.parse_term()?;
        Ok(Statement::Query { conclusion })
    }

    fn parse_number(&mut self) -> Result<f64, ParseError> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(n)
            }
            other => Err(ParseError::Expected {
                expected: "number".into(),
                found: format!("{other:?}"),
                line: t.line,
                col: t.col,
            }),
        }
    }

    fn parse_term(&mut self) -> Result<Term, ParseError> {
        // Depth guard: prevent adversarial nested compounds
        // (`f(f(f(...)))`) from overflowing the stack. Increment on
        // entry, decrement on every exit path (both Ok and Err). The
        // manual increment/decrement avoids a Drop-based RAII guard
        // because we need access to `&mut self` for the recursive
        // call anyway.
        self.term_depth += 1;
        if self.term_depth > MAX_TERM_DEPTH {
            let t = self.peek();
            let err = ParseError::TooDeeplyNested {
                depth: self.term_depth,
                line: t.line,
                col: t.col,
            };
            self.term_depth -= 1;
            return Err(err);
        }
        let result = self.parse_term_inner();
        self.term_depth -= 1;
        result
    }

    /// Body of `parse_term` — separated so the entry/exit depth
    /// management can wrap a single `?`-using inner function.
    fn parse_term_inner(&mut self) -> Result<Term, ParseError> {
        let t = self.peek().clone();
        let name = match t.kind {
            TokenKind::Ident(ref s) => s.clone(),
            other => {
                return Err(ParseError::Expected {
                    expected: "term identifier".into(),
                    found: format!("{other:?}"),
                    line: t.line,
                    col: t.col,
                });
            }
        };
        self.advance();
        if matches!(self.peek().kind, TokenKind::LParen) {
            self.advance();
            let mut args = vec![self.parse_term()?];
            while matches!(self.peek().kind, TokenKind::Comma) {
                self.advance();
                args.push(self.parse_term()?);
            }
            self.expect(|k| matches!(k, TokenKind::RParen), "`)`")?;
            Ok(Term::Compound { functor: name, args })
        } else {
            Ok(Term::Atom(name))
        }
    }

    fn parse_annotations(&mut self) -> Result<Vec<Annotation>, ParseError> {
        let mut out = Vec::new();
        loop {
            match &self.peek().kind {
                TokenKind::KwSource => {
                    self.advance();
                    let s = self.parse_string()?;
                    out.push(Annotation::Source(s));
                }
                TokenKind::KwLocator => {
                    self.advance();
                    let s = self.parse_string()?;
                    out.push(Annotation::Locator(s));
                }
                TokenKind::KwTrust => {
                    self.advance();
                    let tier = self.parse_trust_tier()?;
                    out.push(Annotation::Trust(tier));
                }
                _ => break,
            }
        }
        Ok(out)
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::String(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(ParseError::Expected {
                expected: "string literal".into(),
                found: format!("{other:?}"),
                line: t.line,
                col: t.col,
            }),
        }
    }

    fn parse_trust_tier(&mut self) -> Result<TrustTierName, ParseError> {
        let t = self.peek().clone();
        let tier = match t.kind {
            TokenKind::KwConsensus => TrustTierName::Consensus,
            TokenKind::KwAuthoritative => TrustTierName::Authoritative,
            TokenKind::KwEmpirical => TrustTierName::Empirical,
            TokenKind::KwInferred => TrustTierName::Inferred,
            TokenKind::KwUnattributed => TrustTierName::Unattributed,
            other => {
                return Err(ParseError::Expected {
                    expected: "trust tier (consensus / authoritative / empirical / inferred / unattributed)"
                        .into(),
                    found: format!("{other:?}"),
                    line: t.line,
                    col: t.col,
                });
            }
        };
        self.advance();
        Ok(tier)
    }
}

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedEof);
    }
    Parser::new(tokens).parse_program()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_src(src: &str) -> Result<Program, ParseError> {
        let tokens = lex(src).unwrap();
        parse(&tokens)
    }

    #[test]
    fn parses_minimal_prior() {
        let p = parse_src("prior 0.10 for acs").unwrap();
        assert_eq!(p.statements.len(), 1);
        assert!(matches!(
            p.statements[0],
            Statement::Prior { probability, .. } if probability == 0.10
        ));
    }

    #[test]
    fn parses_contributes_with_annotations() {
        let src = r#"
            contributes 1.5 from pmh(hypertension) to acs
              source "HEART Score"
              trust empirical
        "#;
        let p = parse_src(src).unwrap();
        match &p.statements[0] {
            Statement::Contributes { lr, evidence, conclusion, annotations } => {
                assert_eq!(*lr, 1.5);
                assert!(matches!(evidence, Term::Compound { functor, .. } if functor == "pmh"));
                assert!(matches!(conclusion, Term::Atom(n) if n == "acs"));
                assert_eq!(annotations.len(), 2);
                assert!(matches!(annotations[0], Annotation::Source(_)));
                assert!(matches!(annotations[1], Annotation::Trust(TrustTierName::Empirical)));
            }
            other => panic!("expected Contributes, got {other:?}"),
        }
    }

    #[test]
    fn parses_interacts_with_two_evidence_terms() {
        let src = "interacts 1.3 when symptom_quality(pressure_like) and associated_symptom(diaphoresis) for acs";
        let p = parse_src(src).unwrap();
        match &p.statements[0] {
            Statement::Interacts { evidence_set, .. } => {
                assert_eq!(evidence_set.len(), 2);
            }
            other => panic!("expected Interacts, got {other:?}"),
        }
    }

    #[test]
    fn interacts_with_single_evidence_is_an_error() {
        let src = "interacts 1.3 when pmh(htn) for acs";
        let err = parse_src(src).unwrap_err();
        assert!(matches!(
            err,
            ParseError::InteractNeedsAtLeastTwoEvidence { .. }
        ));
    }

    #[test]
    fn parses_observe_and_query() {
        let src = "observe pmh(hypertension)\n? acs";
        let p = parse_src(src).unwrap();
        assert!(matches!(p.statements[0], Statement::Observe { .. }));
        assert!(matches!(p.statements[1], Statement::Query { .. }));
    }

    #[test]
    fn parses_multi_arg_compound_terms() {
        let src = "observe relation(a, b, c)";
        let p = parse_src(src).unwrap();
        match &p.statements[0] {
            Statement::Observe { term: Term::Compound { args, .. } } => {
                assert_eq!(args.len(), 3);
            }
            other => panic!("expected compound term, got {other:?}"),
        }
    }

    #[test]
    fn parses_full_acs_rulebook() {
        // The smoke test for the language: the ACS rulebook from
        // ADJ36 parses cleanly.
        let src = r#"
            prior 0.10 for acs
              source "Pope JH et al., NEJM 1995"

            contributes 1.5 from pmh(hypertension) to acs
              source "HEART Score; Six 2008"
              trust empirical

            contributes 1.8 from pmh(smoker) to acs
              source "HEART Score; Six 2008"

            contributes 2.5 from symptom_quality(pressure_like) to acs
              source "Panju AA et al., JAMA 1998"

            contributes 2.0 from associated_symptom(diaphoresis) to acs
              source "Panju AA et al., JAMA 1998"

            contributes 0.5 from vital_signs(within_normal_limits) to acs
              source "Panju 1998"

            contributes 0.4 from denied(ecg_acute_st_changes) to acs
              source "Pope 1995"

            interacts 1.3 when symptom_quality(pressure_like)
                           and associated_symptom(diaphoresis)
                           for acs
              source "[empirical] synergy"

            observe pmh(hypertension)
            observe pmh(smoker)
            observe symptom_quality(pressure_like)
            observe associated_symptom(diaphoresis)
            observe vital_signs(within_normal_limits)
            observe denied(ecg_acute_st_changes)

            ? acs
        "#;
        let p = parse_src(src).unwrap();
        // 1 prior + 6 contributes + 1 interacts + 6 observes + 1 query = 15
        assert_eq!(p.statements.len(), 15);
    }

    #[test]
    fn deeply_nested_compound_term_is_rejected_with_too_deeply_nested_error() {
        // Build a synthetic source with MAX_TERM_DEPTH + 10 layers
        // of f(f(...)) nesting. The parser should reject it
        // gracefully instead of stack-overflowing.
        let depth = MAX_TERM_DEPTH + 10;
        let mut src = String::from("observe ");
        for _ in 0..depth {
            src.push_str("f(");
        }
        src.push('x');
        for _ in 0..depth {
            src.push(')');
        }
        let err = parse_src(&src).unwrap_err();
        assert!(
            matches!(err, ParseError::TooDeeplyNested { .. }),
            "expected TooDeeplyNested, got {err:?}"
        );
    }

    #[test]
    fn nesting_up_to_the_limit_is_still_accepted() {
        // Just under the limit must still parse.
        let depth = MAX_TERM_DEPTH - 1;
        let mut src = String::from("observe ");
        for _ in 0..depth {
            src.push_str("f(");
        }
        src.push('x');
        for _ in 0..depth {
            src.push(')');
        }
        assert!(parse_src(&src).is_ok());
    }

    #[test]
    fn missing_for_after_prior_is_an_error() {
        let err = parse_src("prior 0.10 acs").unwrap_err();
        match err {
            ParseError::Expected { expected, .. } => {
                assert!(expected.contains("for"));
            }
            other => panic!("expected Expected(for), got {other:?}"),
        }
    }
}
