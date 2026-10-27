//! Python's `repr(float)`, written in IIR by this compiler (LANG83 PR7c-py).
//!
//! # Why the compiler writes it
//!
//! LANG83 PR7 says the IIR names no source language. Up to 0.3.0 the release
//! path printed a float with the `write_f64_py` builtin, so every one of the
//! nine backends carried a copy of Python's float layout. Now the backends
//! know only neutral float primitives (LANG83 PR6c `f64_signbit`, PR7b
//! `f64_class`, `f64_digits`, `f64_exp10`), and the layout itself (*where
//! the decimal point goes, when to switch to `1e+16`, why `1.0` keeps its
//! `.0`*) is ordinary IIR that this compiler puts into every module that
//! prints a float:
//!
//! | Primitive        | What it says about `x`                                        |
//! | ---------------- | ------------------------------------------------------------- |
//! | `f64_class x`    | `0` for ±0, `1` finite non-zero, `2` ±inf, `3` NaN            |
//! | `f64_signbit x`  | `1` when the sign bit is set (−0.0 included), else `0`        |
//! | `f64_digits x`   | `D`, the shortest round-trip digits of `|x|`, no trailing 0s  |
//! | `f64_exp10 x`    | `e`, the power of ten of `D`'s leading digit                  |
//!
//! So `|x| = 0.D × 10^(e+1)`: the digits of `123.456` are `D = 123456` and
//! `e = 2`; those of `0.0025` are `D = 25` and `e = -3`. Everything else is
//! integer arithmetic and `putchar`. "Shortest round-trip digits" is exactly
//! what CPython's `float_repr` asks `dtoa.c` for (mode 0, `'r'`), so the
//! digits are Python's; only the layout below is the compiler's.
//!
//! # The helpers
//!
//! The names are this compiler's own. They are module-local functions, not
//! registry ops, and no backend treats them specially (they compile like any
//! function a program defines). The release-path subset defines no Python
//! functions at all (a `def` is not in `py-float-print`), and a Python value
//! lives in a register named `pyN`, so nothing a script writes can collide
//! with the `__py_` names.
//!
//! ```text
//!   main ──call──▶ __py_write_float(x)            repr(x), one float
//!                     ├─call─▶ __py_digit_count(D)         how many digits D has
//!                     ├─call─▶ __py_write_digits(D, n, from, to)
//!                     └─call─▶ __py_write_uint(u)           the exponent's digits
//! ```
//!
//! `__py_write_float` is CPython 3's `float_repr` (`PyOS_double_to_string(x,
//! 'r', 0, Py_DTSF_ADD_DOT_0, NULL)`), written as the LANG83 spec's
//! pseudo-code:
//!
//! ```text
//! c = f64_class x;  s = f64_signbit x
//! c == 3: "nan"                          c == 2: s ? "-inf" : "inf"
//! c == 0: s ? "-0.0" : "0.0"
//! D = f64_digits x;  e = f64_exp10 x;  n = digit_count(D);  if s: '-'
//! if -4 <= e < 16:                       # fixed
//!     if e >= 0:  write_digits(D, n, 0, e+1); '.'; if n > e+1: write_digits(D, n, e+1, n) else '0'
//!     else:       "0."; (-e-1) × '0'; write_digits(D, n, 0, n)
//! else:                                  # scientific
//!     write_digits(D, n, 0, 1); if n > 1: '.'; write_digits(D, n, 1, n)
//!     'e'; e < 0 ? '-' : '+'; if |e| < 10: '0'; write_uint(|e|)
//! ```
//!
//! Worked through five values:
//!
//! | `x`         | `D`      | `e`   | `n` | branch     | written                 |
//! | ----------- | -------- | ----- | --- | ---------- | ----------------------- |
//! | `1e15`      | `1`      | `15`  | 1   | integral   | `1` + fifteen `0`s `.0` |
//! | `123.456`   | `123456` | `2`   | 6   | point      | `123` `.` `456`         |
//! | `0.0025`    | `25`     | `-3`  | 2   | below one  | `0.` `00` `25`          |
//! | `1e16`      | `1`      | `16`  | 1   | scientific | `1` `e` `+` `16`        |
//! | `1.5e-07`   | `15`     | `-7`  | 2   | scientific | `1` `.` `5` `e-` `0` `7`|
//!
//! Where the two branches meet (CPython's `format_float_short`: exponent
//! form when `decpt <= -4 or decpt > 16`, with `decpt = e + 1`):
//!
//! | `e`       | -5      | -4       | … | 15                     | 16      |
//! | --------- | ------- | -------- | - | ---------------------- | ------- |
//! | `repr`    | `1e-05` | `0.0001` | … | `1000000000000000.0`   | `1e+16` |
//!
//! Python, unlike JavaScript, prints the sign of every zero itself
//! (`repr(-0.0)` is `'-0.0'`), so a `print` site needs no sign test of its
//! own: it is one `call`. A NaN's sign is never shown (`repr` of a negative
//! NaN is `'nan'`, LANG83 PR6c).
//!
//! # Which integer ops, and why they are safe on every backend
//!
//! The legacy `div` has backend-specific edges for a zero divisor and for
//! negative operands (LANG83 "Legacy edge behaviour"). The helpers never
//! reach either: every `div` divides a **non-negative** value by a
//! **non-zero power of ten**. A digit is `q − (q div 10) × 10` rather than
//! `q mod 10`, so the digit writer also compiles on the generic JIT, which
//! has no `mod` bytecode (the value is the same for `q ≥ 0`).
//!
//! The largest integer is `D < 10^17` (and the power of ten next to it), so
//! every value fits an `i64`. Each helper returns `i64` (`0`, or the count)
//! from a single `ret`, because every AOT calling convention handles an
//! integer return.
//!
//! # What a print costs
//!
//! A print used to be one VM step (`call_builtin write_f64_py`). Now it is
//! the helper's run: [`MAX_STEPS_PER_FLOAT_PRINT`] bounds it, and the release
//! runner's step budget is derived from that bound (`release_api.rs`).

use interpreter_ir::{FunctionTypeStatus, IIRFunction, IIRInstr, Operand};

/// `repr(x)` of one float: the helper every float `print` calls.
pub(crate) const WRITE_FLOAT: &str = "__py_write_float";
/// `write_digits(D, n, from, to)`: digit positions `from..to` of `D`.
pub(crate) const WRITE_DIGITS: &str = "__py_write_digits";
/// `digit_count(D)`: the number of decimal digits of `D ≥ 1`.
pub(crate) const DIGIT_COUNT: &str = "__py_digit_count";
/// `write_uint(u)`: the decimal digits of `u ≥ 0` (BASIC's `__basic_print_uint`).
pub(crate) const WRITE_UINT: &str = "__py_write_uint";

/// A bound on the VM steps one float `print(x)` takes: the helper's run,
/// its `call`, and `main`'s own instructions for that print.
///
/// The dearest layouts write 17 significant digits behind a long place-value
/// scale and a three-digit exponent. Measured on vm-core, the whole
/// `print(x)` program:
///
/// | `x`                         | steps |
/// | --------------------------- | ----- |
/// | `-1.7976931348623157e+308`  | 700   |
/// | `-1.2345678901234567e-308`  | 697   |
/// | `-0.00012345678901234567`   | 596   |
/// | `-1234567890123456.8`       | 557   |
/// | `1.5`                       | 134   |
/// | `0.0`, `nan`                | 32    |
///
/// The release runner's unit tests measure the dearest over a 1,248-value
/// sweep and require it to stay at or below this bound and above half of
/// it, so a change to the helpers cannot silently outgrow the runner's step
/// budget (`release_api.rs`), and the bound cannot drift into meaning
/// nothing. A print used to be one step (`call_builtin write_f64_py`).
pub const MAX_STEPS_PER_FLOAT_PRINT: u64 = 1024;

/// The four helpers, in the order a reader meets them: the entry point
/// first, then what it calls. A module that prints a float gets all four;
/// one that prints none (every `py-int-print` and `py2-int-print` module)
/// gets none.
pub(crate) fn float_repr_helpers() -> Vec<IIRFunction> {
    vec![
        helper(WRITE_FLOAT, &[("x", "f64")], write_float_body()),
        helper(DIGIT_COUNT, &[("d", "i64")], digit_count_body()),
        helper(
            WRITE_DIGITS,
            &[("d", "i64"), ("n", "i64"), ("from", "i64"), ("to", "i64")],
            write_digits_body(),
        ),
        helper(WRITE_UINT, &[("u", "i64")], write_uint_body()),
    ]
}

/// One helper function. Every instruction carries a concrete type hint
/// (`i64`, `f64` or `void`), so (like BASIC's print helpers and unlike
/// `main`, whose output calls are `void`) the function is fully typed, and
/// the JIT compiles it eagerly when it can.
fn helper(name: &str, params: &[(&str, &str)], body: Vec<IIRInstr>) -> IIRFunction {
    let params = params
        .iter()
        .map(|(name, ty)| (name.to_string(), ty.to_string()))
        .collect();
    let mut function = IIRFunction::new(name, params, "i64", body);
    function.type_status = FunctionTypeStatus::FullyTyped;
    function
}

/// A tiny assembler for helper bodies. Register and label names are local to
/// the function being built, so short words (`d`, `n`, `done`) are enough.
#[derive(Default)]
struct Body {
    code: Vec<IIRInstr>,
}

fn var(name: &str) -> Operand {
    Operand::Var(name.to_string())
}

impl Body {
    /// `dest <- op srcs…`, an `i64` result.
    fn op(&mut self, op: &str, dest: &str, srcs: &[&str]) {
        let srcs = srcs.iter().map(|s| var(s)).collect();
        self.code
            .push(IIRInstr::new(op, Some(dest.to_string()), srcs, "i64"));
    }

    /// `dest <- n` (an `i64` constant).
    fn int(&mut self, dest: &str, n: i64) {
        self.code.push(IIRInstr::new(
            "const",
            Some(dest.to_string()),
            vec![Operand::Int(n)],
            "i64",
        ));
    }

    fn label(&mut self, name: &str) {
        self.code
            .push(IIRInstr::new("label", None, vec![var(name)], "void"));
    }

    fn jmp(&mut self, target: &str) {
        self.code
            .push(IIRInstr::new("jmp", None, vec![var(target)], "void"));
    }

    /// Fall through when `cond` is non-zero; jump to `target` when it is `0`.
    fn unless(&mut self, cond: &str, target: &str) {
        self.code.push(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![var(cond), var(target)],
            "void",
        ));
    }

    /// `dest <- call function(args…)`.
    fn call(&mut self, dest: &str, function: &str, args: &[&str]) {
        let mut srcs = vec![var(function)];
        srcs.extend(args.iter().map(|a| var(a)));
        self.code
            .push(IIRInstr::new("call", Some(dest.to_string()), srcs, "i64"));
    }

    /// `putchar` of the byte already in register `byte`.
    fn putchar(&mut self, byte: &str) {
        self.code.push(IIRInstr::new(
            "call_builtin",
            None,
            vec![var("putchar"), var(byte)],
            "void",
        ));
    }

    /// Write fixed text one byte at a time through the scratch register `ch`.
    fn text(&mut self, text: &str) {
        for byte in text.bytes() {
            self.int("ch", i64::from(byte));
            self.putchar("ch");
        }
    }

    /// `ret 0`.
    fn ret_zero(&mut self) {
        self.int("r", 0);
        self.code
            .push(IIRInstr::new("ret", None, vec![var("r")], "i64"));
    }

    fn finish(self) -> Vec<IIRInstr> {
        self.code
    }
}

/// `__py_write_float(x)`: `repr(x)`. One exit (`done`) so every backend sees
/// a single `ret`.
///
/// ```text
///   c = class x, s = signbit x
///   c == 3 ─▶ "nan"                        ┐
///   c == 2 ─▶ s ? "-inf" : "inf"           │ the special values
///   c == 0 ─▶ s ? "-0.0" : "0.0"           ┘
///   D, e, n; s ? '-'
///   -4 <= e < 16 ?  integral-or-point │ below one
///                :  scientific
/// ```
fn write_float_body() -> Vec<IIRInstr> {
    let mut b = Body::default();
    b.int("zero", 0);
    b.int("one", 1);
    b.int("two", 2);
    b.int("three", 3);
    b.op("f64_class", "c", &["x"]);
    b.op("f64_signbit", "s", &["x"]);

    // NaN carries no visible sign: `repr(-nan)` is 'nan'.
    b.op("cmp_eq", "t", &["c", "three"]);
    b.unless("t", "not_nan");
    b.text("nan");
    b.jmp("done");

    b.label("not_nan");
    b.op("cmp_eq", "t", &["c", "two"]);
    b.unless("t", "finite");
    b.unless("s", "positive_infinity");
    b.text("-");
    b.label("positive_infinity");
    b.text("inf");
    b.jmp("done");

    // ±0: Python shows the sign of zero (`repr(-0.0)` is '-0.0'), and a
    // zero is the one integral value whose digits `f64_digits` cannot give.
    b.label("finite");
    b.op("cmp_eq", "t", &["c", "zero"]);
    b.unless("t", "nonzero");
    b.unless("s", "positive_zero");
    b.text("-");
    b.label("positive_zero");
    b.text("0.0");
    b.jmp("done");

    b.label("nonzero");
    b.op("f64_digits", "d", &["x"]);
    b.op("f64_exp10", "e", &["x"]);
    b.call("n", DIGIT_COUNT, &["d"]);
    b.unless("s", "unsigned");
    b.text("-");
    b.label("unsigned");
    // e + 1 is both "one past the last integer digit" and, negated, one more
    // than the count of zeros after "0." (0.0025: e = -3, two zeros).
    b.op("add", "e1", &["e", "one"]);

    // Fixed notation exactly when -4 <= e < 16 (`format_float_short`'s
    // `decpt <= -4 || decpt > 16` test, with decpt = e + 1, negated).
    b.int("low", -4);
    b.op("cmp_ge", "t", &["e", "low"]);
    b.unless("t", "scientific");
    b.int("high", 16);
    b.op("cmp_lt", "t", &["e", "high"]);
    b.unless("t", "scientific");
    b.op("cmp_ge", "t", &["e", "zero"]);
    b.unless("t", "below_one");

    // At least one (0 <= e < 16): the integer digits, then the point. Digit
    // positions past n are '0' (1e15 is "1" then fifteen "0"s).
    b.call("ignored", WRITE_DIGITS, &["d", "n", "zero", "e1"]);
    b.text(".");
    // Digits left after the point (123.456), or Python's `.0` on an
    // integral value (`Py_DTSF_ADD_DOT_0`: 100.0, 1000000000000000.0).
    b.op("cmp_gt", "t", &["n", "e1"]);
    b.unless("t", "integral");
    b.call("ignored", WRITE_DIGITS, &["d", "n", "e1", "n"]);
    b.jmp("done");
    b.label("integral");
    b.text("0");
    b.jmp("done");

    // Below one (-4 <= e < 0): "0.", then -e-1 zeros, then all the digits.
    // The zeros are digit positions n .. n + (-e-1), which lie past D's end
    // and so are written as '0' — no separate zero loop is needed.
    b.label("below_one");
    b.text("0.");
    b.op("sub", "zeros", &["zero", "e1"]);
    b.op("add", "zeros_end", &["n", "zeros"]);
    b.call("ignored", WRITE_DIGITS, &["d", "n", "n", "zeros_end"]);
    b.call("ignored", WRITE_DIGITS, &["d", "n", "zero", "n"]);
    b.jmp("done");

    // Scientific: d[.ddd]e±XX, the exponent at least two digits (1e+16,
    // 1.5e-07, 5e-324): CPython pads with `%+.02d`.
    b.label("scientific");
    b.call("ignored", WRITE_DIGITS, &["d", "n", "zero", "one"]);
    b.op("cmp_gt", "t", &["n", "one"]);
    b.unless("t", "mantissa_done");
    b.text(".");
    b.call("ignored", WRITE_DIGITS, &["d", "n", "one", "n"]);
    b.label("mantissa_done");
    b.text("e");
    b.op("cmp_lt", "t", &["e", "zero"]);
    b.unless("t", "exponent_positive");
    b.text("-");
    b.op("sub", "u", &["zero", "e"]);
    b.jmp("exponent_pad");
    b.label("exponent_positive");
    b.text("+");
    b.op("add", "u", &["e", "zero"]);
    b.label("exponent_pad");
    b.int("ten", 10);
    b.op("cmp_lt", "t", &["u", "ten"]);
    b.unless("t", "exponent_digits");
    b.text("0");
    b.label("exponent_digits");
    b.call("ignored", WRITE_UINT, &["u"]);

    b.label("done");
    b.ret_zero();
    b.finish()
}

/// `__py_digit_count(d)`: the smallest `n ≥ 1` with `d < 10^n`.
///
/// ```text
///   d = 7       n = 1   (7 < 10)
///   d = 25      n = 2   (25 < 100)
///   d = 10^16   n = 17  (the loop stops at p = 10^17)
/// ```
///
/// `f64_digits` promises `d < 10^17`, so `p` never passes `10^17` and never
/// overflows an `i64`.
fn digit_count_body() -> Vec<IIRInstr> {
    let mut b = Body::default();
    b.int("one", 1);
    b.int("ten", 10);
    b.int("n", 1);
    b.int("p", 10);
    b.label("more");
    b.op("cmp_ge", "t", &["d", "p"]);
    b.unless("t", "counted");
    b.op("add", "n", &["n", "one"]);
    b.op("mul", "p", &["p", "ten"]);
    b.jmp("more");
    b.label("counted");
    b.code
        .push(IIRInstr::new("ret", None, vec![var("n")], "i64"));
    b.finish()
}

/// `__py_write_digits(d, n, from, to)`: write digit positions `from..to` of
/// the `n`-digit integer `d`, position 0 being the leading digit. A position
/// at or past `n` is written as `'0'`.
///
/// ```text
///   d = 25, n = 2:   positions   0   1   2   3 …
///                    digits      2   5   0   0 …
///   write_digits(25, 2, 0, 4)  →  "2500"
///   write_digits(25, 2, 1, 2)  →  "5"
/// ```
///
/// `p` is the place value of position `i`, `10^(n-1-i)`. It is computed once
/// for `from`, then divided by ten per digit, so a digit costs one `div` for
/// the quotient and one `div`, `mul`, `sub` for its last decimal place.
fn write_digits_body() -> Vec<IIRInstr> {
    let mut b = Body::default();
    b.int("zero", 0);
    b.int("one", 1);
    b.int("ten", 10);
    b.int("ascii_zero", i64::from(b'0'));

    // p = 10^(n-1-from); when from >= n the exponent is negative, p stays 1
    // and is never used (every position is past the digits).
    b.int("p", 1);
    b.op("sub", "k", &["n", "one"]);
    b.op("sub", "k", &["k", "from"]);
    b.label("scale");
    b.op("cmp_gt", "t", &["k", "zero"]);
    b.unless("t", "scaled");
    b.op("mul", "p", &["p", "ten"]);
    b.op("sub", "k", &["k", "one"]);
    b.jmp("scale");
    b.label("scaled");

    b.op("add", "i", &["from", "zero"]);
    b.label("next");
    b.op("cmp_lt", "t", &["i", "to"]);
    b.unless("t", "written");
    b.op("cmp_lt", "t", &["i", "n"]);
    b.unless("t", "past_end");
    // digit = (d div p) mod 10, spelled q - (q div 10) * 10.
    b.op("div", "q", &["d", "p"]);
    b.op("div", "q10", &["q", "ten"]);
    b.op("mul", "q10", &["q10", "ten"]);
    b.op("sub", "digit", &["q", "q10"]);
    b.op("div", "p", &["p", "ten"]);
    b.jmp("put");
    b.label("past_end");
    b.int("digit", 0);
    b.label("put");
    b.op("add", "ch", &["ascii_zero", "digit"]);
    b.putchar("ch");
    b.op("add", "i", &["i", "one"]);
    b.jmp("next");
    b.label("written");
    b.ret_zero();
    b.finish()
}

/// `__py_write_uint(u)`: the decimal digits of `u ≥ 0`, most significant
/// first — the exponent of the scientific form (at most `324`).
///
/// This is Dartmouth BASIC's `__basic_print_uint`, unchanged: write the
/// digits of `u div 10` first (recursively, only when `u ≥ 10`), then `u`'s
/// own last digit.
///
/// ```text
///   write_uint(324) → write_uint(32) → write_uint(3) → '3'
///                                    → '2'
///                   → '4'
/// ```
fn write_uint_body() -> Vec<IIRInstr> {
    let mut b = Body::default();
    b.int("ten", 10);
    b.op("cmp_ge", "t", &["u", "ten"]);
    b.unless("t", "last_digit");
    b.op("div", "q", &["u", "ten"]);
    b.call("ignored", WRITE_UINT, &["q"]);
    b.label("last_digit");
    b.op("div", "q", &["u", "ten"]);
    b.op("mul", "q", &["q", "ten"]);
    b.op("sub", "digit", &["u", "q"]);
    b.int("ascii_zero", i64::from(b'0'));
    b.op("add", "ch", &["ascii_zero", "digit"]);
    b.putchar("ch");
    b.ret_zero();
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The helpers are built only from the ops the spec allows a layout
    /// helper (plus the ungated `const`, branches and `ret`), and every
    /// `call` names one of the four.
    #[test]
    fn helpers_use_only_neutral_registry_ops_and_call_each_other() {
        let names = [WRITE_FLOAT, DIGIT_COUNT, WRITE_DIGITS, WRITE_UINT];
        for function in float_repr_helpers() {
            assert!(names.contains(&function.name.as_str()));
            assert_eq!(function.type_status, FunctionTypeStatus::FullyTyped);
            assert_eq!(function.return_type, "i64");
            for instruction in &function.instructions {
                let op = instruction.op.as_str();
                assert!(
                    [
                        "const", "label", "jmp", "jmp_if_false", "ret", "call", "call_builtin",
                        "add", "sub", "mul", "div", "cmp_eq", "cmp_lt", "cmp_gt", "cmp_ge",
                        "f64_class", "f64_signbit", "f64_digits", "f64_exp10",
                    ]
                    .contains(&op),
                    "{}: {instruction:?}",
                    function.name
                );
                match op {
                    "call" => assert!(names.contains(&instruction.srcs[0].as_var().unwrap())),
                    "call_builtin" => assert_eq!(instruction.srcs[0], var("putchar")),
                    _ => {}
                }
                // No register or hint is left for a backend to guess.
                assert_ne!(instruction.type_hint, "any", "{}: {instruction:?}", function.name);
            }
            // One exit per helper.
            let rets = function.instructions.iter().filter(|i| i.op == "ret").count();
            assert_eq!(rets, 1, "{}", function.name);
            assert_eq!(function.instructions.last().unwrap().op, "ret", "{}", function.name);
        }
    }

    /// Only `f64_*` reads touch the float; the rest is integer code, and the
    /// float ops live in `__py_write_float` alone.
    #[test]
    fn only_the_entry_helper_reads_the_float() {
        for function in float_repr_helpers() {
            let reads_float = function.instructions.iter().any(|i| i.op.starts_with("f64_"));
            assert_eq!(reads_float, function.name == WRITE_FLOAT, "{}", function.name);
        }
    }

    /// No `mod` anywhere: the generic JIT has no `mod` bytecode, and the
    /// digit is `q - (q div 10) * 10` instead (LANG83 "What PR7c-js landed").
    #[test]
    fn no_helper_uses_mod() {
        for function in float_repr_helpers() {
            assert!(function.instructions.iter().all(|i| i.op != "mod"), "{}", function.name);
        }
    }
}
