//! # echo — Display a Line of Text
//!
//! This module implements the business logic for the `echo` command.
//! The `echo` utility writes its arguments to standard output, separated
//! by single spaces, followed by a newline — unless `-n` suppresses it.
//!
//! ## Escape Sequences
//!
//! When `-e` is passed, `echo` interprets the following backslash escapes
//! within each argument string:
//!
//! ```text
//!     \\     backslash
//!     \a     alert (bell)
//!     \b     backspace
//!     \c     produce no further output (stops immediately)
//!     \f     form feed
//!     \n     new line
//!     \r     carriage return
//!     \t     horizontal tab
//!     \v     vertical tab
//!     \0nnn  byte with octal value nnn (1 to 3 digits)
//!     \xHH   byte with hexadecimal value HH (1 to 2 digits)
//! ```
//!
//! By default (`-E`), no escape interpretation is performed. The strings
//! are printed exactly as given.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Process echo arguments and return the resulting output string.
///
/// This function takes the list of positional arguments (the strings to
/// echo), along with two boolean flags:
///
/// - `no_newline`: if true, suppress the trailing newline (the `-n` flag)
/// - `enable_escapes`: if true, interpret backslash escapes (the `-e` flag)
///
/// # Returns
///
/// A `String` containing the text to be printed. The caller is responsible
/// for writing it to stdout.
///
/// # Example
///
/// ```text
///     process_echo(&["hello".into(), "world".into()], false, false)
///     // => "hello world\n"
///
///     process_echo(&["hello".into()], true, false)
///     // => "hello"   (no trailing newline)
///
///     process_echo(&["hello\\tworld".into()], false, true)
///     // => "hello\tworld\n"   (tab is expanded)
/// ```
pub fn process_echo(args: &[String], no_newline: bool, enable_escapes: bool) -> String {
    // --- Step 1: Join the arguments with spaces ---
    // The POSIX spec says arguments are separated by single spaces.
    // This is the simplest part of echo.
    let joined = args.join(" ");

    // --- Step 2: Optionally interpret escape sequences ---
    // If `-e` was passed, we scan the string for backslash sequences
    // and replace them with their meaning. If `-E` (or neither) was
    // passed, we leave the string exactly as-is.
    let processed = if enable_escapes {
        interpret_escapes(&joined)
    } else {
        EscapeResult {
            text: joined,
            stop: false,
        }
    };

    // --- Step 3: Optionally append a newline ---
    // By default, echo adds a trailing newline. The `-n` flag suppresses it.
    // If the `\c` escape was encountered during processing, we also
    // suppress the newline — `\c` means "stop all output here."
    let mut output = processed.text;
    if !no_newline && !processed.stop {
        output.push('\n');
    }

    output
}

// ---------------------------------------------------------------------------
// Escape sequence interpreter
// ---------------------------------------------------------------------------

/// The result of interpreting escape sequences in a string.
///
/// The `stop` field indicates whether a `\c` escape was encountered.
/// When `\c` appears, all output after it (including the trailing newline)
/// is suppressed.
struct EscapeResult {
    text: String,
    stop: bool,
}

/// Interpret backslash escape sequences in a string.
///
/// This implements the escape sequences defined by POSIX for `echo -e`.
/// We process the string character by character, building up a result
/// string. When we encounter a backslash, we look at the next character
/// to determine what to substitute.
///
/// ## The State Machine
///
/// ```text
///     Normal char  ──────►  append to output
///     '\\' char    ──────►  look ahead:
///        'n'  → newline       't'  → tab
///        'a'  → bell          'b'  → backspace
///        'f'  → form feed     'r'  → carriage return
///        'v'  → vertical tab  '\\' → literal backslash
///        'c'  → STOP (return immediately)
///        '0'  → read up to 3 octal digits → byte
///        'x'  → read up to 2 hex digits   → byte
///        other → keep the backslash and char as-is
/// ```
fn interpret_escapes(input: &str) -> EscapeResult {
    let mut output = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\\' && i + 1 < len {
            // We found a backslash — look at the next character.
            i += 1;
            match chars[i] {
                // --- Simple single-character escapes ---
                '\\' => output.push('\\'),
                'a' => output.push('\x07'),  // BEL (alert/bell)
                'b' => output.push('\x08'),  // BS  (backspace)
                'f' => output.push('\x0C'),  // FF  (form feed)
                'n' => output.push('\n'),     // LF  (newline)
                'r' => output.push('\r'),     // CR  (carriage return)
                't' => output.push('\t'),     // HT  (horizontal tab)
                'v' => output.push('\x0B'),   // VT  (vertical tab)

                // --- \c: stop all output ---
                // This is the "produce no further output" escape.
                // We return immediately with what we have so far.
                'c' => {
                    return EscapeResult {
                        text: output,
                        stop: true,
                    };
                }

                // --- \0nnn: octal byte value ---
                // Read up to 3 octal digits after the '0'.
                '0' => {
                    let mut octal = String::new();
                    let mut j = i + 1;
                    while j < len && j < i + 4 && chars[j].is_digit(8) {
                        octal.push(chars[j]);
                        j += 1;
                    }
                    if octal.is_empty() {
                        // \0 with no digits means null byte
                        output.push('\0');
                    } else {
                        let value = u8::from_str_radix(&octal, 8).unwrap_or(0);
                        output.push(value as char);
                    }
                    i = j - 1; // -1 because the loop will increment
                }

                // --- \xHH: hexadecimal byte value ---
                // Read up to 2 hex digits after the 'x'.
                'x' => {
                    let mut hex = String::new();
                    let mut j = i + 1;
                    while j < len && j < i + 3 && chars[j].is_ascii_hexdigit() {
                        hex.push(chars[j]);
                        j += 1;
                    }
                    if hex.is_empty() {
                        // \x with no digits: keep literal
                        output.push('\\');
                        output.push('x');
                    } else {
                        let value = u8::from_str_radix(&hex, 16).unwrap_or(0);
                        output.push(value as char);
                    }
                    i = j - 1;
                }

                // --- Unknown escape: keep both characters ---
                other => {
                    output.push('\\');
                    output.push(other);
                }
            }
        } else {
            // Normal character — just append it.
            output.push(chars[i]);
        }
        i += 1;
    }

    EscapeResult {
        text: output,
        stop: false,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_echo() {
        let args: Vec<String> = vec!["hello".into(), "world".into()];
        assert_eq!(process_echo(&args, false, false), "hello world\n");
    }

    #[test]
    fn no_newline_flag() {
        let args: Vec<String> = vec!["hello".into()];
        assert_eq!(process_echo(&args, true, false), "hello");
    }

    #[test]
    fn empty_args() {
        let args: Vec<String> = vec![];
        assert_eq!(process_echo(&args, false, false), "\n");
    }

    #[test]
    fn escape_tab() {
        let args: Vec<String> = vec!["hello\\tworld".into()];
        assert_eq!(process_echo(&args, false, true), "hello\tworld\n");
    }

    #[test]
    fn escape_newline() {
        let args: Vec<String> = vec!["line1\\nline2".into()];
        assert_eq!(process_echo(&args, false, true), "line1\nline2\n");
    }

    #[test]
    fn escape_c_stops_output() {
        let args: Vec<String> = vec!["hello\\cworld".into()];
        // \c stops output — no trailing newline, no "world"
        assert_eq!(process_echo(&args, false, true), "hello");
    }

    #[test]
    fn escape_backslash() {
        let args: Vec<String> = vec!["back\\\\slash".into()];
        assert_eq!(process_echo(&args, false, true), "back\\slash\n");
    }

    #[test]
    fn escape_octal() {
        // \0101 is octal for 'A' (65)
        let args: Vec<String> = vec!["\\0101".into()];
        assert_eq!(process_echo(&args, false, true), "A\n");
    }

    #[test]
    fn escape_hex() {
        // \x41 is hex for 'A' (65)
        let args: Vec<String> = vec!["\\x41".into()];
        assert_eq!(process_echo(&args, false, true), "A\n");
    }

    #[test]
    fn escapes_not_interpreted_by_default() {
        let args: Vec<String> = vec!["hello\\tworld".into()];
        assert_eq!(process_echo(&args, false, false), "hello\\tworld\n");
    }
}
