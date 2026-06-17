//! # Octave REPL — an interactive Read-Eval-Print loop for GNU Octave.
//!
//! [`OctaveRepl`] wraps a persistent [`Interpreter`] (which normalizes Octave
//! syntax to MATLAB and delegates to `matlab-runtime`) and adds the console
//! behaviours: line continuation across open brackets and unterminated blocks —
//! recognising both `end` and Octave's `endif`/`endfor`/`endwhile`/`endfunction`/
//! `endswitch`/`end_try_catch` terminators — plus `#`/`%` comments and `"`/`'`
//! strings. Sibling of `matlab-repl`.

use coding_adventures_octave_runtime::Interpreter;
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    Output(String),
    NeedMore,
    Quit,
}

/// A persistent interactive Octave session.
pub struct OctaveRepl {
    interp: Interpreter,
    buffer: String,
}

impl Default for OctaveRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl OctaveRepl {
    pub fn new() -> Self {
        OctaveRepl {
            interp: Interpreter::new(),
            buffer: String::new(),
        }
    }

    pub fn prompt(&self) -> &'static str {
        if self.buffer.is_empty() {
            "octave> "
        } else {
            "... "
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "exit" | "quit()" | "exit()" => return ReplResponse::Quit,
                _ => {}
            }
        }
        self.buffer.push_str(line);
        self.buffer.push('\n');

        if is_incomplete(&self.buffer) {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        if src.trim().is_empty() {
            return ReplResponse::Output(String::new());
        }
        match self.interp.feed(&src) {
            Ok(text) => ReplResponse::Output(text),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Is the accumulated source incomplete — an open bracket, an unterminated
/// `"`-string, or a block keyword without its `end`/`endX`? `#` and `%` line
/// comments are skipped; `'` is treated as transpose (char arrays are
/// single-line).
fn is_incomplete(src: &str) -> bool {
    let mut bracket: i32 = 0;
    let mut blocks: i32 = 0;
    let mut in_string = false;

    for raw_line in src.lines() {
        let mut word = String::new();
        let flush = |w: &mut String, blocks: &mut i32, bracket: i32| {
            if !w.is_empty() {
                match w.as_str() {
                    "if" | "for" | "while" | "switch" | "try" | "function" | "parfor" | "do" => {
                        *blocks += 1
                    }
                    // Any of these closes a block — but only at bracket depth 0,
                    // since a bare `end` inside `( )` is the index sentinel.
                    "end" | "endif" | "endfor" | "endwhile" | "endfunction" | "endswitch"
                    | "endparfor" | "end_try_catch" | "until"
                        if bracket == 0 =>
                    {
                        *blocks -= 1
                    }
                    _ => {}
                }
                w.clear();
            }
        };
        for ch in raw_line.chars() {
            if in_string {
                if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '%' | '#' => break, // line comment
                '"' => {
                    flush(&mut word, &mut blocks, bracket);
                    in_string = true;
                }
                '(' | '[' | '{' => {
                    flush(&mut word, &mut blocks, bracket);
                    bracket += 1;
                }
                ')' | ']' | '}' => {
                    flush(&mut word, &mut blocks, bracket);
                    bracket -= 1;
                }
                c if c.is_alphanumeric() || c == '_' => word.push(c),
                _ => flush(&mut word, &mut blocks, bracket),
            }
        }
        flush(&mut word, &mut blocks, bracket);
    }
    bracket > 0 || blocks > 0 || in_string
}

/// Drive a full interactive Octave session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = OctaveRepl::new();
    writeln!(
        writer,
        "GNU Octave (on the MATLAB stack) — type quit to exit."
    )?;

    loop {
        write!(writer, "{}", repl.prompt())?;
        writer.flush()?;

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            writeln!(writer)?;
            break;
        }
        let line = line.strip_suffix('\n').unwrap_or(&line);
        let line = line.strip_suffix('\r').unwrap_or(line);

        match repl.feed(line) {
            ReplResponse::Output(text) => {
                write!(writer, "{text}")?;
                writer.flush()?;
            }
            ReplResponse::NeedMore => {}
            ReplResponse::Quit => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continues_across_an_endfor_block() {
        let mut r = OctaveRepl::new();
        assert_eq!(r.feed("s = 0;"), ReplResponse::Output(String::new()));
        assert_eq!(r.feed("for i = 1:3"), ReplResponse::NeedMore);
        assert_eq!(r.feed("  s = s + i;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("endfor"), ReplResponse::Output(_))); // endfor closes it
        assert!(matches!(r.feed("s"), ReplResponse::Output(t) if t.contains("6")));
    }

    #[test]
    fn if_endif_with_bang_not() {
        let mut r = OctaveRepl::new();
        assert_eq!(r.feed("x = 0;"), ReplResponse::Output(String::new()));
        assert_eq!(r.feed("if !x"), ReplResponse::NeedMore);
        assert_eq!(r.feed("  y = 7;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("endif"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("y"), ReplResponse::Output(t) if t.contains("7")));
    }

    #[test]
    fn hash_comment_and_brackets() {
        let mut r = OctaveRepl::new();
        assert_eq!(r.feed("A = [1 2  # a row"), ReplResponse::NeedMore); // open bracket
        assert!(matches!(r.feed("3 4];"), ReplResponse::Output(_)));
    }

    #[test]
    fn quit_and_errors() {
        assert_eq!(OctaveRepl::new().feed("quit"), ReplResponse::Quit);
        let mut r = OctaveRepl::new();
        assert!(matches!(r.feed("nope"), ReplResponse::Output(t) if t.contains("Error")));
        assert!(matches!(r.feed("1 + 1"), ReplResponse::Output(t) if t.contains("2")));
    }

    #[test]
    fn run_drives_a_session() {
        let input = b"x = 3;\nx != 2\nquit\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains('1')); // 3 != 2 is true
    }
}
