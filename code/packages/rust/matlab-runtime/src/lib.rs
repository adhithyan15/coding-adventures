//! # MATLAB Runtime — a tree-walking evaluator over `array-runtime`.
//!
//! This is item **MA-3d** of the MATLAB frontend (spec
//! `code/specs/MA01-matlab-language.md`): the runtime that makes the MATLAB
//! lexer/parser executable. It parses with [`coding_adventures_matlab_parser`]
//! and walks the resulting tree with a recursive [`Interpreter`], computing
//! [`MatValue`]s over [`array_runtime`]. Unlike R (which reuses the shared S
//! evaluator), MATLAB has its *own* evaluator because its value model is the
//! `array-runtime` `Array` and its semantics are MATLAB's (1-based,
//! column-major, matrix-first).
//!
//! ## The payoff
//!
//! A matrix product `A * B` lowers to [`array_runtime::execute`]`(MatMul, …)`,
//! which plans the operation and runs it on the cheapest available backend — CPU
//! today, a GPU executor the moment one is registered. So matrix acceleration is
//! automatic and by cost, with no `gpuArray` and no language-level GPU code:
//! the whole MA-1 → MA-2 substrate lights up through real MATLAB syntax.
//!
//! ```
//! use coding_adventures_matlab_runtime::eval;
//!
//! // A matrix product, executed through the array-runtime planner.
//! let out = eval("A = [1 2; 3 4]; A * eye(2)\n").unwrap();
//! assert!(out.contains("ans ="));
//! ```
//!
//! For a persistent session (the REPL), construct an [`Interpreter`] and call
//! [`Interpreter::feed`] repeatedly — variables persist between calls.

mod builtins;
mod eval;
mod value;

pub use eval::Interpreter;
pub use value::MatValue;

use coding_adventures_matlab_parser::try_parse_matlab;

/// Maximum bracket nesting depth accepted by [`Interpreter::feed`]. The
/// recursive-descent parser uses one stack frame per nesting level, so this
/// bound rejects pathologically nested input (`((((…))))`) *before* parsing,
/// turning a would-be stack overflow into a clean error. Real code nests a
/// handful of levels deep; 200 is generous.
const MAX_NESTING: usize = 200;

impl Interpreter {
    /// Parse and evaluate a chunk of MATLAB source, returning the concatenated
    /// prompt echo of every unsuppressed result. Variables persist across calls.
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        check_nesting(source)?;
        let tree = try_parse_matlab(source)?;
        self.run(&tree)
    }
}

/// Reject source whose `(`/`[`/`{` nesting exceeds [`MAX_NESTING`], so the parser
/// is never handed input deep enough to exhaust the stack. A linear pre-scan.
fn check_nesting(source: &str) -> Result<(), String> {
    let mut depth: usize = 0;
    for ch in source.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return Err(format!(
                        "matlab-runtime: input nests deeper than the limit of {MAX_NESTING}"
                    ));
                }
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Evaluate MATLAB source in a fresh session and return its display output.
pub fn eval(source: &str) -> Result<String, String> {
    Interpreter::new().feed(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evaluate and return the trimmed display output.
    fn run(src: &str) -> String {
        eval(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"))
    }

    /// Evaluate `expr` (suppressed) and read back the bound variable's data via a
    /// follow-up `disp`-free echo — here we just parse the echoed scalar.
    fn scalar(src: &str) -> f64 {
        let out = run(src);
        // The echo looks like `ans = 6` or `x = 6`.
        out.rsplit('=')
            .next()
            .unwrap()
            .trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    // --- arithmetic and display -----------------------------------------

    #[test]
    fn scalar_arithmetic_echoes_ans() {
        assert_eq!(scalar("2 + 3 * 4\n"), 14.0);
        assert_eq!(scalar("2 ^ 10\n"), 1024.0);
        assert_eq!(scalar("-2 ^ 2\n"), -4.0); // unary looser than power
    }

    #[test]
    fn semicolon_suppresses_display() {
        assert_eq!(run("x = 5;\n"), ""); // suppressed
        assert!(run("x = 5\n").contains("x = 5")); // shown
    }

    #[test]
    fn assignment_persists_in_a_session() {
        let mut m = Interpreter::new();
        m.feed("a = 10;\n").unwrap();
        m.feed("b = 20;\n").unwrap();
        assert!(m.feed("a + b\n").unwrap().contains("30"));
    }

    // --- matrices, ranges, indexing -------------------------------------

    #[test]
    fn matrix_literal_and_indexing() {
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        assert_eq!(scalar_in(&mut m, "A(2, 1)\n"), 3.0); // 1-based, row 2 col 1
        assert_eq!(scalar_in(&mut m, "A(3)\n"), 2.0); // linear, column-major
        assert_eq!(scalar_in(&mut m, "A(end)\n"), 4.0); // end = last
    }

    #[test]
    fn ranges_and_for_loop() {
        let mut m = Interpreter::new();
        // sum 1..5 via a for loop over a range.
        m.feed("s = 0;\nfor i = 1:5\n  s = s + i;\nend\n").unwrap();
        assert_eq!(scalar_in(&mut m, "s\n"), 15.0);
        // a stepped range
        assert!(m.feed("0:2:6\n").unwrap().contains("ans"));
    }

    #[test]
    fn whole_column_and_row_indexing() {
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        // A(:, 1) is the first column [1; 3]; its sum is 4.
        assert_eq!(scalar_in(&mut m, "sum(A(:, 1))\n"), 4.0);
        assert_eq!(scalar_in(&mut m, "sum(A(2, :))\n"), 7.0); // row 2: 3 + 4
    }

    // --- the headline: matmul through array-runtime ---------------------

    #[test]
    fn matrix_product_lowers_to_execute() {
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        m.feed("B = [5 6; 7 8];\n").unwrap();
        m.feed("C = A * B;\n").unwrap();
        // [[1,2],[3,4]] * [[5,6],[7,8]] = [[19,22],[43,50]].
        assert_eq!(scalar_in(&mut m, "C(1, 1)\n"), 19.0);
        assert_eq!(scalar_in(&mut m, "C(2, 2)\n"), 50.0);
        // A * I == A (the layout round-trips through the executor).
        m.feed("D = A * eye(2);\n").unwrap();
        assert_eq!(scalar_in(&mut m, "D(2, 1)\n"), 3.0);
    }

    #[test]
    fn elementwise_vs_matrix_multiply() {
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        // .* is element-wise; A .* A squares each entry → A(2,2) entry is 16.
        assert_eq!(scalar_in(&mut m, "B = A .* A;\nB(2, 2)\n"), 16.0);
        // scalar * matrix is element-wise scaling.
        assert_eq!(scalar_in(&mut m, "C = 2 * A;\nC(2, 2)\n"), 8.0);
    }

    #[test]
    fn transpose_and_builtins() {
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        assert_eq!(scalar_in(&mut m, "T = A';\nT(1, 2)\n"), 3.0); // transpose
        assert_eq!(scalar_in(&mut m, "sum(sum(eye(3)))\n"), 3.0);
        assert_eq!(scalar_in(&mut m, "numel(zeros(2, 3))\n"), 6.0);
        assert_eq!(scalar_in(&mut m, "length([1 2 3 4])\n"), 4.0);
    }

    #[test]
    fn comparison_and_if() {
        let mut m = Interpreter::new();
        m.feed("x = 5;\nif x > 3\n  y = 1;\nelse\n  y = 0;\nend\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "y\n"), 1.0);
    }

    #[test]
    fn while_loop() {
        let mut m = Interpreter::new();
        m.feed("n = 0;\nwhile n < 10\n  n = n + 1;\nend\n").unwrap();
        assert_eq!(scalar_in(&mut m, "n\n"), 10.0);
    }

    // --- safety / errors ------------------------------------------------

    #[test]
    fn dimension_mismatch_and_oob_are_errors() {
        assert!(eval("[1 2] * [3 4]\n").is_err()); // inner dims disagree
        let mut m = Interpreter::new();
        m.feed("A = [1 2 3];\n").unwrap();
        assert!(m.feed("A(9)\n").is_err()); // out of bounds
        assert!(eval("zeros(1e18)\n").is_err()); // capped, not OOM
        assert!(eval("undefined_thing\n").is_err());
    }

    #[test]
    fn constructor_rejects_an_astronomical_element_product() {
        // Security regression: each dimension alone (67,108,864) is within
        // count()'s own per-dimension cap, but their PRODUCT (~4.5e15
        // elements, ~36 petabytes) is not -- this must be a clean error, not
        // an attempted allocation that aborts the process.
        assert!(eval("zeros(67108864, 67108864)\n").is_err());
        assert!(eval("eye(67108864)\n").is_err());
        // A genuinely small, in-bounds construction still works.
        assert!(eval("zeros(3, 4)\n").is_ok());
    }

    #[test]
    fn matrix_self_concatenation_cannot_double_past_the_element_cap() {
        // Security regression: `[A A]`/`[A; A]` repeated doubles the element
        // count each time with no individually-large input -- only the
        // ACCUMULATED result grows exponentially. 40 repetitions alone
        // already reaches 2^40 elements; this must error out well before an
        // attacker-reachable number of repetitions produces an
        // allocation-aborting size.
        let mut m = Interpreter::new();
        m.feed("A = 1;\n").unwrap();
        let mut last_ok = true;
        for _ in 0..40 {
            last_ok = m.feed("A = [A A];\n").is_ok();
            if !last_ok {
                break;
            }
        }
        assert!(
            !last_ok,
            "40 doublings (up to 2^40 elements) must be rejected before completing"
        );
    }

    #[test]
    fn two_d_indexing_rejects_a_product_overflow() {
        // Security regression: each index vector's own length is bounded
        // independently (here, by `ones`'s own dimension cap) but nothing
        // bounded their PRODUCT -- `A(idx, idx)` with two
        // independently-in-bounds index vectors could still request an
        // astronomical result. 8200x8200 (67,240,000 elements) exceeds the
        // 1<<26 (67,108,864) total-element cap and must be rejected before
        // any allocation is attempted.
        let mut m = Interpreter::new();
        m.feed("A = 0;\n").unwrap();
        m.feed("idx = ones(1, 8200);\n").unwrap();
        assert!(m.feed("B = A(idx, idx);\n").is_err());

        // A genuinely small, in-bounds 2-D index still works.
        let mut m2 = Interpreter::new();
        m2.feed("A = 0;\n").unwrap();
        m2.feed("idx2 = ones(1, 3);\n").unwrap();
        assert!(m2.feed("B = A(idx2, idx2);\n").is_ok());
    }

    #[test]
    fn deeply_nested_input_errors_instead_of_crashing() {
        // Pathologically nested parentheses must be a clean error, not a stack
        // overflow that aborts the process.
        let src = format!("{}1{}\n", "(".repeat(2000), ")".repeat(2000));
        assert!(eval(&src).is_err());
    }

    /// Helper: evaluate in an existing session and read the scalar echo.
    fn scalar_in(m: &mut Interpreter, src: &str) -> f64 {
        let out = m
            .feed(src)
            .unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"));
        out.rsplit('=')
            .next()
            .unwrap()
            .trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }
}
