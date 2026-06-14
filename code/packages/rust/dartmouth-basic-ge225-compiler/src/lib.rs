//! # dartmouth-basic-ge225-compiler
//!
//! The full Dartmouth BASIC → GE-225 compiled pipeline in a single call.
//!
//! ## Historical context
//!
//! In 1964, a Dartmouth student typed a BASIC program on a Teletype terminal,
//! pressed RETURN, and within seconds the GE-225 time-sharing system printed
//! the output on the same terminal.  The pipeline recreates that sequence:
//!
//! 1. **Lex & parse** (`dartmouth-basic-parser`) — tokenise the BASIC source
//!    and build an AST.
//! 2. **IR compile** (`dartmouth-basic-ir-compiler`) — lower the AST to a
//!    target-independent `IrProgram` where every variable occupies a fixed
//!    virtual register.  `int_bits=20` is used so that the digit-extraction
//!    power constants never exceed the GE-225's 20-bit register range.
//! 3. **GE-225 backend** (`ir-to-ge225-compiler`) — three-pass assembler that
//!    emits 20-bit GE-225 machine words packed three bytes each.
//! 4. **Simulate** (`ge225-simulator`) — load the binary into a behavioural
//!    GE-225 and step until the halt stub is reached.
//!
//! ## Memory layout
//!
//! ```text
//! word 0           : TON  (enable typewriter — emitted by the backend)
//! word 1 …         : compiled IR code
//! word code_end    : BRU code_end  (halt self-loop)
//! word data_base … : spill slots (one per virtual register)
//! ```
//!
//! Register spill slots start at `data_base` (returned by the GE-225 backend).
//! The GE-225 uses 20-bit two's-complement arithmetic; register values are
//! sign-extended to Rust `i32` before being returned in [`RunResult`].
//!
//! ## Quick start
//!
//! ```rust
//! use dartmouth_basic_ge225_compiler::run_basic;
//!
//! let result = run_basic("10 LET A = 6 * 7\n20 END\n").unwrap();
//! assert_eq!(result.var_values["A"], 42);
//! assert_eq!(result.output, "");
//! ```

use coding_adventures_ge225_simulator::{unpack_words, Simulator};
use dartmouth_basic_ir_compiler::{compile_basic_with_options, CharEncoding};
use ir_to_ge225_compiler::compile_to_ge225;
use std::collections::HashMap;

// ===========================================================================
// Public types
// ===========================================================================

/// Outcome of a successful [`run_basic`] call.
///
/// # Fields
///
/// - `output`       — Typewriter characters produced by `PRINT` statements.
///   GE-225 carriage-return codes (0o37) are converted to Unix newlines (`\n`).
/// - `var_values`   — Final integer values of all 26 BASIC scalar variables
///   A–Z after the program halts.  The 20-bit two's-complement words are
///   sign-extended to `i32`.
/// - `steps`        — Number of GE-225 instructions executed.
/// - `halt_address` — Word address of the halt stub (`BRU halt_address`).
///
/// # Example
///
/// ```rust
/// use dartmouth_basic_ge225_compiler::run_basic;
///
/// let result = run_basic("10 LET A = 40 + 2\n20 END\n").unwrap();
/// assert_eq!(result.var_values["A"], 42);
/// ```
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Typewriter output — newline-terminated strings produced by `PRINT`.
    pub output: String,
    /// Final values of BASIC scalar variables A–Z (sign-extended from 20 bits).
    pub var_values: HashMap<String, i32>,
    /// Number of GE-225 instructions simulated.
    pub steps: usize,
    /// Word address of the halt stub (`BRU halt_address`).
    pub halt_address: usize,
}

/// Error returned when the BASIC program cannot be compiled or executed.
///
/// Wraps failures from any of the four pipeline stages: parse, IR compile,
/// GE-225 codegen, and simulation.
///
/// # Example
///
/// ```rust
/// use dartmouth_basic_ge225_compiler::{run_basic, BasicError};
///
/// // GOSUB is not supported in V1
/// let err: Result<_, BasicError> = run_basic("10 GOSUB 100\n20 END\n");
/// assert!(err.is_err());
/// ```
#[derive(Debug, Clone)]
pub struct BasicError(pub String);

impl std::fmt::Display for BasicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BasicError {}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Compile and run a Dartmouth BASIC program on the GE-225 simulator.
///
/// Uses the default options: 4 096 memory words (the full GE-225 machine)
/// and a 100 000 instruction safety limit.
///
/// # Arguments
///
/// - `source` — Dartmouth BASIC source text.  Lines must begin with a line
///   number followed by a statement.  Lines are separated by `\n`.
///
/// # Returns
///
/// A [`RunResult`] with the typewriter output, final variable values,
/// instruction count, and halt address.
///
/// # Errors
///
/// Returns a [`BasicError`] if:
/// - The program uses an unsupported V1 feature (GOSUB, DIM, INPUT, arrays,
///   `^` operator).
/// - A string literal contains a character with no GE-225 typewriter code.
/// - A runtime error occurs (e.g. division by zero in the simulator).
/// - The program has not halted within 100 000 instructions.
///
/// # Example
///
/// ```rust
/// use dartmouth_basic_ge225_compiler::run_basic;
///
/// let result = run_basic(
///     "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n"
/// ).unwrap();
/// assert_eq!(result.output, "1\n2\n3\n4\n5\n");
/// ```
pub fn run_basic(source: &str) -> Result<RunResult, BasicError> {
    run_basic_with_options(source, 4096, 100_000)
}

/// Compile and run a Dartmouth BASIC program with explicit options.
///
/// # Arguments
///
/// - `source`       — BASIC source text
/// - `memory_words` — Total GE-225 memory in 20-bit words (default 4 096)
/// - `max_steps`    — Safety limit on simulated instruction count.
///   [`BasicError`] is returned if the program has not halted by this point.
///
/// # Example
///
/// ```rust
/// use dartmouth_basic_ge225_compiler::run_basic_with_options;
///
/// let result = run_basic_with_options(
///     "10 LET A = 100\n20 END\n",
///     4096,
///     50_000,
/// ).unwrap();
/// assert_eq!(result.var_values["A"], 100);
/// ```
pub fn run_basic_with_options(
    source: &str,
    memory_words: i32,
    max_steps: usize,
) -> Result<RunResult, BasicError> {
    // ── Stage 2: IR compilation ──────────────────────────────────────────────
    //
    // The GE-225 is a 20-bit machine; its maximum positive signed value is
    // 2^19 − 1 = 524,287.  We pass `int_bits=20` so that the digit-extraction
    // powers emitted by `emit_print_number` are bounded by 100,000 (the
    // largest power of ten < 524,287).  Using the default `int_bits=32` would
    // emit `LOAD_IMM 1_000_000_000`, which silently overflows a 20-bit word
    // and produces garbled output.
    //
    // NOTE: The parser is called internally by `compile_basic_with_options`.
    let ir_result = compile_basic_with_options(source, CharEncoding::Ge225, 20)
        .map_err(|e| BasicError(e.to_string()))?;

    // ── Stage 3: GE-225 backend ──────────────────────────────────────────────
    let ge225_result = compile_to_ge225(&ir_result.program)
        .map_err(|e| BasicError(e.to_string()))?;

    // ── Stage 4: simulation ──────────────────────────────────────────────────
    let mut sim = Simulator::new(memory_words);

    // The binary is packed 3 bytes per 20-bit word; unpack before loading.
    let words = unpack_words(&ge225_result.binary)
        .map_err(|e| BasicError(format!("runtime error: {e}")))?;
    sim.load_words(&words, 0)
        .map_err(|e| BasicError(format!("runtime error: {e}")))?;

    let halt_addr = ge225_result.halt_address as i32;
    let mut steps = 0usize;

    loop {
        if steps >= max_steps {
            return Err(BasicError(format!(
                "program did not halt within {max_steps} GE-225 instructions \
                 (possible infinite loop)"
            )));
        }
        let trace = sim.step()
            .map_err(|e| BasicError(format!("runtime error: {e}")))?;
        steps += 1;
        if trace.address == halt_addr {
            break;
        }
    }

    // ── Collect results ──────────────────────────────────────────────────────
    //
    // Each BASIC variable has a fixed virtual register index (A→1, B→2, …Z→26).
    // The GE-225 backend assigns each register a spill slot at
    //   data_base + reg_idx
    // Read the final values and sign-extend from 20-bit two's complement.
    let mut var_values: HashMap<String, i32> = HashMap::new();
    for (var_name, &reg_idx) in &ir_result.var_regs {
        let word_addr = (ge225_result.data_base + reg_idx) as i32;
        let raw = sim.read_word(word_addr).unwrap_or(0);
        // Sign-extend 20-bit two's complement to i32:
        //   if bit 19 is set the value is negative.
        let signed = if raw & (1 << 19) != 0 {
            raw - (1 << 20)
        } else {
            raw
        };
        var_values.insert(var_name.clone(), signed);
    }

    // Convert GE-225 carriage-return (\r, code 0o37) to Unix newline (\n).
    let output = sim.get_typewriter_output().replace('\r', "\n");

    Ok(RunResult {
        output,
        var_values,
        steps,
        halt_address: ge225_result.halt_address,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ────────────────────────────────────────────────────────────

    fn output(source: &str) -> String {
        run_basic(source).expect("compilation/run should succeed").output
    }

    fn result(source: &str) -> RunResult {
        run_basic(source).expect("compilation/run should succeed")
    }

    // ── LET — variable assignment ──────────────────────────────────────────

    #[test]
    fn test_let_constant() {
        let r = result("10 LET A = 42\n20 END\n");
        assert_eq!(r.var_values["A"], 42);
    }

    #[test]
    fn test_let_addition() {
        let r = result("10 LET A = 3 + 4\n20 END\n");
        assert_eq!(r.var_values["A"], 7);
    }

    #[test]
    fn test_let_subtraction() {
        let r = result("10 LET A = 10 - 3\n20 END\n");
        assert_eq!(r.var_values["A"], 7);
    }

    #[test]
    fn test_let_multiplication() {
        let r = result("10 LET A = 6 * 7\n20 END\n");
        assert_eq!(r.var_values["A"], 42);
    }

    #[test]
    fn test_let_division() {
        let r = result("10 LET A = 100 / 4\n20 END\n");
        assert_eq!(r.var_values["A"], 25);
    }

    #[test]
    fn test_let_chained_variables() {
        let r = result("10 LET A = 5\n20 LET B = A * 2\n30 LET C = B + A\n40 END\n");
        assert_eq!(r.var_values["A"], 5);
        assert_eq!(r.var_values["B"], 10);
        assert_eq!(r.var_values["C"], 15);
    }

    #[test]
    fn test_let_negative_result() {
        let r = result("10 LET A = 3 - 10\n20 END\n");
        assert_eq!(r.var_values["A"], -7);
    }

    #[test]
    fn test_let_unary_minus() {
        let r = result("10 LET A = -9\n20 END\n");
        assert_eq!(r.var_values["A"], -9);
    }

    #[test]
    fn test_let_complex_expression() {
        // (2 + 3) * (10 - 4) = 5 * 6 = 30
        let r = result("10 LET A = (2 + 3) * (10 - 4)\n20 END\n");
        assert_eq!(r.var_values["A"], 30);
    }

    #[test]
    fn test_let_overwrites_previous_value() {
        let r = result("10 LET A = 1\n20 LET A = 99\n30 END\n");
        assert_eq!(r.var_values["A"], 99);
    }

    // ── PRINT — string output ──────────────────────────────────────────────

    #[test]
    fn test_hello_world() {
        assert_eq!(output("10 PRINT \"HELLO WORLD\"\n20 END\n"), "HELLO WORLD\n");
    }

    #[test]
    fn test_print_appends_newline() {
        // Each PRINT ends with GE-225 carriage return → converted to \n
        assert_eq!(
            output("10 PRINT \"A\"\n20 PRINT \"B\"\n30 END\n"),
            "A\nB\n"
        );
    }

    #[test]
    fn test_print_bare() {
        // Bare PRINT with no argument emits only the carriage return
        assert_eq!(output("10 PRINT\n20 END\n"), "\n");
    }

    // ── PRINT — numeric output ─────────────────────────────────────────────

    #[test]
    fn test_print_zero() {
        assert_eq!(output("10 PRINT 0\n20 END\n"), "0\n");
    }

    #[test]
    fn test_print_positive_integer() {
        assert_eq!(output("10 PRINT 42\n20 END\n"), "42\n");
    }

    #[test]
    fn test_print_negative_integer() {
        assert_eq!(output("10 LET X = -7\n20 PRINT X\n30 END\n"), "-7\n");
    }

    #[test]
    fn test_print_variable() {
        assert_eq!(output("10 LET A = 99\n20 PRINT A\n30 END\n"), "99\n");
    }

    #[test]
    fn test_print_expression_result() {
        assert_eq!(output("10 PRINT 3 + 4\n20 END\n"), "7\n");
    }

    #[test]
    fn test_print_large_number() {
        assert_eq!(output("10 LET X = 12345\n20 PRINT X\n30 END\n"), "12345\n");
    }

    #[test]
    fn test_print_max_20bit() {
        // Maximum positive 20-bit signed integer: 2^19 − 1 = 524 287
        assert_eq!(
            output("10 LET X = 524287\n20 PRINT X\n30 END\n"),
            "524287\n"
        );
    }

    #[test]
    fn test_print_leading_zero_suppressed() {
        // 7 should print as "7", not "000007"
        assert_eq!(output("10 PRINT 7\n20 END\n"), "7\n");
    }

    #[test]
    fn test_print_mixed_string_and_number() {
        let src = "10 LET N = 99\n20 PRINT \"N IS \", N\n30 END\n";
        assert_eq!(output(src), "N IS 99\n");
    }

    // ── FOR / NEXT loops ───────────────────────────────────────────────────

    #[test]
    fn test_for_prints_sequence() {
        let src = "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n";
        assert_eq!(output(src), "1\n2\n3\n4\n5\n");
    }

    #[test]
    fn test_for_sum_one_to_ten() {
        let src = concat!(
            "10 LET S = 0\n",
            "20 FOR I = 1 TO 10\n",
            "30 LET S = S + I\n",
            "40 NEXT I\n",
            "50 PRINT S\n",
            "60 END\n"
        );
        let r = result(src);
        assert_eq!(r.var_values["S"], 55);
        assert_eq!(r.output, "55\n");
    }

    #[test]
    fn test_for_step_2() {
        // FOR I = 0 TO 8 STEP 2 → 0, 2, 4, 6, 8
        let src = "10 FOR I = 0 TO 8 STEP 2\n20 PRINT I\n30 NEXT I\n40 END\n";
        assert_eq!(output(src), "0\n2\n4\n6\n8\n");
    }

    // ── IF / THEN conditionals ─────────────────────────────────────────────

    #[test]
    fn test_if_lt_taken() {
        // IF 3 < 5 THEN skip past END → prints "YES"
        let src = "10 IF 3 < 5 THEN 30\n20 END\n30 PRINT \"YES\"\n40 END\n";
        assert_eq!(output(src), "YES\n");
    }

    #[test]
    fn test_if_lt_not_taken() {
        let src = "10 IF 5 < 3 THEN 30\n20 PRINT \"NO\"\n30 END\n";
        // Branch not taken → prints "NO"
        assert_eq!(output(src), "NO\n");
    }

    #[test]
    fn test_if_eq() {
        let src = "10 LET A = 5\n20 IF A = 5 THEN 40\n30 END\n40 PRINT \"EQ\"\n50 END\n";
        assert_eq!(output(src), "EQ\n");
    }

    #[test]
    fn test_if_le() {
        // <= synthesised as NOT(CMP_GT)
        let src = "10 LET A = 5\n20 IF A <= 5 THEN 40\n30 END\n40 PRINT \"LE\"\n50 END\n";
        assert_eq!(output(src), "LE\n");
    }

    // ── GOTO ───────────────────────────────────────────────────────────────

    #[test]
    fn test_goto() {
        // GOTO 30 skips line 20 PRINT "SKIP"
        let src = "10 GOTO 30\n20 PRINT \"SKIP\"\n30 PRINT \"OK\"\n40 END\n";
        assert_eq!(output(src), "OK\n");
    }

    // ── Classic programs ───────────────────────────────────────────────────

    #[test]
    fn test_fibonacci_first_ten() {
        // Compute F(10) = 55 using BASIC iteration
        let src = concat!(
            "10 LET A = 0\n",
            "20 LET B = 1\n",
            "30 FOR I = 1 TO 9\n",
            "40 LET C = A + B\n",
            "50 LET A = B\n",
            "60 LET B = C\n",
            "70 NEXT I\n",
            "80 PRINT B\n",
            "90 END\n"
        );
        let r = result(src);
        assert_eq!(r.output, "55\n");
    }

    #[test]
    fn test_countdown() {
        // Count down from 3 to 1
        let src = concat!(
            "10 LET N = 3\n",
            "20 PRINT N\n",
            "30 LET N = N - 1\n",
            "40 IF N > 0 THEN 20\n",
            "50 END\n"
        );
        assert_eq!(output(src), "3\n2\n1\n");
    }

    // ── Error handling ─────────────────────────────────────────────────────

    #[test]
    fn test_gosub_returns_basic_error() {
        let err = run_basic("10 GOSUB 100\n20 END\n100 END\n");
        assert!(err.is_err(), "GOSUB should return BasicError");
        let msg = err.unwrap_err().0;
        assert!(msg.contains("GOSUB"), "error message should mention GOSUB");
    }

    #[test]
    fn test_max_steps_exceeded() {
        // Infinite loop: GOTO itself
        let err = run_basic_with_options("10 GOTO 10\n", 4096, 500);
        assert!(err.is_err(), "infinite loop should exceed max_steps");
        let msg = err.unwrap_err().0;
        assert!(msg.contains("500") || msg.contains("infinite"), "error should mention limit");
    }

    // ── RunResult fields ───────────────────────────────────────────────────

    #[test]
    fn test_result_steps_positive() {
        let r = result("10 LET A = 1\n20 END\n");
        assert!(r.steps > 0, "steps should be > 0 after running");
    }

    #[test]
    fn test_result_halt_address_nonzero() {
        let r = result("10 END\n");
        assert!(r.halt_address > 0, "halt address should be beyond word 0");
    }
}
