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

impl Interpreter {
    /// Parse and evaluate a chunk of MATLAB source, returning the concatenated
    /// prompt echo of every unsuppressed result. Variables persist across calls.
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        let tree = try_parse_matlab(source)?;
        self.run(&tree)
    }
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
