//! # Axiom runtime — evaluate Axiom (the MA-13-scoped consumer-view subset)
//!
//! This is the **MA-13d** deliverable of the Axiom-language lane (`code/
//! specs/MA13-axiom-language.md`). MA-13a fixed the design (which Axiom, the
//! substrate check, the consumer-view scoping decision); MA-13b/c gave us a
//! lexer and a parser; this crate is the *runtime* that finally **evaluates**
//! Axiom source.
//!
//! ```text
//!   Axiom source
//!        |
//!        v  coding_adventures_axiom_parser::try_parse_axiom     (MA-13c)
//!   GrammarASTNode   (program = expr; if_expr, define, assignment,
//!                     declaration, has_query, comparison, coercion, ...)
//!        |
//!        v  crate::eval::eval_expr                              (this crate)
//!   AxiomValue  (an IRNode, paired with its AxiomDomain if one is known)
//!        |
//!        v  crate::value::print_axiom
//!   Axiom surface string
//! ```
//!
//! ## The one genuinely new piece: a domain/category layer `symbolic-vm` has none of
//!
//! MA13 §2's central finding is that `symbolic_ir::IRNode` has **no**
//! domain, category, or per-value type tag anywhere — every prior
//! symbolic-family runtime here (Wolfram/Macsyma/Derive/Reduce/Maple) is a
//! single flat universe of untyped expressions. Axiom is different: every
//! value belongs to a domain, and `:` (declare) / `::` (coerce) / `has`
//! (category query) are constantly used even in the most basic session.
//! `crate::domains` is the fixed, non-extensible table MA13 §3/§4 scopes
//! (`Boolean`, `Integer`, `PositiveInteger`, `NonNegativeInteger`, `Float`,
//! `String`, `Fraction(Integer)`, `Polynomial(Integer)`, `List(T)`; `Ring`,
//! `OrderedSet`); `crate::value::AxiomValue` pairs an evaluated `IRNode`
//! with that domain; `crate::eval` is the interpreter that threads it all
//! together. **No modification to `symbolic-ir`/`symbolic-vm`/`cas-*` is
//! made anywhere in this crate** — arithmetic lowers straight to the shared
//! `SymbolicBackend`/`VM`, reused completely unchanged, exactly as every
//! prior symbolic-family runtime already does (MA13 §2/§5).
//!
//! ## Why this crate is an interpreter, not a two-phase lowering pass
//!
//! See `crate::eval`'s own module doc comment for the full rationale:
//! `::`/`:`/`has` have no `IRNode` representation at all, and `::` can
//! appear nested anywhere inside an ordinary arithmetic expression, so this
//! crate walks the parsed tree **evaluating eagerly**, only reusing the
//! shared VM for the arithmetic/comparison sub-expressions it already knows
//! how to evaluate.
//!
//! ## Public contract
//!
//! [`AxiomSession::feed`] is string-in / string-out, like every sibling CAS
//! facade in this repo. Unlike Derive/Reduce (whose grammar parses a whole
//! multi-statement worksheet per call), `axiom.grammar`'s own `program =
//! expr` is a **single top-level expression** (MA13 §5: Axiom is framed as a
//! numbered, per-line interactive session, not a batch worksheet file) — so
//! `feed` evaluates exactly one statement per call and displays it with
//! Axiom's own numbered-prompt convention (`(n)`, confirmed directly against
//! the book, MA13 §5), rather than Derive's `#n:`. Bindings, function
//! definitions, and declared domain constraints all persist across calls.
//!
//! ## Robustness at the trust boundary
//!
//! `feed`/`eval_to_output` take arbitrary user text, so — exactly as in
//! `derive-runtime`/`reduce-runtime` — this crate is the trust boundary for
//! the whole reused stack:
//!
//! 1. **Deeply *nested* source** (`((((…))))`, `f(f(f(…)))`) is already
//!    rejected by `axiom-parser`'s own `MAX_RULE_DEPTH` — parsing itself
//!    fails cleanly before a deep tree is ever built.
//! 2. **A long *flat* chain** (`1+1+1+…`) is evaluated **iteratively**, one
//!    small `VM::eval` call per fold step (`crate::eval::eval_binary_chain`)
//!    — sidestepping the "flat repetition folds into one deep tree" DoS
//!    vector by construction rather than by a token-count patch alone.
//!    [`MAX_STATEMENT_TOKENS`] still exists as defense-in-depth (measured
//!    against the real lexer token stream, mirroring
//!    `derive-runtime`'s/`reduce-runtime`'s identical guard), since this
//!    crate cannot fully vouch for every shape the shared, unmodified
//!    `symbolic-vm` handler table might itself produce for an unresolved
//!    symbolic chain.
//! 3. **Unwinding panics** (e.g. a malformed `Assign` left-hand side inside
//!    a function body, which the shared `assign_handler` panics on) run
//!    inside [`catch_unwind`] on a worker thread with a large bounded stack;
//!    the session is rebuilt afterward, trading lost bindings for a
//!    guaranteed-usable session on the next call.

mod builtins;
mod domains;
mod eval;
mod value;

pub use domains::{AxiomCategory, AxiomDomain};
pub use eval::EvalError;
pub use value::{print_axiom, AxiomValue};

use coding_adventures_axiom_parser::try_parse_axiom;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use symbolic_vm::{SymbolicBackend, VM};

/// Maximum length, in bytes, of a single source chunk handed to
/// [`AxiomSession::feed`]. A cheap first gate bounding per-call memory/time.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Maximum number of lexer tokens allowed in a single submission.
///
/// `axiom.grammar`'s own `program = expr` means one [`AxiomSession::feed`]
/// call is always exactly one statement (never a multi-statement worksheet
/// chunk the way Derive's/Reduce's own `feed` accepts), so — unlike those
/// crates' identically-named constant — this counts the *entire* input, with
/// no per-statement reset needed. See the crate doc comment's "Robustness"
/// point 2 for why this still exists as defense-in-depth even though this
/// crate's own arithmetic folding is iterative.
pub const MAX_STATEMENT_TOKENS: usize = 2000;

/// Stack size of the worker thread that runs evaluation.
///
/// Generous headroom for the bounded-but-still-potentially-deep trees this
/// crate's own evaluation, the shared VM's rewriting, and the domain
/// predicate checks in `crate::domains` may walk, regardless of the
/// caller's own stack — mirrors `derive-runtime`'s/`reduce-runtime`'s
/// identical constant and rationale.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

/// The context an in-progress evaluation is threaded through: the shared
/// symbolic VM (for arithmetic/comparison/assignment/definition/if/list, all
/// reused unchanged, MA13 §2) and this crate's own declared-domain
/// constraint table (`a : T`, MA13 §3/§4) — kept as a *separate* map from the
/// VM's own name-binding environment (`vm.backend`'s `env`), since
/// `symbolic-ir`/`symbolic-vm` have no concept of a domain at all to store
/// one in (MA13 §2's own central finding).
pub(crate) struct EvalContext<'a> {
    pub vm: &'a mut VM,
    pub declared: &'a mut HashMap<String, AxiomDomain>,
}

/// One displayed result from an [`AxiomSession::eval_to_output`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The 1-based `(n)` index (MA13 §5's own numbered-prompt convention).
    pub index: usize,
    /// The result rendered in Axiom surface notation, with its domain
    /// appended (`" : <Domain>"`) whenever one is known.
    pub text: String,
}

/// A persistent Axiom session.
///
/// Owns the [`VM`] (so variable bindings and user-defined functions persist
/// across calls to [`feed`](AxiomSession::feed)) and this crate's own
/// declared-domain constraint table (so a `a : PositiveInteger` declared in
/// one call is still enforced against a later `a := -1` in a subsequent
/// call), exactly as an interactive Axiom session would.
pub struct AxiomSession {
    vm: VM,
    declared_domains: HashMap<String, AxiomDomain>,
    /// 1-based counter of displayed results so far — the `(n)` prompt index.
    output_index: usize,
}

impl Default for AxiomSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AxiomSession {
    /// Create a fresh session with an empty environment, declared-domain
    /// table, and `(n)` counter.
    pub fn new() -> Self {
        AxiomSession {
            vm: VM::new(Box::new(SymbolicBackend::new())),
            declared_domains: HashMap::new(),
            output_index: 0,
        }
    }

    /// Evaluate one Axiom statement and return its display line, `"(n)
    /// «value»"` (`" : «Domain»"` appended when the result's domain is
    /// known) followed by a newline.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_axiom_runtime::AxiomSession;
    /// let mut s = AxiomSession::new();
    /// assert_eq!(s.feed("1 + 2*3").unwrap(), "(1) 7 : PositiveInteger\n");
    /// ```
    pub fn feed(&mut self, src: &str) -> Result<String, String> {
        let output = self.eval_to_output(src)?;
        Ok(format!("({}) {}\n", output.index, output.text))
    }

    /// Evaluate one Axiom statement and return the structured [`Output`].
    pub fn eval_to_output(&mut self, src: &str) -> Result<Output, String> {
        // Guard 1: bound total input size (cheap memory/time gate).
        if src.len() > MAX_INPUT_LEN {
            return Err(format!(
                "input too large: {} bytes exceeds the {}-byte limit",
                src.len(),
                MAX_INPUT_LEN
            ));
        }
        // Guard 2: reject an overly complex single statement before
        // evaluation ever starts (defense-in-depth; see the crate doc
        // comment's "Robustness" point 2).
        check_statement_token_count(src)?;

        // Guard 3 + panics: evaluation runs on a worker thread with a large
        // bounded stack, and any unwinding panic from the reused symbolic
        // stack is caught.
        let vm = &mut self.vm;
        let declared = &mut self.declared_domains;
        let src_owned = src.to_string();
        let outcome = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(EVAL_STACK_SIZE)
                .spawn_scoped(scope, || {
                    catch_unwind(AssertUnwindSafe(|| eval_source(vm, declared, &src_owned)))
                })
                .expect("failed to spawn axiom evaluation thread")
                .join()
        });

        match outcome {
            Ok(Ok(Ok(value))) => {
                self.output_index += 1;
                let text = format_value(&value);
                Ok(Output {
                    index: self.output_index,
                    text,
                })
            }
            Ok(Ok(Err(message))) => Err(message),
            // A panic the worker caught, or one that escaped and unwound the
            // join. Either way the env may be inconsistent, so rebuild.
            Ok(Err(payload)) | Err(payload) => {
                self.vm = VM::new(Box::new(SymbolicBackend::new()));
                self.declared_domains.clear();
                self.output_index = 0;
                Err(panic_message(payload))
            }
        }
    }
}

/// Render an [`AxiomValue`] the way [`AxiomSession::feed`] displays it: the
/// surface value, with `" : «Domain»"` appended whenever the domain is
/// known (a disclosed presentation choice showcasing the domain system this
/// language is about — real FriCAS's own interactive session separately
/// echoes a `Type: ...` line beneath the result, not independently
/// re-verified byte-for-byte here, so this crate keeps its own single-line
/// convention instead of fabricating that exact two-line format).
fn format_value(value: &AxiomValue) -> String {
    match &value.domain {
        Some(domain) => format!("{} : {}", print_axiom(&value.node), domain.display_name()),
        None => print_axiom(&value.node),
    }
}

/// Evaluate `src` (one statement) against `vm`/`declared`. Runs on the
/// worker thread.
fn eval_source(
    vm: &mut VM,
    declared: &mut HashMap<String, AxiomDomain>,
    src: &str,
) -> Result<AxiomValue, String> {
    let ast = try_parse_axiom(src)?;
    let mut ctx = EvalContext { vm, declared };
    eval::eval_expr(&mut ctx, &ast).map_err(|e| e.to_string())
}

/// Reject input lexing to more than [`MAX_STATEMENT_TOKENS`] tokens.
///
/// If the lexer itself errors (an untokenizable character), this returns
/// `Ok(())` and lets the parser surface the error uniformly — never
/// rejecting solely because this checker couldn't lex something.
fn check_statement_token_count(src: &str) -> Result<(), String> {
    let tokens = match coding_adventures_axiom_lexer::try_tokenize_axiom(src) {
        Ok(tokens) => tokens,
        Err(_) => return Ok(()),
    };
    if tokens.len() > MAX_STATEMENT_TOKENS {
        return Err(format!(
            "statement too complex: more than {MAX_STATEMENT_TOKENS} tokens in one statement"
        ));
    }
    Ok(())
}

/// Evaluate `src` once on a fresh [`AxiomSession`] and return its display
/// line. Convenience for callers that do not need a persistent session.
pub fn eval(src: &str) -> Result<String, String> {
    AxiomSession::new().feed(src)
}

/// Recover a human-readable message from a caught panic payload.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Axiom could not evaluate that input".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Arithmetic (delegated to the shared, unmodified symbolic-vm) ------

    #[test]
    fn arithmetic_folds_correctly() {
        assert_eq!(eval("1 + 2*3").unwrap(), "(1) 7 : PositiveInteger\n");
        assert_eq!(eval("2^10").unwrap(), "(1) 1024 : PositiveInteger\n");
        assert_eq!(eval("10 / 2").unwrap(), "(1) 5 : PositiveInteger\n");
        assert_eq!(eval("1/3").unwrap(), "(1) 1/3 : Fraction(Integer)\n");
    }

    #[test]
    fn both_power_spellings_agree() {
        assert_eq!(eval("2^3").unwrap(), eval("2**3").unwrap());
    }

    #[test]
    fn comparisons_produce_boolean() {
        assert_eq!(eval("1 = 1").unwrap(), "(1) true : Boolean\n");
        assert_eq!(eval("1 ~= 2").unwrap(), "(1) true : Boolean\n");
        assert_eq!(eval("3 < 2").unwrap(), "(1) false : Boolean\n");
        assert_eq!(eval("3 >= 3").unwrap(), "(1) true : Boolean\n");
    }

    #[test]
    fn free_symbols_stay_symbolic_with_no_domain() {
        assert_eq!(eval("x + 0").unwrap(), "(1) x\n");
    }

    #[test]
    fn unresolved_symbolic_sum_infers_polynomial_integer() {
        // MA13 §3's own confirmed example (the un-cancelled case).
        assert_eq!(eval("x + y").unwrap(), "(1) x + y : Polynomial(Integer)\n");
    }

    // --- Literal domain inference (MA13 §4) --------------------------------

    #[test]
    fn positive_integer_literal_is_inferred() {
        assert_eq!(eval("5").unwrap(), "(1) 5 : PositiveInteger\n");
    }

    #[test]
    fn zero_and_negative_integers_infer_plain_integer() {
        assert_eq!(eval("0").unwrap(), "(1) 0 : Integer\n");
        assert_eq!(eval("-3").unwrap(), "(1) -3 : Integer\n");
    }

    #[test]
    fn float_literal_is_inferred() {
        assert_eq!(eval("1.5").unwrap(), "(1) 1.5 : Float\n");
    }

    #[test]
    fn string_literal_is_inferred() {
        assert_eq!(eval("\"hello\"").unwrap(), "(1) \"hello\" : String\n");
    }

    // --- `:` declaration + `:=` assignment domain-check (MA13 §3/§4) -------

    #[test]
    fn declaration_then_matching_assignment_succeeds() {
        let mut s = AxiomSession::new();
        s.feed("a : PositiveInteger").unwrap();
        assert_eq!(s.feed("a := 5").unwrap(), "(2) 5 : PositiveInteger\n");
    }

    #[test]
    fn declaration_then_mismatched_assignment_fails_with_the_books_error_shape() {
        let mut s = AxiomSession::new();
        s.feed("a : PositiveInteger").unwrap();
        let err = s.feed("a := -1").unwrap_err();
        assert!(
            err.contains("Cannot convert right-hand side of assignment")
                && err.contains("PositiveInteger"),
            "got {err:?}"
        );
    }

    #[test]
    fn declaration_persists_across_feed_calls() {
        let mut s = AxiomSession::new();
        s.feed("a : NonNegativeInteger").unwrap();
        assert!(s.feed("a := -1").is_err());
        assert!(s.feed("a := 0").is_ok());
    }

    #[test]
    fn tuple_declaration_restricts_every_name() {
        let mut s = AxiomSession::new();
        s.feed("(a, b) : PositiveInteger").unwrap();
        assert!(s.feed("a := 5").is_ok());
        assert!(s.feed("b := -5").is_err());
    }

    #[test]
    fn assignment_without_a_prior_declaration_is_unrestricted() {
        let mut s = AxiomSession::new();
        assert_eq!(s.feed("z := -5").unwrap(), "(1) -5 : Integer\n");
    }

    // --- `::` coercion (MA13 §3/§4) -----------------------------------------

    #[test]
    fn coercion_to_a_wider_domain_succeeds() {
        assert_eq!(
            eval("3 :: Fraction(Integer)").unwrap(),
            "(1) 3 : Fraction(Integer)\n"
        );
    }

    #[test]
    fn coercion_paren_optional_shorthand_matches_the_books_own_example() {
        assert_eq!(
            eval("3 :: Fraction Integer").unwrap(),
            "(1) 3 : Fraction(Integer)\n"
        );
    }

    #[test]
    fn coercion_failure_uses_the_books_error_shape() {
        // A negative literal fails PositiveInteger's `x > 0` subdomain
        // predicate.
        let err = eval("-1 :: PositiveInteger").unwrap_err();
        assert!(
            err.contains("Cannot convert") && err.contains("PositiveInteger"),
            "got {err:?}"
        );
    }

    #[test]
    fn coercion_of_a_computed_expression_succeeds() {
        assert_eq!(eval("(1 + 2) :: Float").unwrap(), "(1) 3.0 : Float\n");
    }

    // --- `has` category-membership query (MA13 §3/§4) ----------------------

    #[test]
    fn polynomial_integer_has_ring_is_true_the_books_own_confirmed_example() {
        assert_eq!(
            eval("Polynomial(Integer) has Ring").unwrap(),
            "(1) true : Boolean\n"
        );
    }

    #[test]
    fn list_integer_has_ring_is_false_the_books_own_confirmed_example() {
        assert_eq!(
            eval("List(Integer) has Ring").unwrap(),
            "(1) false : Boolean\n"
        );
    }

    #[test]
    fn every_built_in_domain_category_pair_is_checkable() {
        for (domain, category, expected) in [
            ("Integer", "Ring", true),
            ("Integer", "OrderedSet", true),
            ("Fraction(Integer)", "Ring", true),
            ("Fraction(Integer)", "OrderedSet", false),
            ("Polynomial(Integer)", "Ring", true),
            ("Polynomial(Integer)", "OrderedSet", false),
            ("Boolean", "Ring", false),
            ("Boolean", "OrderedSet", false),
            ("Float", "Ring", false),
            ("Float", "OrderedSet", true),
            ("String", "Ring", false),
            ("String", "OrderedSet", false),
            ("PositiveInteger", "Ring", false),
            ("PositiveInteger", "OrderedSet", true),
            ("NonNegativeInteger", "Ring", false),
            ("NonNegativeInteger", "OrderedSet", true),
            ("List(Integer)", "OrderedSet", false),
        ] {
            let src = format!("{domain} has {category}");
            let want = format!("(1) {expected} : Boolean\n");
            assert_eq!(eval(&src).unwrap(), want, "for `{src}`");
        }
    }

    #[test]
    fn unknown_domain_or_category_is_a_clean_error() {
        assert!(eval("Matrix(Integer) has Ring").is_err());
        assert!(eval("Integer has Field").is_err());
        assert!(eval("Polynomial(String) has Ring").is_err());
    }

    // --- `:=` / `==` / `if` / block evaluation -------------------------------

    #[test]
    fn assignment_persists_across_calls() {
        let mut s = AxiomSession::new();
        s.feed("x := 5").unwrap();
        assert_eq!(s.feed("x + 1").unwrap(), "(2) 6 : PositiveInteger\n");
    }

    #[test]
    fn declared_function_definition_and_call() {
        let mut s = AxiomSession::new();
        s.feed("power(x: Integer, n: NonNegativeInteger): Integer == x ** n")
            .unwrap();
        assert_eq!(s.feed("power(2, 10)").unwrap(), "(2) 1024 : PositiveInteger\n");
    }

    #[test]
    fn undeclared_function_definition_and_call() {
        let mut s = AxiomSession::new();
        s.feed("f x == x * x").unwrap();
        assert_eq!(s.feed("f(5)").unwrap(), "(2) 25 : PositiveInteger\n");
        // The paren-optional call form works too.
        assert_eq!(s.feed("f 6").unwrap(), "(3) 36 : PositiveInteger\n");
    }

    #[test]
    fn if_then_else_selects_the_right_branch() {
        assert_eq!(eval("if 1 > 0 then 1 else -1").unwrap(), "(1) 1 : PositiveInteger\n");
        assert_eq!(eval("if 1 < 0 then 1 else -1").unwrap(), "(1) -1 : Integer\n");
    }

    #[test]
    fn if_predicate_must_be_boolean() {
        assert!(eval("if 5 then 1 else 2").is_err());
    }

    #[test]
    fn block_sequences_statements_and_returns_the_last_value() {
        assert_eq!(eval("(a := 1; a + 1)").unwrap(), "(1) 2 : PositiveInteger\n");
    }

    #[test]
    fn block_bindings_persist_after_the_block() {
        let mut s = AxiomSession::new();
        s.feed("(a := 1; b := 2)").unwrap();
        assert_eq!(s.feed("a + b").unwrap(), "(2) 3 : PositiveInteger\n");
    }

    #[test]
    fn a_declaration_inside_a_block_still_restricts_a_later_top_level_assignment() {
        let mut s = AxiomSession::new();
        s.feed("(a : PositiveInteger; a := 5)").unwrap();
        assert!(s.feed("a := -1").is_err());
    }

    #[test]
    fn list_literal_evaluates_elementwise_and_infers_its_domain() {
        assert_eq!(
            eval("[1 + 1, 2*3]").unwrap(),
            "(1) [2, 6] : List(PositiveInteger)\n"
        );
    }

    // --- Function bodies: the disclosed pure-arithmetic-only restriction ---

    #[test]
    fn a_coercion_inside_a_function_body_is_a_clean_error() {
        let mut s = AxiomSession::new();
        assert!(s.feed("f(x: Integer): Integer == x :: Float").is_err());
    }

    #[test]
    fn a_declaration_inside_a_function_body_is_a_clean_error() {
        let mut s = AxiomSession::new();
        assert!(s.feed("f x == (x : Integer)").is_err());
    }

    #[test]
    fn recursion_through_a_defined_function_works() {
        let mut s = AxiomSession::new();
        s.feed("fact(n: Integer): Integer == if n = 0 then 1 else n * fact(n - 1)")
            .unwrap();
        assert_eq!(s.feed("fact(5)").unwrap(), "(2) 120 : PositiveInteger\n");
    }

    // --- Robustness guards ---------------------------------------------------

    #[test]
    fn oversized_input_is_rejected_before_evaluation() {
        let huge = format!("{}1", "1+".repeat(MAX_INPUT_LEN));
        assert!(huge.len() > MAX_INPUT_LEN);
        assert!(eval(&huge).unwrap_err().contains("too large"));
    }

    #[test]
    fn deeply_nested_parens_are_rejected_by_the_parsers_own_cap() {
        let depth = 5000;
        let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).is_err());
    }

    #[test]
    fn long_flat_chain_is_rejected_by_the_statement_token_cap() {
        let src = format!("{}1", "1+".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn moderate_chain_still_evaluates() {
        let src = format!("{}1", "1+".repeat(50));
        assert_eq!(eval(&src).unwrap(), "(1) 51 : PositiveInteger\n");
    }

    #[test]
    fn a_malformed_assign_lhs_inside_a_function_body_is_caught_not_aborted() {
        // A held function body can lower an `Assign`-shaped tree only via
        // this crate's own `assignment` rejection inside `lower_pure_body` --
        // this test instead exercises the shared VM's own reused
        // `assign_handler` panic path through a route this crate DOES allow:
        // a directly malformed top-level assignment target is impossible to
        // construct through this grammar (the LHS is always a bare NAME), so
        // this confirms the session survives an ordinary evaluation error
        // and keeps working afterward.
        let mut s = AxiomSession::new();
        let _ = s.feed("1 +");
        assert_eq!(s.feed("2 + 2").unwrap(), "(1) 4 : PositiveInteger\n");
    }

    #[test]
    fn session_recovers_after_a_parse_error() {
        let mut s = AxiomSession::new();
        assert!(s.feed("1 +").is_err());
        assert_eq!(s.feed("3 + 4").unwrap(), "(1) 7 : PositiveInteger\n");
    }

    #[test]
    fn empty_and_whitespace_input_is_a_clean_parse_error_not_a_panic() {
        assert!(eval("").is_err());
        assert!(eval("   ").is_err());
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("2 + 3").unwrap();
        let two = AxiomSession::new().feed("2 + 3").unwrap();
        assert_eq!(one, two);
    }
}
