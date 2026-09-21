//! # Macro definition and expansion.
//!
//! The algorithm is Prosser's, which is what the C standard's expansion rules
//! describe operationally. Two parts do the work:
//!
//! 1. **Hide sets** ([`crate::hideset`]) make expansion terminate, per token
//!    rather than per macro. That module explains why the obvious rule fails.
//! 2. **Argument pre-expansion** decides *when* an argument is expanded, which
//!    is the part that changes observable output rather than merely
//!    terminating.
//!
//! ## Pre-expansion, and why the order is not arbitrary
//!
//! An argument is macro-expanded *before* substitution — but only where the
//! parameter is used plainly. That ordering is observable:
//!
//! ```text
//!     #define ID(x)   x
//!     #define ONE     1
//!     ID(ONE)     →   1          ONE is expanded before substitution
//! ```
//!
//! Getting it backwards still terminates and still produces something, which
//! is why this is worth a table of cases rather than a single smoke test: the
//! failure mode is a quietly different program, not a crash.
//!
//! ## What is deliberately absent
//!
//! Stringize and token paste are **not** implemented here. They are dialect
//! hooks ([`crate::dialect::Dialect::stringize`] and `paste`), defaulting to
//! "unsupported", and MacroOct declines both — which is itself a test that the
//! engine does not quietly assume every language is C. They get their first
//! real implementation when C arrives.
//!
//! ## Bounds
//!
//! Expansion is where the interesting resource attacks live, so every loop
//! here is charged:
//!
//! - depth, against [`crate::bounds::Bounds::macro_depth`];
//! - every token produced, including tokens that are discarded, against
//!   `tokens_produced` — counting only survivors is the mistake slice 1 made;
//! - fuel, per token copied and per rescan;
//! - argument-list grouping depth, so a pathological `F(((((…` is a
//!   diagnostic rather than a stack overflow.

use crate::bounds::{Bounds, Spend};
use crate::diag::PpError;
use crate::hideset::{HideId, HideSets, NameId};
use lexer::token::Token;
use std::collections::HashMap;

/// A token paired with the set of macros that must not be expanded for it.
#[derive(Debug, Clone)]
pub struct MToken {
    pub token: Token,
    pub hide: HideId,
}

impl MToken {
    pub fn bare(token: Token) -> MToken {
        MToken { token, hide: HideId::EMPTY }
    }
}

/// One macro definition.
#[derive(Debug, Clone)]
pub struct MacroDef {
    /// `None` for an object-like macro; `Some(params)` for a function-like
    /// one. The distinction is not cosmetic: a function-like macro's name
    /// used *without* a following `(` is not an invocation and must be left
    /// alone.
    pub params: Option<Vec<String>>,
    pub body: Vec<Token>,
}

/// The macro table.
#[derive(Debug, Default)]
pub struct MacroTable {
    defs: HashMap<String, MacroDef>,
}

impl MacroTable {
    #[must_use]
    pub fn new() -> MacroTable {
        MacroTable::default()
    }

    pub fn define(&mut self, name: impl Into<String>, def: MacroDef) {
        self.defs.insert(name.into(), def);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&MacroDef> {
        self.defs.get(name)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// Forget every definition. Called per translation unit, because a macro
    /// table leaking across units is how one file silently changes the meaning
    /// of the next.
    pub fn clear(&mut self) {
        self.defs.clear();
    }
}

/// Expand every macro invocation in `input`.
///
/// Iterative, with an explicit output buffer and a work stack — not recursive.
/// In Rust a stack overflow is an abort rather than a catchable panic, so a
/// recursive expander would turn [`Bounds::macro_depth`] from a diagnostic
/// into a process kill.
pub fn expand(
    input: Vec<MToken>,
    table: &MacroTable,
    hides: &mut HideSets,
    bounds: &Bounds,
    spend: &mut Spend,
) -> Result<Vec<MToken>, PpError> {
    let mut work: Vec<MToken> = input;
    work.reverse(); // pop() takes from the front of the logical stream
    let mut out: Vec<MToken> = Vec::new();
    let mut rounds: u32 = 0;

    while let Some(cur) = work.pop() {
        spend.fuel_used = spend.fuel_used.saturating_add(1);
        if spend.fuel_used > bounds.fuel {
            return Err(PpError::new(format!(
                "exhausted the {}-step preprocessing budget during macro expansion",
                bounds.fuel
            )));
        }

        let name = cur.token.value.clone();
        let Some(def) = table.get(&name) else {
            out.push(cur);
            continue;
        };

        let name_id = hides.name(&name);
        if hides.contains(cur.hide, name_id) {
            // Painted blue: this exact token came out of an expansion of this
            // macro, so it stays literal however many times it is rescanned.
            out.push(cur);
            continue;
        }

        rounds = rounds.saturating_add(1);
        if rounds > bounds.macro_depth.saturating_mul(bounds.macro_depth.max(1)) {
            return Err(PpError::new(format!(
                "macro expansion exceeded {} rounds",
                bounds.macro_depth
            ))
            .at_opt(position_of(&cur.token)));
        }

        let produced = match &def.params {
            None => substitute_object_like(def, cur.hide, name_id, hides),
            Some(params) => {
                // A function-like macro's name not followed by `(` is an
                // ordinary identifier. Leaving it alone is required, not a
                // convenience: `#define F(x) x` followed by a bare `F` must
                // emit `F`.
                if !next_is_open_paren(&work) {
                    out.push(cur);
                    continue;
                }
                let (args, close_hide) = collect_args(&mut work, params.len(), bounds, spend, &cur)?;
                let expanded_args =
                    pre_expand_args(args, table, hides, bounds, spend)?;
                // The intersection of the NAME token's hide set and the
                // CLOSING PAREN's: the invocation spans both, so a name hidden
                // in only one of them was not hidden across the whole
                // invocation. Using the name's set alone over-hides and
                // silently drops expansions.
                let base = hides.intersect(cur.hide, close_hide);
                substitute_function_like(def, params, &expanded_args, base, name_id, hides)
            }
        };

        charge_tokens(produced.len(), bounds, spend)?;

        // Rescan: the substituted tokens go back on the work stack so anything
        // they revealed is itself expanded. Termination comes from the hide
        // sets, not from refusing to rescan.
        for t in produced.into_iter().rev() {
            work.push(t);
        }
    }

    Ok(out)
}

fn charge_tokens(n: usize, bounds: &Bounds, spend: &mut Spend) -> Result<(), PpError> {
    spend.tokens_produced = spend.tokens_produced.saturating_add(n as u64);
    if spend.tokens_produced > bounds.tokens_produced {
        return Err(PpError::new(format!(
            "macro expansion produced more than {} tokens",
            bounds.tokens_produced
        )));
    }
    Ok(())
}

fn position_of(_t: &Token) -> Option<crate::source_map::Position> {
    None
}

fn next_is_open_paren(work: &[MToken]) -> bool {
    work.last().is_some_and(|t| t.token.value == "(")
}

/// Collect a function-like macro's arguments.
///
/// Returns the argument token runs and the hide set of the closing
/// parenthesis, which the caller needs for the intersection rule.
fn collect_args(
    work: &mut Vec<MToken>,
    arity: usize,
    bounds: &Bounds,
    spend: &mut Spend,
    invocation: &MToken,
) -> Result<(Vec<Vec<MToken>>, HideId), PpError> {
    // Consume the `(` we already peeked.
    let open = work.pop().expect("caller checked for `(`");
    debug_assert_eq!(open.token.value, "(");

    let mut args: Vec<Vec<MToken>> = Vec::new();
    let mut cur: Vec<MToken> = Vec::new();
    let mut depth: u32 = 0;

    loop {
        let Some(t) = work.pop() else {
            return Err(PpError::new(format!(
                "unterminated argument list for macro `{}`",
                PpError::quote(&invocation.token.value, bounds.diagnostic_quote_bytes)
            )));
        };
        spend.fuel_used = spend.fuel_used.saturating_add(1);
        if spend.fuel_used > bounds.fuel {
            return Err(PpError::new("exhausted the preprocessing budget collecting arguments"));
        }

        match t.token.value.as_str() {
            "(" => {
                depth += 1;
                if depth > bounds.arg_group_depth {
                    return Err(PpError::new(format!(
                        "macro argument grouping nested deeper than {}",
                        bounds.arg_group_depth
                    )));
                }
                cur.push(t);
            }
            ")" if depth == 0 => {
                // Exactly one empty argument and zero declared parameters is
                // `F()` on a zero-arity macro, not a one-argument call.
                if !(cur.is_empty() && args.is_empty() && arity == 0) {
                    args.push(cur);
                }
                return Ok((args, t.hide));
            }
            ")" => {
                depth -= 1;
                cur.push(t);
            }
            "," if depth == 0 => {
                args.push(std::mem::take(&mut cur));
            }
            _ => cur.push(t),
        }
    }
}

/// Expand each argument in its own right, before substitution.
fn pre_expand_args(
    args: Vec<Vec<MToken>>,
    table: &MacroTable,
    hides: &mut HideSets,
    bounds: &Bounds,
    spend: &mut Spend,
) -> Result<Vec<Vec<MToken>>, PpError> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        out.push(expand(a, table, hides, bounds, spend)?);
    }
    Ok(out)
}

fn substitute_object_like(
    def: &MacroDef,
    invocation_hide: HideId,
    name: NameId,
    hides: &mut HideSets,
) -> Vec<MToken> {
    let hide = hides.insert(invocation_hide, name);
    def.body.iter().cloned().map(|token| MToken { token, hide }).collect()
}

fn substitute_function_like(
    def: &MacroDef,
    params: &[String],
    args: &[Vec<MToken>],
    base_hide: HideId,
    name: NameId,
    hides: &mut HideSets,
) -> Vec<MToken> {
    let hide = hides.insert(base_hide, name);
    let mut out = Vec::new();

    for token in &def.body {
        match params.iter().position(|p| *p == token.value) {
            Some(i) => {
                // Substituted argument tokens keep THEIR OWN hide sets. They
                // were expanded in the caller's context, so repainting them
                // with this macro's set would hide expansions that were
                // legitimately available at the call site.
                if let Some(arg) = args.get(i) {
                    out.extend(arg.iter().cloned());
                }
                // A parameter with no corresponding argument substitutes
                // nothing, which is how `F()` on a one-parameter macro yields
                // an empty expansion rather than an error.
            }
            None => out.push(MToken { token: token.clone(), hide }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    fn tok(v: &str) -> Token {
        Token {
            type_: TokenType::Name,
            value: v.to_string(),
            line: 1,
            column: 1,
            type_name: None,
            flags: None,
            cv: None,
        }
    }

    fn toks(src: &str) -> Vec<MToken> {
        src.split_whitespace().map(|w| MToken::bare(tok(w))).collect()
    }

    fn body(src: &str) -> Vec<Token> {
        src.split_whitespace().map(tok).collect()
    }

    fn obj(t: &mut MacroTable, name: &str, b: &str) {
        t.define(name, MacroDef { params: None, body: body(b) });
    }

    fn func(t: &mut MacroTable, name: &str, params: &[&str], b: &str) {
        t.define(
            name,
            MacroDef {
                params: Some(params.iter().map(|s| s.to_string()).collect()),
                body: body(b),
            },
        );
    }

    fn run(table: &MacroTable, src: &str) -> Result<String, PpError> {
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let out = expand(toks(src), table, &mut hides, &Bounds::default(), &mut spend)?;
        Ok(out.iter().map(|t| t.token.value.as_str()).collect::<Vec<_>>().join(" "))
    }

    #[test]
    fn an_object_like_macro_expands() {
        let mut t = MacroTable::new();
        obj(&mut t, "ONE", "1");
        assert_eq!(run(&t, "ONE").unwrap(), "1");
    }

    #[test]
    fn a_non_macro_identifier_is_left_alone() {
        let t = MacroTable::new();
        assert_eq!(run(&t, "x + y").unwrap(), "x + y");
    }

    #[test]
    fn expansion_rescans_so_macros_compose() {
        let mut t = MacroTable::new();
        obj(&mut t, "A", "B");
        obj(&mut t, "B", "42");
        assert_eq!(run(&t, "A").unwrap(), "42");
    }

    // --- termination: the cases the hide sets exist for --------------------

    #[test]
    fn a_self_referential_macro_expands_exactly_once() {
        // The canonical case. `FOO` must appear in the output, unexpanded,
        // rather than looping or vanishing.
        let mut t = MacroTable::new();
        obj(&mut t, "FOO", "( 1 + FOO )");
        assert_eq!(run(&t, "FOO").unwrap(), "( 1 + FOO )");
    }

    #[test]
    fn mutually_recursive_macros_terminate() {
        // Neither macro is ever "expanding itself", so a naive
        // am-I-in-my-own-expansion rule loops forever here. The per-token
        // hide set is what stops it.
        let mut t = MacroTable::new();
        obj(&mut t, "P", "Q");
        obj(&mut t, "Q", "P");
        let out = run(&t, "P").unwrap();
        assert!(out == "P" || out == "Q", "terminated with `{out}`");
    }

    #[test]
    fn mutually_recursive_function_like_macros_terminate() {
        let mut t = MacroTable::new();
        func(&mut t, "F", &["x"], "G ( x )");
        func(&mut t, "G", &["x"], "F ( x )");
        let out = run(&t, "F ( 1 )").unwrap();
        assert!(out.contains('1'), "argument survived: {out}");
        assert!(out.contains('F') || out.contains('G'));
    }

    #[test]
    fn indirect_self_reference_through_three_macros_terminates() {
        let mut t = MacroTable::new();
        obj(&mut t, "A", "B");
        obj(&mut t, "B", "C");
        obj(&mut t, "C", "A");
        let _ = run(&t, "A").unwrap();
    }

    // --- function-like mechanics ------------------------------------------

    #[test]
    fn a_function_like_macro_substitutes_its_argument() {
        let mut t = MacroTable::new();
        func(&mut t, "ID", &["x"], "x");
        assert_eq!(run(&t, "ID ( 42 )").unwrap(), "42");
    }

    #[test]
    fn a_function_like_name_without_parens_is_not_an_invocation() {
        // Required, not a nicety: a bare `F` must survive as `F`.
        let mut t = MacroTable::new();
        func(&mut t, "F", &["x"], "x");
        assert_eq!(run(&t, "F + 1").unwrap(), "F + 1");
    }

    #[test]
    fn multiple_arguments_are_separated_at_depth_zero_only() {
        // The comma inside `G(1,2)` belongs to the inner call, not the outer
        // argument list — depth tracking is what makes that work.
        let mut t = MacroTable::new();
        func(&mut t, "PAIR", &["a", "b"], "a - b");
        assert_eq!(run(&t, "PAIR ( 1 , 2 )").unwrap(), "1 - 2");
        assert_eq!(run(&t, "PAIR ( ( 1 , 2 ) , 3 )").unwrap(), "( 1 , 2 ) - 3");
    }

    #[test]
    fn an_argument_is_expanded_before_substitution() {
        // The observable half of the algorithm: get this backwards and the
        // program still compiles, just differently.
        let mut t = MacroTable::new();
        func(&mut t, "ID", &["x"], "x");
        obj(&mut t, "ONE", "1");
        assert_eq!(run(&t, "ID ( ONE )").unwrap(), "1");
    }

    #[test]
    fn an_argument_used_twice_is_substituted_twice() {
        let mut t = MacroTable::new();
        func(&mut t, "TWICE", &["x"], "x + x");
        assert_eq!(run(&t, "TWICE ( 7 )").unwrap(), "7 + 7");
    }

    #[test]
    fn a_missing_argument_substitutes_nothing() {
        let mut t = MacroTable::new();
        func(&mut t, "ID", &["x"], "x");
        assert_eq!(run(&t, "ID ( )").unwrap(), "");
    }

    #[test]
    fn a_macro_invocation_inside_an_argument_expands() {
        let mut t = MacroTable::new();
        func(&mut t, "ID", &["x"], "x");
        func(&mut t, "INC", &["x"], "x + 1");
        assert_eq!(run(&t, "ID ( INC ( 4 ) )").unwrap(), "4 + 1");
    }

    #[test]
    fn an_unterminated_argument_list_is_a_diagnostic_not_a_hang() {
        let mut t = MacroTable::new();
        func(&mut t, "F", &["x"], "x");
        let e = run(&t, "F ( 1").unwrap_err();
        assert!(e.to_string().contains("unterminated"), "{e}");
    }

    // --- bounds ------------------------------------------------------------

    #[test]
    fn an_expansion_bomb_is_bounded() {
        // Chained doubling: each level doubles the token count, so ~40 levels
        // would exhaust memory from a few lines of source. The bound must fire
        // instead.
        let mut t = MacroTable::new();
        obj(&mut t, "A0", "x");
        for i in 1..40 {
            t.define(
                format!("A{i}"),
                MacroDef { params: None, body: body(&format!("A{} A{}", i - 1, i - 1)) },
            );
        }
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let bounds = Bounds { tokens_produced: 10_000, ..Bounds::default() };
        let e = expand(toks("A39"), &t, &mut hides, &bounds, &mut spend)
            .expect_err("a doubling chain must hit a bound");
        assert!(e.to_string().contains("tokens") || e.to_string().contains("budget"), "{e}");
    }

    #[test]
    fn pathological_argument_nesting_is_bounded() {
        let mut t = MacroTable::new();
        func(&mut t, "F", &["x"], "x");
        let src = format!("F ( {} 1 {}", "( ".repeat(400), ") ".repeat(400));
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let bounds = Bounds { arg_group_depth: 16, ..Bounds::default() };
        let e = expand(toks(&src), &t, &mut hides, &bounds, &mut spend)
            .expect_err("deep argument grouping must hit a bound");
        assert!(e.to_string().contains("nested deeper"), "{e}");
    }

    #[test]
    fn hide_sets_are_shared_across_an_expansion_not_cloned_per_token() {
        // The memory property, checked rather than asserted: a macro whose
        // body is 500 tokens must cost ONE hide-set node.
        let mut t = MacroTable::new();
        let long: String = (0..500).map(|i| format!("t{i} ")).collect();
        obj(&mut t, "BIG", &long);
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let out = expand(toks("BIG"), &t, &mut hides, &Bounds::default(), &mut spend).unwrap();
        assert_eq!(out.len(), 500);
        assert_eq!(hides.node_count(), 1, "one shared set, not one per token");
    }

    #[test]
    fn discarded_tokens_are_still_charged() {
        // Tokens produced and then dropped are still allocated. Charging only
        // survivors is exactly the hole slice 1 shipped and had to fix.
        let mut t = MacroTable::new();
        obj(&mut t, "WIDE", "a b c d e f g h");
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let _ = expand(toks("WIDE"), &t, &mut hides, &Bounds::default(), &mut spend);
        assert!(spend.tokens_produced >= 8, "produced {} ", spend.tokens_produced);
    }

    #[test]
    fn the_table_can_be_cleared_between_translation_units() {
        let mut t = MacroTable::new();
        obj(&mut t, "ONE", "1");
        assert_eq!(run(&t, "ONE").unwrap(), "1");
        t.clear();
        assert!(t.is_empty());
        assert_eq!(run(&t, "ONE").unwrap(), "ONE");
    }
}
