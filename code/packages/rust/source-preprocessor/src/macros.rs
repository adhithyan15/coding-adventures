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

/// A definition plus the lookup table substitution needs.
#[derive(Debug)]
pub struct StoredMacro {
    pub def: MacroDef,
    /// Parameter name -> position, built ONCE when the macro is defined.
    ///
    /// Not per invocation, and the difference is a denial of service. Building
    /// it per invocation costs O(|params|) each time, so N invocations of a
    /// k-parameter macro cost O(N x k) from O(N + k) of source -- and the build
    /// sits before the only loop that charges fuel, which iterates the BODY, so
    /// a short body charges nothing for it. A security review measured 0.66 MB
    /// of source at 359 seconds with every counter flat and the preprocessor
    /// returning `Ok`.
    ///
    /// That is the same signature as the per-body-token scan fixed one round
    /// earlier. Hoisting to definition time removes the cost outright rather
    /// than charging for it: a definition is charged once, as source.
    ///
    /// NOTE: `collect` lets a later duplicate win, where the old linear scan
    /// took the first. Dialects reject duplicate parameters, but the engine
    /// must not depend on that, so the behaviour is stated here rather than
    /// assumed away.
    pub param_index: HashMap<String, usize>,
}

/// The macro table.
#[derive(Debug, Default)]
pub struct MacroTable {
    defs: HashMap<String, StoredMacro>,
}

impl MacroTable {
    #[must_use]
    pub fn new() -> MacroTable {
        MacroTable::default()
    }

    pub fn define(&mut self, name: impl Into<String>, def: MacroDef) {
        let param_index = def
            .params
            .as_ref()
            .map(|ps| ps.iter().enumerate().map(|(i, p)| (p.clone(), i)).collect())
            .unwrap_or_default();
        self.defs.insert(name.into(), StoredMacro { def, param_index });
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&StoredMacro> {
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
/// Rescanning is iterative, over an explicit work stack. Argument
/// pre-expansion recurses, bounded by [`Bounds::macro_depth`] — see
/// [`expand_at`] for why that bound is load-bearing rather than defensive.
///
/// `bounds` is clamped to the defaults on entry, so this entry point carries
/// the same tighten-only guarantee `preprocess` does.
pub fn expand(
    input: Vec<MToken>,
    table: &MacroTable,
    hides: &mut HideSets,
    bounds: &Bounds,
    spend: &mut Spend,
) -> Result<Vec<MToken>, PpError> {
    // Clamp on entry, exactly as `preprocess` does.
    //
    // This is a PUBLIC entry point, and `Bounds`'s fields are `pub`, so a host
    // calling `expand` directly could otherwise hand it `u64::MAX` and
    // reinstate the widening bypass that slice 1's review found and fixed at
    // the other entry point. A guarantee enforced at one of two doors is not
    // enforced.
    let bounds = bounds.tighten(Bounds::default());
    expand_at(input, table, hides, &bounds, spend, 0)
}

/// The real expander. `depth` counts nested ARGUMENT pre-expansion, which is
/// the only place this function recurses.
#[allow(clippy::too_many_arguments)]
fn expand_at(
    input: Vec<MToken>,
    table: &MacroTable,
    hides: &mut HideSets,
    bounds: &Bounds,
    spend: &mut Spend,
    depth: u32,
) -> Result<Vec<MToken>, PpError> {
    // The bound that makes `macro_depth` mean what `bounds.rs` says it means.
    //
    // Argument pre-expansion recurses natively, and a security review proved
    // that unbounded: ~800 chained macros, each placing the next invocation
    // inside an ARGUMENT, overflowed the stack from a 21 KB source file. No
    // existing bound came close — the nesting is created by expansion rather
    // than present in the text, so each level's paren depth is only 1, and the
    // token and fuel counters see O(N) work for N levels of stack.
    //
    // A stack overflow in Rust is an abort, not a catchable panic: an
    // embedding host cannot contain it with `catch_unwind`. This module's own
    // header claimed that could not happen here. It could.
    if depth > bounds.macro_depth {
        return Err(PpError::new(format!(
            "macro argument expansion nested deeper than {}",
            bounds.macro_depth
        )));
    }

    let mut work: Vec<MToken> = input;
    work.reverse(); // pop() takes from the front of the logical stream
    let mut out: Vec<MToken> = Vec::new();

    while let Some(cur) = work.pop() {
        spend.fuel_used = spend.fuel_used.saturating_add(1);
        if spend.fuel_used > bounds.fuel {
            return Err(PpError::new(format!(
                "exhausted the {}-step preprocessing budget during macro expansion",
                bounds.fuel
            )));
        }

        let name = cur.token.value.clone();
        let Some(stored) = table.get(&name) else {
            out.push(cur);
            continue;
        };

        let name_id = hides.name(&name);

        // Bound the paint chain. Membership walks it once per token, so an
        // unbounded chain is an unbounded per-token cost that neither the
        // token counter nor fuel can see -- a security review measured the
        // resulting quadratic at 838 KB of source taking 1.59 s and
        // quadrupling per doubling.
        if hides.depth_of(cur.hide) >= bounds.hide_set_depth {
            return Err(PpError::new(format!(
                "more than {} distinct macros painted onto one token",
                bounds.hide_set_depth
            )));
        }

        if hides.contains(cur.hide, name_id) {
            // Painted blue: this exact token came out of an expansion of this
            // macro, so it stays literal however many times it is rescanned.
            out.push(cur);
            continue;
        }

        // Charged against `Spend`, which is the translation unit's tally --
        // NOT a local. `expand` is called once per emitted line and once per
        // condition, so a local counter reset every line and the limit bounded
        // nothing across a file. An earlier fix added a comment claiming this
        // property while the counter was still a local; the counter moved to
        // `Spend` to make the comment true.
        //
        // The limit is its own `Bounds` field rather than `macro_depth`
        // squared, so tightening the stack bound does not silently collapse
        // the work budget.
        spend.expansion_rounds = spend.expansion_rounds.saturating_add(1);
        if spend.expansion_rounds > bounds.expansion_rounds {
            return Err(PpError::new(format!(
                "macro expansion exceeded {} rounds",
                bounds.expansion_rounds
            )));
        }

        let def = &stored.def;
        let produced = match &def.params {
            None => substitute_object_like(def, &cur.token, cur.hide, name_id, hides, bounds, spend)?,
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
                    pre_expand_args(args, table, hides, bounds, spend, depth + 1)?;
                // The intersection of the NAME token's hide set and the
                // CLOSING PAREN's: the invocation spans both, so a name hidden
                // in only one of them was not hidden across the whole
                // invocation. Using the name's set alone over-hides and
                // silently drops expansions.
                let base = hides.intersect(cur.hide, close_hide);
                substitute_function_like(
                    def, &stored.param_index, &cur.token, &expanded_args, base, name_id,
                    hides, bounds, spend,
                )?
            }
        };

        // Rescan: the substituted tokens go back on the work stack so anything
        // they revealed is itself expanded. Termination comes from the hide
        // sets, not from refusing to rescan.
        for t in produced.into_iter().rev() {
            work.push(t);
        }
    }

    Ok(out)
}

/// Charge `n` tokens and `bytes` of token text against the budgets.
///
/// **Called before the allocation, not after it.** That ordering is the whole
/// point. An earlier version built the substitution and then inspected the
/// result, which is unsound for function-like macros: the output is
/// `|parameter occurrences| x |argument tokens|` while the source that
/// produces it costs only their SUM. A security review drove that to **5 GB of
/// working set from a 20 KB file** -- the `Err` arrived, but only after the
/// memory had been allocated, which is no use at all.
///
/// A token costs ~135 bytes all told (104 for `Token`, plus a heap `String`,
/// plus a `Locus`), so a token counter is a memory budget with the units filed
/// off. Charging bytes too is what makes `bounds.rs`'s "bound work AND bytes"
/// true rather than aspirational -- `synthesised_text_bytes` was declared,
/// defaulted, `tighten`ed, and read by nothing until this slice.
fn charge(n: u64, bytes: u64, bounds: &Bounds, spend: &mut Spend) -> Result<(), PpError> {
    spend.tokens_produced = spend.tokens_produced.saturating_add(n);
    if spend.tokens_produced > bounds.tokens_produced {
        return Err(PpError::new(format!(
            "macro expansion produced more than {} tokens",
            bounds.tokens_produced
        )));
    }
    spend.synthesised_text_bytes = spend.synthesised_text_bytes.saturating_add(bytes);
    if spend.synthesised_text_bytes > bounds.synthesised_text_bytes {
        return Err(PpError::new(format!(
            "macro expansion produced more than {} bytes of token text",
            bounds.synthesised_text_bytes
        )));
    }
    Ok(())
}

fn spelling_ok(t: &Token, bounds: &Bounds) -> Result<(), PpError> {
    if t.value.len() as u64 > bounds.token_spelling_bytes {
        return Err(PpError::new(format!(
            "macro expansion produced a token longer than {} bytes",
            bounds.token_spelling_bytes
        )));
    }
    Ok(())
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
    depth: u32,
) -> Result<Vec<Vec<MToken>>, PpError> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        out.push(expand_at(a, table, hides, bounds, spend, depth)?);
    }
    Ok(out)
}

/// Restamp a macro-body token with the position of the invocation it came from.
///
/// Without this, an expanded token carries the line and column it had in the
/// macro BODY, while `emit` stamps it with the file currently being read. A
/// body defined in an included header therefore surfaced at a position inside
/// the *including* file's `@include` line -- text with no relationship to the
/// token. Confidently wrong provenance, which is worse than none.
///
/// The honest interim, not the finished thing: full fidelity needs an
/// `ExpansionId` on `MToken` written into `Locus::expansion`, tracked as
/// VM-069. Until then an expanded token at least points at real text in the
/// file that really produced it.
fn at_invocation(body: &Token, invocation: &Token) -> Token {
    let mut t = body.clone();
    t.line = invocation.line;
    t.column = invocation.column;
    t
}

fn substitute_object_like(
    def: &MacroDef,
    invocation: &Token,
    invocation_hide: HideId,
    name: NameId,
    hides: &mut HideSets,
    bounds: &Bounds,
    spend: &mut Spend,
) -> Result<Vec<MToken>, PpError> {
    // Bounded by the body length, but charged up front all the same, so every
    // path into `Spend` goes through one place.
    let bytes: u64 = def.body.iter().map(|t| t.value.len() as u64).sum();
    charge(def.body.len() as u64, bytes, bounds, spend)?;

    let hide = hides.insert(invocation_hide, name);
    let mut out = Vec::with_capacity(def.body.len());
    for token in &def.body {
        spelling_ok(token, bounds)?;
        out.push(MToken { token: at_invocation(token, invocation), hide });
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn substitute_function_like(
    def: &MacroDef,
    // The precomputed index replaces the parameter slice entirely: nothing
    // here needs the names in order any more, only name -> position.
    param_index: &HashMap<String, usize>,
    invocation: &Token,
    args: &[Vec<MToken>],
    base_hide: HideId,
    name: NameId,
    hides: &mut HideSets,
    bounds: &Bounds,
    spend: &mut Spend,
) -> Result<Vec<MToken>, PpError> {
    // PROJECT the cost before building anything.
    //
    // This is the fix for the quadratic overshoot: a body using its parameter
    // N times, called with N argument tokens, produces N^2 tokens while the
    // source costs 2N. Inspecting the finished vector charges honestly but far
    // too late -- the allocation has already happened, and a 20 KB file
    // reached 5 GB of working set before its diagnostic arrived.
    // Measure each argument ONCE, up front.
    //
    // The obvious projection re-sums an argument's bytes at every parameter
    // occurrence, which makes computing the estimate O(occurrences x argument
    // tokens) -- the very N^2 the projection exists to avoid. It allocates
    // nothing, so memory stays flat and no bound fires; it simply burns CPU.
    // Measured before this hoist: 781 KB of source took 85.8 s, against 0.49 s
    // for the same byte count with a single parameter occurrence. 175x the
    // work, invisible to every counter.
    //
    // With the metrics hoisted the projection is O(|body| x |params|) and each
    // argument is summed once.
    let arg_metrics: Vec<(u64, u64)> = args
        .iter()
        .map(|a| {
            (a.len() as u64, a.iter().map(|t| t.token.value.len() as u64).sum::<u64>())
        })
        .collect();

    // The parameter index arrives precomputed -- see `StoredMacro`. Both loops
    // below once used `params.iter().position(..)` per body token, which is
    // O(|body| x |params|); building the map here instead merely moved the
    // same O(|params|) cost to once per INVOCATION, which a security review
    // then drove to 359 seconds from 0.66 MB of source with every counter
    // flat. Built at definition time it is charged once, as source.

    let mut projected_tokens: u64 = 0;
    let mut projected_bytes: u64 = 0;
    for token in &def.body {
        // Charged per body token, so even the estimate is inside the fuel
        // budget rather than outside it. `bounds.rs` says every loop in the
        // engine needs a finite budget; this one used to be the exception.
        spend.fuel_used = spend.fuel_used.saturating_add(1);
        if spend.fuel_used > bounds.fuel {
            return Err(PpError::new(
                "exhausted the preprocessing budget projecting a substitution",
            ));
        }
        match param_index.get(&token.value).copied() {
            Some(i) => {
                if let Some((n, bytes)) = arg_metrics.get(i) {
                    projected_tokens = projected_tokens.saturating_add(*n);
                    projected_bytes = projected_bytes.saturating_add(*bytes);
                }
            }
            None => {
                projected_tokens = projected_tokens.saturating_add(1);
                projected_bytes = projected_bytes.saturating_add(token.value.len() as u64);
            }
        }
    }
    charge(projected_tokens, projected_bytes, bounds, spend)?;

    let hide = hides.insert(base_hide, name);
    let mut out = Vec::new();

    for token in &def.body {
        match param_index.get(&token.value).copied() {
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
            None => {
                spelling_ok(token, bounds)?;
                out.push(MToken { token: at_invocation(token, invocation), hide });
            }
        }
    }
    Ok(out)
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
    fn deep_argument_nesting_is_a_diagnostic_not_a_stack_overflow() {
        // The shape a security review used to abort the process from a 21 KB
        // source file, before `macro_depth` was enforced as a depth.
        //
        // Each macro puts the next invocation inside an ARGUMENT, so argument
        // pre-expansion recurses once per level. Crucially the nesting is
        // created BY EXPANSION, not present in the text: every level's paren
        // depth is 1, and the token, fuel and round counters see only O(N)
        // work for N levels of native stack. Nothing else catches it.
        //
        // Run on a deliberately small stack: 1 MiB, half of Rust's 2 MiB
        // default for a spawned thread. A stack overflow is an abort, so a
        // regression has to die on a stack the harness cannot quietly enlarge,
        // or it hides behind whatever the runner happened to provide.
        //
        // 1 MiB rather than something smaller because it is MEASURED, not
        // guessed: the default depth of 200 fits here with room to spare and
        // overflows at 256 KiB, so a frame costs somewhere between 1.3 and
        // 5 KiB. That measurement is why `Bounds::macro_depth` documents a
        // minimum stack requirement -- a host on a smaller stack must tighten
        // the bound, and now knows to.
        let handle = std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(|| {
                let mut t = MacroTable::new();
                func(&mut t, "E", &["y"], "y");
                func(&mut t, "D0", &["x"], "x");
                for i in 1..2000 {
                    t.define(
                        format!("D{i}"),
                        MacroDef {
                            params: Some(vec!["x".to_string()]),
                            body: body(&format!("E ( D{} ( x ) )", i - 1)),
                        },
                    );
                }
                // Returns a Result either way; the point is that it RETURNS.
                run(&t, "D1999 ( 42 )").is_err()
            })
            .expect("spawn");

        let hit_the_bound = handle
            .join()
            .expect("expansion must return a diagnostic, not abort the process");
        assert!(hit_the_bound, "a 2000-deep argument chain must hit the depth bound");
    }

    #[test]
    fn the_depth_bound_names_the_configured_depth() {
        let mut t = MacroTable::new();
        func(&mut t, "E", &["y"], "y");
        func(&mut t, "D0", &["x"], "x");
        for i in 1..40 {
            t.define(
                format!("D{i}"),
                MacroDef {
                    params: Some(vec!["x".to_string()]),
                    body: body(&format!("E ( D{} ( x ) )", i - 1)),
                },
            );
        }
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let bounds = Bounds { macro_depth: 4, ..Bounds::default() };
        let e = expand(toks("D39 ( 1 )"), &t, &mut hides, &bounds, &mut spend)
            .expect_err("depth 4 must refuse a 39-deep chain");
        assert!(e.to_string().contains("nested deeper than 4"), "{e}");
    }

    #[test]
    fn a_widened_budget_is_clamped_at_this_entry_point_too() {
        // `expand` is public. Slice 1 found the tighten-only guarantee false
        // at `preprocess`; enforcing it at one of two doors is not enforcing
        // it.
        let mut t = MacroTable::new();
        obj(&mut t, "WIDE", "a b c d e f g h i j");
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let greedy = Bounds {
            tokens_produced: u64::MAX,
            synthesised_text_bytes: u64::MAX,
            macro_depth: u32::MAX,
            ..Bounds::default()
        };
        // Still succeeds on a small program -- clamping is not refusal.
        assert!(expand(toks("WIDE"), &t, &mut hides, &greedy, &mut spend).is_ok());
        // But the budget in force is the default, not u64::MAX.
        assert!(spend.tokens_produced <= Bounds::default().tokens_produced);
    }

    #[test]
    fn expansion_charges_bytes_not_only_token_count() {
        // A token is ~135 bytes in practice, so a token counter alone is a
        // memory budget with the units filed off. Charging bytes is what makes
        // `bounds.rs`'s "bound work AND bytes" true rather than aspirational.
        let mut t = MacroTable::new();
        let wide: String = (0..50).map(|i| format!("tok{i} ")).collect();
        obj(&mut t, "WIDE", &wide);
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let bounds = Bounds { synthesised_text_bytes: 32, ..Bounds::default() };
        let e = expand(toks("WIDE"), &t, &mut hides, &bounds, &mut spend)
            .expect_err("a byte budget of 32 must refuse ~250 bytes of token text");
        assert!(e.to_string().contains("bytes of token text"), "{e}");
    }

    #[test]
    fn repeated_invocations_do_not_re_pay_for_the_parameter_list() {
        // The parameter index is built when the macro is DEFINED, not per
        // invocation. Built per invocation it cost O(|params|) each time, so N
        // invocations of a k-parameter macro cost O(N x k) from O(N + k) of
        // source -- and the build ran before the only fuel-charging loop,
        // which iterates the BODY, so a short body charged nothing for it. A
        // security review measured 0.66 MB of source at 359 seconds with every
        // counter flat and preprocessing returning `Ok`.
        //
        // As with the sibling tests, timing is not asserted. What is asserted
        // is completion at a size the O(N x k) version could not finish, and
        // that the work actually happened.
        let k = 2_000;
        let n = 2_000;
        let names: Vec<String> = (0..k).map(|i| format!("p{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();

        let mut t = MacroTable::new();
        func(&mut t, "F", &refs, "0");

        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let src = std::iter::repeat_n("F ( )", n).collect::<Vec<_>>().join(" ");
        let out = expand(toks(&src), &t, &mut hides, &Bounds::default(), &mut spend)
            .expect("many invocations of a wide macro is legal");
        assert_eq!(out.len(), n, "each invocation yields the one-token body");
    }

    #[test]
    fn a_duplicate_parameter_resolves_to_the_last_occurrence() {
        // Dialects reject duplicate parameters, but the engine must not depend
        // on that -- so its behaviour is pinned rather than left to whichever
        // lookup happens to be in use.
        //
        // The precomputed index is a map, so a later duplicate wins; the old
        // linear `position()` scan took the first. Neither is more correct
        // (the construct is ill-formed), but the charge/build invariant must
        // still hold, which is what actually matters for the memory bound.
        let mut t = MacroTable::new();
        func(&mut t, "F", &["x", "x"], "x + x");

        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let out = expand(toks("F ( 1 , 2 )"), &t, &mut hides, &Bounds::default(), &mut spend)
            .expect("an ill-formed duplicate must not panic");
        let got: Vec<&str> = out.iter().map(|m| m.token.value.as_str()).collect();
        assert_eq!(got, ["2", "+", "2"], "the later duplicate wins under a map lookup");

        // The projection and the build must still agree, or the memory bound
        // is bypassable.
        assert_eq!(spend.tokens_produced, out.len() as u64);
    }

    #[test]
    fn a_long_parameter_list_does_not_make_substitution_quadratic() {
        // `params.iter().position(..)` per body token is O(|body| x |params|),
        // and nothing bounds |params|. A security review drove 1.08 MB of
        // source to 36.8 s with output EMPTY, memory flat and tokens_produced
        // at zero -- every counter saw O(k) while the CPU did O(k^2).
        //
        // Timing is not asserted (too flaky for CI). What is asserted is that
        // the work COMPLETES at a size the quadratic could not have finished,
        // which is the observable consequence.
        let k = 4_000;
        let names: Vec<String> = (0..k).map(|i| format!("p{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let last = names[k - 1].clone();
        let body: String = std::iter::repeat_n(last.as_str(), k).collect::<Vec<_>>().join(" ");

        let mut t = MacroTable::new();
        func(&mut t, "F", &refs, &body);

        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let out = expand(toks("F ( 1 )"), &t, &mut hides, &Bounds::default(), &mut spend)
            .expect("a wide parameter list is legal, just unusual");
        // Every body token names the last parameter, and there is one argument
        // (index 0), so nothing substitutes.
        assert!(out.is_empty(), "produced {} tokens", out.len());
    }

    #[test]
    fn a_long_paint_chain_is_refused_rather_than_walked_forever() {
        // A token's hide set is a chain and membership walks it, so an
        // unbounded chain is an unbounded per-token cost -- quadratic overall,
        // with fuel and rounds both linear and blind to it. Measured before
        // the bound: 838 KB of source at 1.59 s, quadrupling per doubling.
        //
        // The 64-bit Bloom summary on each node makes short chains free but
        // saturates after ~64 distinct names, so it cannot bound this; the
        // depth limit is what does.
        let n = 1_000;
        let mut t = MacroTable::new();
        for i in 1..n {
            t.define(format!("M{i}"), MacroDef { params: None, body: body(&format!("M{}", i + 1)) });
        }
        t.define(format!("M{n}"), MacroDef { params: None, body: body("1") });

        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let bounds = Bounds { hide_set_depth: 64, ..Bounds::default() };
        let e = expand(toks("M1"), &t, &mut hides, &bounds, &mut spend)
            .expect_err("a 1000-deep paint chain must be refused at a depth of 64");
        assert!(e.to_string().contains("distinct macros painted"), "{e}");
    }

    #[test]
    fn ordinary_nesting_is_well_inside_the_paint_bound() {
        // The bound must not reject real code. Ten nested distinct macros is
        // already unusual; the default allows 256.
        let mut t = MacroTable::new();
        for i in 1..10 {
            t.define(format!("M{i}"), MacroDef { params: None, body: body(&format!("M{}", i + 1)) });
        }
        t.define("M10", MacroDef { params: None, body: body("42") });
        assert_eq!(run(&t, "M1").unwrap(), "42");
    }

    #[test]
    fn a_quadratic_substitution_is_refused_before_it_is_built() {
        // A body using its parameter N times, called with N argument tokens,
        // produces N^2 tokens while the SOURCE costs only 2N. Charging the
        // finished vector is honest but useless: a security review reached
        // 5 GB of working set from a 20 KB file, getting its `Err` only after
        // the allocation had happened.
        //
        // So the projection must refuse it. The assertion that matters is not
        // merely "errors" -- it is that `spend.tokens_produced` never records
        // the full N^2, i.e. the budget was consulted before the build.
        let n = 400;
        let mut t = MacroTable::new();
        let body: String = "x ".repeat(n);
        func(&mut t, "BIG", &["x"], &body);
        let arg: String = "1 ".repeat(n);

        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let bounds = Bounds { tokens_produced: 10_000, ..Bounds::default() };
        let src = format!("BIG ( {arg})");
        let e = expand(toks(&src), &t, &mut hides, &bounds, &mut spend)
            .expect_err("N^2 = 160,000 against a 10,000 budget must be refused");
        assert!(e.to_string().contains("tokens"), "{e}");

        // The projection is charged as one lump, so the overshoot is bounded
        // by a single substitution's projection rather than by N^2.
        assert!(
            spend.tokens_produced <= 200_000,
            "charged {} tokens; the budget must be consulted BEFORE building",
            spend.tokens_produced
        );
    }

    #[test]
    fn the_round_budget_is_independent_of_the_stack_bound() {
        // These were once the same number (`macro_depth` squared), so a host
        // tightening the stack bound silently lost over 99% of its work
        // budget and ordinary programs started failing with a rounds
        // diagnostic. A stack limit and a work limit are unrelated.
        let mut t = MacroTable::new();
        obj(&mut t, "A0", "x");
        for i in 1..11 {
            t.define(format!("A{i}"), MacroDef { params: None, body: body(&format!("A{}", i - 1)) });
        }

        // A tight stack bound must not refuse this ordinary program.
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let tight_stack = Bounds { macro_depth: 8, ..Bounds::default() };
        assert!(
            expand(toks("A10"), &t, &mut hides, &tight_stack, &mut spend).is_ok(),
            "tightening macro_depth must cost stack depth, not work budget"
        );

        // And the round budget still bites when IT is what is tightened.
        let mut hides = HideSets::new();
        let mut spend = Spend::default();
        let tight_rounds = Bounds { expansion_rounds: 3, ..Bounds::default() };
        let e = expand(toks("A10"), &t, &mut hides, &tight_rounds, &mut spend)
            .expect_err("a 3-round budget must refuse an 11-deep chain");
        assert!(e.to_string().contains("exceeded 3 rounds"), "{e}");
    }

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
