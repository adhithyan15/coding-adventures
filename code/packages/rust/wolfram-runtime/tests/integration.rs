//! End-to-end integration tests for the Wolfram runtime.
//!
//! These exercise the *whole* pipeline — parse → lower → evaluate (via the shared
//! `symbolic-vm`) → pretty-print — through the public [`WolframSession`] /
//! [`eval`] surface, mirroring how a REPL or embedder uses the crate. The unit
//! tests inside `src/` pin the individual stages; these assert the observable
//! string-in / string-out contract a user sees.

use coding_adventures_wolfram_runtime::{eval, WolframSession};

/// A representative `1 + 2*3 → 7` end-to-end (the canonical "arithmetic works"
/// check from the W-4 brief).
#[test]
fn arithmetic_precedence_end_to_end() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("(1 + 2)*3\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("2^10\n").unwrap(), "Out[1]= 1024\n");
}

/// The explicit head-applications evaluate identically to their infix forms —
/// the head-name bridge (Plus→Add, Times→Mul, Power→Pow) is the crux of W-4.
#[test]
fn head_applications_match_infix() {
    assert_eq!(eval("Plus[1, 2, 3]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Times[2, 3, 4]\n").unwrap(), "Out[1]= 24\n");
    assert_eq!(eval("Power[2, 10]\n").unwrap(), "Out[1]= 1024\n");
    assert_eq!(eval("Subtract[10, 3]\n").unwrap(), "Out[1]= 7\n");
}

/// Symbols stay symbolic; algebraic identities fold (the `SymbolicBackend`).
#[test]
fn symbolic_evaluation() {
    assert_eq!(eval("x + 0\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x*1\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x^1\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x^0\n").unwrap(), "Out[1]= 1\n");
}

/// List literals evaluate element-wise and round-trip to brace notation.
#[test]
fn lists() {
    assert_eq!(eval("{1, 2, 3}\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("{1 + 1, 2*2, 3^2}\n").unwrap(), "Out[1]= {2, 4, 9}\n");
    assert_eq!(eval("{}\n").unwrap(), "Out[1]= {}\n");
    assert_eq!(
        eval("{1, {2, 3}, 4}\n").unwrap(),
        "Out[1]= {1, {2, 3}, 4}\n"
    );
}

/// Built-in elementary functions reach the shared handlers.
#[test]
fn elementary_functions() {
    assert_eq!(eval("Sin[0]\n").unwrap(), "Out[1]= 0\n");
    assert_eq!(eval("Cos[0]\n").unwrap(), "Out[1]= 1\n");
    // An unknown head passes through unevaluated (Mathematica semantics).
    assert_eq!(eval("g[1, 2]\n").unwrap(), "Out[1]= g[1, 2]\n");
}

/// Assignment binds in the session env and persists across `feed` calls.
#[test]
fn stateful_assignment() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("a = 10\n").unwrap(), "Out[1]= 10\n");
    assert_eq!(s.feed("b = a + 5\n").unwrap(), "Out[2]= 15\n");
    assert_eq!(s.feed("a*b\n").unwrap(), "Out[3]= 150\n");
}

/// A user-defined function via `:=`, then applied.
#[test]
fn user_functions() {
    let mut s = WolframSession::new();
    s.feed("cube[x_] := x^3;\n").unwrap();
    assert_eq!(s.feed("cube[2]\n").unwrap(), "Out[2]= 8\n");
    assert_eq!(s.feed("cube[5]\n").unwrap(), "Out[3]= 125\n");
}

/// `/.` replacement with single rules, list rules, and pattern rules.
#[test]
fn replace_all() {
    assert_eq!(eval("x /. x -> 9\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(
        eval("{a, b} /. {a -> 1, b -> 2}\n").unwrap(),
        "Out[1]= {1, 2}\n"
    );
    // A pattern rule that captures and reuses the bound name on the RHS.
    assert_eq!(eval("h[5] /. h[n_] -> n + 1\n").unwrap(), "Out[1]= 6\n");
}

/// Comparisons and logic fold when fully numeric, stay symbolic otherwise.
#[test]
fn comparisons_and_logic() {
    // `1 < 2` folds to True; an open comparison stays symbolic.
    let out = eval("1 < 2\n").unwrap();
    assert!(out.contains("True"), "got {out:?}");
    assert_eq!(eval("a == a\n").unwrap(), "Out[1]= True\n");
}

/// Multi-statement feed numbers `Out` in order; `;` suppresses display.
#[test]
fn output_numbering_and_suppression() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("1\n2\n").unwrap(), "Out[1]= 1\nOut[2]= 2\n");
    // A suppressed statement consumes an Out index but prints nothing.
    assert_eq!(s.feed("3;\n").unwrap(), "");
    assert_eq!(s.feed("4\n").unwrap(), "Out[4]= 4\n");
}

/// Errors are returned cleanly (never a panic) and the session survives.
#[test]
fn errors_are_clean_and_recoverable() {
    let mut s = WolframSession::new();
    assert!(s.feed("1 +\n").is_err());
    assert!(s.feed("f[x\n").is_err()); // unclosed bracket
    assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[1]= 4\n");
}

/// A short program exercising several features at once.
#[test]
fn a_small_program() {
    let mut s = WolframSession::new();
    s.feed("area[r_] := Pi*r^2;\n").unwrap();
    s.feed("nums = {1, 2, 3};\n").unwrap();
    let out = s.feed("Power[2, 5] + Times[3, 4]\n").unwrap();
    // 32 + 12 = 44
    assert_eq!(out, "Out[3]= 44\n");
}

// ===========================================================================
// W-5 — list / functional / control / numeric built-ins, end-to-end.
// Each case is one of the acceptance examples from the W-5 brief.
// ===========================================================================

/// `Length`, `First`, `Last` on list literals.
#[test]
fn w5_length_first_last() {
    assert_eq!(eval("Length[{1, 2, 3}]\n").unwrap(), "Out[1]= 3\n");
    assert_eq!(eval("First[{9, 8}]\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("Last[{9, 8, 7}]\n").unwrap(), "Out[1]= 7\n");
    // Length of an empty list is 0.
    assert_eq!(eval("Length[{}]\n").unwrap(), "Out[1]= 0\n");
}

/// `First`/`Last` of an empty list are left unevaluated (no panic).
#[test]
fn w5_first_of_empty_is_unevaluated() {
    assert_eq!(eval("First[{}]\n").unwrap(), "Out[1]= First[{}]\n");
}

/// `Part` — 1-based, with negatives and the `0` (head) index.
#[test]
fn w5_part() {
    assert_eq!(eval("Part[{a, b, c}, 2]\n").unwrap(), "Out[1]= b\n");
    assert_eq!(eval("Part[{a, b, c}, -1]\n").unwrap(), "Out[1]= c\n");
    // Out of range stays unevaluated.
    assert_eq!(
        eval("Part[{a, b, c}, 9]\n").unwrap(),
        "Out[1]= Part[{a, b, c}, 9]\n"
    );
}

/// `Append` builds a new list.
#[test]
fn w5_append() {
    assert_eq!(eval("Append[{1, 2}, 3]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
}

/// `Range` in its one-, two-, and three-argument forms.
#[test]
fn w5_range() {
    assert_eq!(eval("Range[3]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("Range[2, 5]\n").unwrap(), "Out[1]= {2, 3, 4, 5}\n");
    assert_eq!(eval("Range[1, 7, 2]\n").unwrap(), "Out[1]= {1, 3, 5, 7}\n");
}

/// `Range[10^9]`-style giant span is refused (left unevaluated), never
/// allocated — the W-5 DoS cap.
#[test]
fn w5_range_oversize_is_unevaluated_not_oom() {
    // 100_000_000 is well above MAX_RANGE_LENGTH; this must NOT hang/OOM.
    let out = eval("Range[100000000]\n").unwrap();
    assert_eq!(out, "Out[1]= Range[100000000]\n");
}

/// `Map` applies a function element-wise and re-evaluates the results.
#[test]
fn w5_map() {
    // f is unbound, so the results stay symbolic f[1], f[2].
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    // A built-in folds: Map[Sin, {0}] → {0}.
    assert_eq!(eval("Map[Sin, {0}]\n").unwrap(), "Out[1]= {0}\n");
}

/// `Apply` swaps the list head and re-evaluates: `Apply[Plus, {…}]` sums via the
/// Plus→Add bridge.
#[test]
fn w5_apply() {
    assert_eq!(eval("Apply[Plus, {1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Apply[Times, {2, 3, 4}]\n").unwrap(), "Out[1]= 24\n");
    // Apply with an unbound head stays symbolic as the application.
    assert_eq!(eval("Apply[g, {a, b}]\n").unwrap(), "Out[1]= g[a, b]\n");
}

/// `If` — the held control head selects a branch; comparisons drive the test.
#[test]
fn w5_if() {
    assert_eq!(eval("If[1 > 0, a, b]\n").unwrap(), "Out[1]= a\n");
    assert_eq!(eval("If[1 < 0, a, b]\n").unwrap(), "Out[1]= b\n");
    // A non-boolean condition leaves the If unevaluated.
    assert_eq!(eval("If[x, a, b]\n").unwrap(), "Out[1]= If[x, a, b]\n");
}

/// `N` coerces exact numbers to floats and maps over a list.
#[test]
fn w5_numeric_n() {
    assert_eq!(eval("N[1/2]\n").unwrap(), "Out[1]= 0.5\n");
    assert_eq!(eval("N[3]\n").unwrap(), "Out[1]= 3.0\n");
    assert_eq!(eval("N[{1, 1/4}]\n").unwrap(), "Out[1]= {1.0, 0.25}\n");
}

/// Built-ins compose: the VM evaluates inner heads before the outer one.
#[test]
fn w5_builtins_compose() {
    // Length[Append[Range[3], 9]] = Length[{1,2,3,9}] = 4
    assert_eq!(
        eval("Length[Append[Range[3], 9]]\n").unwrap(),
        "Out[1]= 4\n"
    );
    // First[Map[f, Range[2]]] = First[{f[1], f[2]}] = f[1]
    assert_eq!(eval("First[Map[f, Range[2]]]\n").unwrap(), "Out[1]= f[1]\n");
    // Apply[Plus, Range[4]] = 1+2+3+4 = 10
    assert_eq!(eval("Apply[Plus, Range[4]]\n").unwrap(), "Out[1]= 10\n");
}

// ===========================================================================
// W-6 — operator sugar /@, @@, [[ ]], end-to-end.
// Each sugar form must evaluate IDENTICALLY to its W-5 head form (MA04 §9).
// ===========================================================================

/// `f /@ x` ≡ `Map[f, x]` — same output string, exactly.
#[test]
fn w6_map_sugar_equals_map_head() {
    assert_eq!(
        eval("f /@ {1, 2}\n").unwrap(),
        eval("Map[f, {1, 2}]\n").unwrap()
    );
    assert_eq!(eval("f /@ {1, 2}\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    // A built-in folds through the sugar just as through the head form.
    assert_eq!(eval("Sin /@ {0}\n").unwrap(), "Out[1]= {0}\n");
}

/// `f @@ x` ≡ `Apply[f, x]`; `Plus @@ {1, 2, 3}` is `6`.
#[test]
fn w6_apply_sugar_equals_apply_head() {
    assert_eq!(
        eval("Plus @@ {1, 2, 3}\n").unwrap(),
        eval("Apply[Plus, {1, 2, 3}]\n").unwrap()
    );
    assert_eq!(eval("Plus @@ {1, 2, 3}\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Times @@ {2, 3, 4}\n").unwrap(), "Out[1]= 24\n");
    // Unbound head stays symbolic as the application, like the head form.
    assert_eq!(eval("g @@ {a, b}\n").unwrap(), "Out[1]= g[a, b]\n");
}

/// `x[[i]]` ≡ `Part[x, i]`; `{a, b, c}[[2]]` is `b`.
#[test]
fn w6_part_sugar_equals_part_head() {
    assert_eq!(
        eval("{a, b, c}[[2]]\n").unwrap(),
        eval("Part[{a, b, c}, 2]\n").unwrap()
    );
    assert_eq!(eval("{a, b, c}[[2]]\n").unwrap(), "Out[1]= b\n");
    // Negative indexing and out-of-range carry over from Part unchanged.
    assert_eq!(eval("{a, b, c}[[-1]]\n").unwrap(), "Out[1]= c\n");
    assert_eq!(
        eval("{a, b, c}[[9]]\n").unwrap(),
        "Out[1]= Part[{a, b, c}, 9]\n"
    );
}

/// Nested / chained `[[ ]]` indexes a nested list: `{{1,2},{3,4}}[[1]][[2]]` = 2.
#[test]
fn w6_part_sugar_nests() {
    assert_eq!(eval("{{1, 2}, {3, 4}}[[1]][[2]]\n").unwrap(), "Out[1]= 2\n");
    // The multi-index spelling is identical: m[[1, 2]] == m[[1]][[2]].
    assert_eq!(
        eval("{{1, 2}, {3, 4}}[[1, 2]]\n").unwrap(),
        eval("{{1, 2}, {3, 4}}[[1]][[2]]\n").unwrap()
    );
}

/// Sugar interleaves with ordinary application without disturbing it.
#[test]
fn w6_sugar_interleaves_with_application() {
    // First[{a, b}][[…]] is nonsense, but f[…][[…]] is fine:
    // Range[3][[2]] = Part[{1,2,3}, 2] = 2.
    assert_eq!(eval("Range[3][[2]]\n").unwrap(), "Out[1]= 2\n");
    // Apply sugar feeding a head form: Plus @@ Range[4] = 10.
    assert_eq!(eval("Plus @@ Range[4]\n").unwrap(), "Out[1]= 10\n");
    // Map sugar then Part sugar: (f /@ {1,2})[[1]] = f[1].
    assert_eq!(eval("(f /@ {1, 2})[[1]]\n").unwrap(), "Out[1]= f[1]\n");
}

/// W-4/W-5 behaviour is unchanged by the W-6 grammar growth — the existing
/// forms still parse and evaluate exactly as before (regression guard).
#[test]
fn w6_does_not_disturb_existing_forms() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("f[g[x]]\n").unwrap(), "Out[1]= f[g[x]]\n"); // nested apply
    assert_eq!(eval("x /. x -> 9\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    assert_eq!(eval("Part[{a, b, c}, 2]\n").unwrap(), "Out[1]= b\n");
}


// ---------------------------------------------------------------------------
// W-7 — iteration constructs (Table, Do, Sum, Product)
// ---------------------------------------------------------------------------

/// `Table` builds a list of the body evaluated with the index bound over a
/// range — both the `{i, imax}` and `{i, imin, imax}` spec forms.
#[test]
fn w7_table_two_and_three_bound_forms() {
    // Table[i^2, {i, 3}] → {1, 4, 9} (index ranges 1..=3).
    assert_eq!(eval("Table[i^2, {i, 3}]\n").unwrap(), "Out[1]= {1, 4, 9}\n");
    // Table[i, {i, 2, 4}] → {2, 3, 4} (explicit lower bound).
    assert_eq!(eval("Table[i, {i, 2, 4}]\n").unwrap(), "Out[1]= {2, 3, 4}\n");
    // Stepped: Table[i, {i, 1, 9, 2}] → {1, 3, 5, 7, 9}.
    assert_eq!(
        eval("Table[i, {i, 1, 9, 2}]\n").unwrap(),
        "Out[1]= {1, 3, 5, 7, 9}\n"
    );
}

/// The index is *local*: the body sees `i` bound to each value, and the symbol
/// `i` does not leak into the session afterward (still a free symbol).
#[test]
fn w7_table_index_is_local() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("Table[i, {i, 3}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    // After the Table, `i` is still unbound (free), not 3 — no env leak.
    assert_eq!(s.feed("i\n").unwrap(), "Out[2]= i\n");
}

/// The iterator *bounds* are evaluated even though the head is held — a bound
/// may be an expression (`{i, 1+1}`) or reference a session binding.
#[test]
fn w7_iterator_bounds_are_evaluated() {
    // A computed bound: Table[i, {i, 1+1}] → {1, 2}.
    assert_eq!(eval("Table[i, {i, 1+1}]\n").unwrap(), "Out[1]= {1, 2}\n");
    // A bound that references a prior binding.
    let mut s = WolframSession::new();
    s.feed("n = 4\n").unwrap();
    assert_eq!(s.feed("Table[i, {i, n}]\n").unwrap(), "Out[2]= {1, 2, 3, 4}\n");
}

/// `Sum` folds `+` over the range; `Product` folds `×`. The canonical
/// acceptance values from the W-7 brief.
#[test]
fn w7_sum_and_product() {
    assert_eq!(eval("Sum[i, {i, 1, 10}]\n").unwrap(), "Out[1]= 55\n");
    assert_eq!(eval("Product[i, {i, 1, 4}]\n").unwrap(), "Out[1]= 24\n");
    // A non-trivial body: Sum[i^2, {i, 1, 3}] = 1 + 4 + 9 = 14.
    assert_eq!(eval("Sum[i^2, {i, 1, 3}]\n").unwrap(), "Out[1]= 14\n");
}

/// An empty range returns the fold identity: `Sum` → 0, `Product` → 1,
/// `Table` → `{}`. (A wrong-way / degenerate range iterates zero times.)
#[test]
fn w7_empty_range_returns_identity() {
    assert_eq!(eval("Sum[i, {i, 5, 1}]\n").unwrap(), "Out[1]= 0\n");
    assert_eq!(eval("Product[i, {i, 5, 1}]\n").unwrap(), "Out[1]= 1\n");
    assert_eq!(eval("Table[i, {i, 0}]\n").unwrap(), "Out[1]= {}\n");
}

/// `Do` evaluates the body once per index *for side effects* and returns
/// `Null`. We prove the body ran the right number of times by observing the
/// final value of a variable it assigns: after `Do[x = i, {i, 3}]`, `x` is 3.
#[test]
fn w7_do_runs_n_times_and_returns_null() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("Do[x = i, {i, 3}]\n").unwrap(), "Out[1]= Null\n");
    // The body ran for i = 1, 2, 3 — the last assignment leaves x = 3.
    assert_eq!(s.feed("x\n").unwrap(), "Out[2]= 3\n");
}

/// Nested `Table` — each level binds its own index cleanly, and the cap
/// composes (the inner build is itself bounded). `i*j` requires explicit `*`.
#[test]
fn w7_nested_table() {
    // i ∈ {1, 2}, j ∈ {1, 2}: rows are {i*1, i*2}.
    assert_eq!(
        eval("Table[Table[i*j, {j, 2}], {i, 2}]\n").unwrap(),
        "Out[1]= {{1, 2}, {2, 4}}\n"
    );
}

/// **DoS cap**: an over-large iterator is left unevaluated rather than hanging
/// or exhausting memory — exactly the `Range` `MAX_RANGE_LENGTH` behaviour. The
/// test returns promptly (no allocation of two-million elements).
#[test]
fn w7_oversize_iterator_is_capped_not_oom() {
    // 2,000,000 > MAX_RANGE_LENGTH (1,000,000): stays unevaluated.
    assert_eq!(
        eval("Table[0, {i, 2000000}]\n").unwrap(),
        "Out[1]= Table[0, {i, 2000000}]\n"
    );
    // `Do` is capped identically even though it allocates nothing — the cap
    // bounds wall-clock work, so this also returns immediately.
    assert_eq!(
        eval("Do[0, {i, 2000000}]\n").unwrap(),
        "Out[1]= Do[0, {i, 2000000}]\n"
    );
}

/// A span too wide for `i64` but valid `i64` *bounds* must not overflow: the
/// count is computed in `i128`, exceeds the cap, and the form stays
/// unevaluated (no panic, no wrap).
#[test]
fn w7_extreme_span_does_not_overflow() {
    let src = "Sum[1, {i, -9000000000000000000, 9000000000000000000}]\n";
    assert_eq!(
        eval(src).unwrap(),
        "Out[1]= Sum[1, {i, -9000000000000000000, 9000000000000000000}]\n"
    );
}

/// A malformed iterator spec leaves the whole form unevaluated (never a panic):
/// a missing bound (`{i}`), a zero step, or a non-integer bound.
#[test]
fn w7_malformed_spec_stays_unevaluated() {
    // No bound: {i} is not a valid iterator.
    assert_eq!(eval("Table[i, {i}]\n").unwrap(), "Out[1]= Table[i, {i}]\n");
    // Zero step never terminates — refused.
    assert_eq!(
        eval("Table[i, {i, 1, 5, 0}]\n").unwrap(),
        "Out[1]= Table[i, {i, 1, 5, 0}]\n"
    );
    // A non-integer bound: this subset only iterates over integer ranges.
    assert_eq!(
        eval("Table[i, {i, 1.5}]\n").unwrap(),
        "Out[1]= Table[i, {i, 1.5}]\n"
    );
}

/// A negative / descending stepped range works (reuses the `Range` span logic).
#[test]
fn w7_negative_and_descending_ranges() {
    assert_eq!(eval("Table[i, {i, -2, 2}]\n").unwrap(), "Out[1]= {-2, -1, 0, 1, 2}\n");
    assert_eq!(eval("Table[i, {i, 5, 1, -2}]\n").unwrap(), "Out[1]= {5, 3, 1}\n");
}
