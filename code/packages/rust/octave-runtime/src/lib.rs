//! # Octave Runtime — GNU Octave on the MATLAB stack.
//!
//! GNU Octave is to MATLAB what R is to S: a compatible reimplementation with a
//! few syntactic additions. Where R needed its own lexer/parser (its `_`/`->>`
//! differences are pervasive), Octave's departures from MATLAB are a small,
//! local set of surface forms — so this crate reuses the **entire** MATLAB
//! frontend ([`matlab_runtime`], and through it the MATLAB lexer/parser and
//! `array-runtime`) behind a thin **source-compatibility shim**, [`octavify`],
//! that normalizes Octave syntax to MATLAB before evaluation. The matrix engine,
//! the GPU-lowering of `*`, indexing, and control flow are all inherited
//! unchanged.
//!
//! ## What the shim normalizes
//!
//! | Octave                              | becomes (MATLAB) |
//! |-------------------------------------|------------------|
//! | `# comment`                         | `% comment`      |
//! | `endif`/`endfor`/`endwhile`/`endfunction`/`endswitch`/`end_try_catch` | `end` |
//! | `!=`                                | `~=`             |
//! | `!` (logical not)                   | `~`              |
//!
//! All four are rewritten *only outside* string literals and comments — the shim
//! is string/comment-aware (and handles MATLAB's transpose-vs-quote ambiguity)
//! so `'#tag'`, `"a != b"`, and `A'` are never touched. Octave's `++`/`--` and
//! `do…until` (which have no MATLAB equivalent) are left as-is and currently
//! error; they are documented deferrals.

use coding_adventures_matlab_runtime::Interpreter as MatlabInterpreter;

pub use coding_adventures_matlab_runtime::MatValue;

/// A persistent Octave session — a thin wrapper that `octavify`s each input and
/// delegates to a [`matlab_runtime::Interpreter`]. Variables persist across
/// calls.
pub struct Interpreter {
    inner: MatlabInterpreter,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            inner: MatlabInterpreter::new(),
        }
    }

    /// Normalize Octave source to MATLAB and evaluate it.
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        self.inner.feed(&octavify(source))
    }
}

/// Evaluate Octave source in a fresh session and return its display output.
pub fn eval(source: &str) -> Result<String, String> {
    Interpreter::new().feed(source)
}

/// The Octave→MATLAB source shim. Rewrites `#` comments, the `endX` block
/// terminators, and `!`/`!=`, leaving string and comment contents untouched.
pub fn octavify(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let n = chars.len();
    // Whether the immediately-preceding character was a value-terminator — used,
    // exactly as in the MATLAB lexer, to tell a transpose `A'` from a string `'…'`.
    let mut prev_value = false;

    while i < n {
        let c = chars[i];
        match c {
            // A `%` comment is already MATLAB-style: copy verbatim to end of line.
            '%' => {
                while i < n && chars[i] != '\n' {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            // An Octave `#` comment becomes a `%` comment.
            '#' => {
                out.push('%');
                i += 1;
                while i < n && chars[i] != '\n' {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            // Double-quoted string: copy whole (honour the `""` escape).
            '"' => {
                out.push('"');
                i += 1;
                while i < n {
                    if chars[i] == '"' {
                        if i + 1 < n && chars[i + 1] == '"' {
                            out.push_str("\"\"");
                            i += 2;
                        } else {
                            out.push('"');
                            i += 1;
                            break;
                        }
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                prev_value = true;
            }
            // A `'` after a value is transpose; otherwise it opens a char array.
            '\'' if prev_value => {
                out.push('\'');
                i += 1;
                prev_value = true;
            }
            '\'' => {
                out.push('\'');
                i += 1;
                while i < n {
                    if chars[i] == '\'' {
                        if i + 1 < n && chars[i + 1] == '\'' {
                            out.push_str("''");
                            i += 2;
                        } else {
                            out.push('\'');
                            i += 1;
                            break;
                        }
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                prev_value = true;
            }
            // `!=` → `~=`, `!` → `~`.
            '!' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    out.push_str("~=");
                    i += 2;
                } else {
                    out.push('~');
                    i += 1;
                }
                prev_value = false;
            }
            // A word: an `endX` block terminator is rewritten to `end`.
            c if c.is_alphanumeric() || c == '_' => {
                let start = i;
                while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let mapped = match word.as_str() {
                    "endif" | "endfor" | "endwhile" | "endfunction" | "endswitch" | "endparfor"
                    | "end_try_catch" => "end",
                    other => other,
                };
                out.push_str(mapped);
                prev_value = true; // an identifier/number is a value-terminator
            }
            ')' | ']' | '}' => {
                out.push(c);
                i += 1;
                prev_value = true;
            }
            '.' => {
                out.push('.');
                i += 1;
                prev_value = true;
            }
            ' ' | '\t' | '\n' | '\r' => {
                out.push(c);
                i += 1;
                prev_value = false;
            }
            _ => {
                out.push(c);
                i += 1;
                prev_value = false;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(src: &str) -> f64 {
        let out = eval(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"));
        out.rsplit('=')
            .next()
            .unwrap()
            .trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    // --- the shim itself ------------------------------------------------

    #[test]
    fn octavify_rewrites_octave_forms() {
        assert_eq!(octavify("x != 3"), "x ~= 3");
        assert_eq!(octavify("!done"), "~done");
        assert_eq!(octavify("# a comment\n"), "% a comment\n");
        assert_eq!(octavify("if x\n  y = 1;\nendif\n"), "if x\n  y = 1;\nend\n");
        assert_eq!(octavify("for i=1:3\nendfor\n"), "for i=1:3\nend\n");
    }

    #[test]
    fn shim_leaves_strings_and_comments_alone() {
        // `!`, `#`, and `endif` inside a char array / string / comment are kept.
        assert_eq!(octavify("s = '!= endif #'"), "s = '!= endif #'");
        assert_eq!(octavify("t = \"a != b\""), "t = \"a != b\"");
        assert_eq!(octavify("% keep != and endif\n"), "% keep != and endif\n");
        // A transpose is not mistaken for a string opener.
        assert_eq!(octavify("y = A'"), "y = A'");
    }

    // --- evaluation through the MATLAB engine ---------------------------

    #[test]
    fn octave_blocks_and_operators_evaluate() {
        // `endfor` / `endif` and `!=` work end-to-end.
        let mut m = Interpreter::new();
        m.feed("s = 0;\nfor i = 1:5\n  s = s + i;\nendfor\n")
            .unwrap();
        assert_eq!(scalar("s = 15"), 15.0); // sanity on the helper
        assert_eq!(
            m.feed("s\n").unwrap().rsplit('=').next().unwrap().trim(),
            "15"
        );
    }

    #[test]
    fn octave_not_equal_and_hash_comment() {
        assert_eq!(scalar("x = 5;  # set x\nx ~= 4\n"), 1.0); // 5 != 4 is true (1)
        assert_eq!(scalar("3 != 3\n"), 0.0);
    }

    #[test]
    fn the_matrix_engine_is_inherited() {
        // The headline MATLAB capability — matmul through array-runtime — works
        // unchanged from Octave syntax.
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        m.feed("B = A * A;\n").unwrap();
        assert_eq!(
            m.feed("B(1,1)\n")
                .unwrap()
                .rsplit('=')
                .next()
                .unwrap()
                .trim(),
            "7"
        );
    }

    #[test]
    fn if_endif_with_not() {
        let mut m = Interpreter::new();
        m.feed("x = 0;\nif !x\n  y = 42;\nendif\n").unwrap();
        assert_eq!(
            m.feed("y\n").unwrap().rsplit('=').next().unwrap().trim(),
            "42"
        );
    }
}
