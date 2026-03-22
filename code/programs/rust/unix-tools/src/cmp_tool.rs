//! # cmp — Compare Two Files Byte by Byte
//!
//! This module implements the business logic for the `cmp` command.
//! Unlike `diff`, which compares files line by line, `cmp` works at
//! the byte level — it finds the exact position where two files
//! first diverge.
//!
//! ## How It Works
//!
//! ```text
//!     File A: [0x48, 0x65, 0x6C, 0x6C, 0x6F]  "Hello"
//!     File B: [0x48, 0x65, 0x78, 0x6C, 0x6F]  "Hexlo"
//!                          ^^^^
//!     cmp reports: byte 3, line 1 differ: 154 170
//! ```
//!
//! The algorithm is simple: read both files simultaneously, byte by
//! byte, and stop at the first mismatch.
//!
//! ## Flags
//!
//! ```text
//!     Flag              Field        Effect
//!     ────────────────  ───────────  ──────────────────────────────
//!     -l, --verbose     verbose      Print byte number and values for ALL
//!                                    differing bytes, not just the first
//!     -s, --silent      silent       Print nothing — only set exit status
//!     -b, --print-bytes print_bytes  Print differing bytes as characters
//!     -i, --ignore-init skip_bytes   Skip N bytes at the start of both files
//!     -n, --bytes       max_bytes    Compare at most N bytes
//! ```

use std::fs::File;
use std::io::{BufReader, Read};

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling how `compare_files` behaves.
#[derive(Debug, Clone, Default)]
pub struct CmpOptions {
    /// Print all differing bytes, not just the first (-l).
    pub verbose: bool,
    /// Suppress all output; only set exit status (-s).
    pub silent: bool,
    /// Also print the differing bytes as characters (-b).
    pub print_bytes: bool,
    /// Skip this many bytes at the start of each file (-i).
    pub skip_bytes: usize,
    /// Compare at most this many bytes (-n). None means compare all.
    pub max_bytes: Option<usize>,
}

// ---------------------------------------------------------------------------
// Result Type
// ---------------------------------------------------------------------------

/// The result of comparing two files.
///
/// ```text
///     Outcome       Meaning
///     ────────────  ──────────────────────────────────────
///     Identical     Files are byte-for-byte identical
///     Differ        Files differ (details in the message)
///     SizeDiffer    One file is a prefix of the other
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum CmpResult {
    /// Files are identical (exit code 0).
    Identical,
    /// Files differ at the given byte offset and line number.
    Differ {
        byte_offset: usize,
        line_number: usize,
        byte_a: u8,
        byte_b: u8,
    },
    /// One file is shorter — it's a prefix of the other.
    Eof {
        shorter_file: String,
    },
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compare two files byte by byte.
///
/// # Algorithm
///
/// ```text
///     1. Open both files with buffered readers
///     2. Skip `skip_bytes` bytes if requested
///     3. Read one byte from each file
///     4. Compare:
///         a. Both EOF → Identical
///         b. One EOF  → Eof (shorter file)
///         c. Bytes differ → Differ (record position)
///         d. Bytes match → continue
///     5. Track byte offset and line number (count newlines)
/// ```
pub fn compare_files(file_a: &str, file_b: &str, opts: &CmpOptions) -> Result<Vec<CmpResult>, String> {
    let f_a = File::open(file_a)
        .map_err(|e| format!("cmp: {}: {}", file_a, e))?;
    let f_b = File::open(file_b)
        .map_err(|e| format!("cmp: {}: {}", file_b, e))?;

    let mut reader_a = BufReader::new(f_a);
    let mut reader_b = BufReader::new(f_b);

    compare_readers(&mut reader_a, &mut reader_b, file_a, file_b, opts)
}

/// Compare two readers byte by byte. This is the testable core.
pub fn compare_readers<R1: Read, R2: Read>(
    reader_a: &mut R1,
    reader_b: &mut R2,
    name_a: &str,
    name_b: &str,
    opts: &CmpOptions,
) -> Result<Vec<CmpResult>, String> {
    // --- Skip initial bytes if requested ---
    if opts.skip_bytes > 0 {
        let mut skip_buf_a = vec![0u8; opts.skip_bytes];
        let mut skip_buf_b = vec![0u8; opts.skip_bytes];
        let _ = reader_a.read(&mut skip_buf_a);
        let _ = reader_b.read(&mut skip_buf_b);
    }

    let mut byte_offset = 1usize; // 1-indexed like cmp
    let mut line_number = 1usize;
    let mut results = Vec::new();
    let max = opts.max_bytes.unwrap_or(usize::MAX);

    let mut buf_a = [0u8; 1];
    let mut buf_b = [0u8; 1];

    loop {
        if byte_offset - 1 >= max {
            break;
        }

        let read_a = reader_a.read(&mut buf_a).map_err(|e| format!("cmp: {}: {}", name_a, e))?;
        let read_b = reader_b.read(&mut buf_b).map_err(|e| format!("cmp: {}: {}", name_b, e))?;

        match (read_a, read_b) {
            (0, 0) => {
                // Both files ended at the same point — identical so far.
                break;
            }
            (0, _) => {
                // File A is shorter
                results.push(CmpResult::Eof {
                    shorter_file: name_a.to_string(),
                });
                break;
            }
            (_, 0) => {
                // File B is shorter
                results.push(CmpResult::Eof {
                    shorter_file: name_b.to_string(),
                });
                break;
            }
            _ => {
                if buf_a[0] != buf_b[0] {
                    results.push(CmpResult::Differ {
                        byte_offset,
                        line_number,
                        byte_a: buf_a[0],
                        byte_b: buf_b[0],
                    });

                    // In non-verbose mode, stop at the first difference.
                    if !opts.verbose {
                        return Ok(results);
                    }
                }

                // Track line numbers — newlines increment the counter.
                if buf_a[0] == b'\n' {
                    line_number += 1;
                }

                byte_offset += 1;
            }
        }
    }

    if results.is_empty() {
        results.push(CmpResult::Identical);
    }

    Ok(results)
}

/// Format a CmpResult into the standard cmp output string.
///
/// ```text
///     CmpResult::Differ { byte_offset: 3, line_number: 1, byte_a: 0x6C, byte_b: 0x78 }
///     → "file_a file_b differ: byte 3, line 1"
///
///     CmpResult::Eof { shorter_file: "file_a" }
///     → "cmp: EOF on file_a after byte 2"
/// ```
pub fn format_result(result: &CmpResult, name_a: &str, name_b: &str, opts: &CmpOptions) -> String {
    if opts.silent {
        return String::new();
    }

    match result {
        CmpResult::Identical => String::new(),
        CmpResult::Differ { byte_offset, line_number, byte_a, byte_b } => {
            if opts.verbose {
                if opts.print_bytes {
                    format!(
                        "{:>5} {:3o} {:>3}   {:3o} {:>3}",
                        byte_offset,
                        byte_a,
                        format_byte_char(*byte_a),
                        byte_b,
                        format_byte_char(*byte_b),
                    )
                } else {
                    format!("{:>5} {:3o} {:3o}", byte_offset, byte_a, byte_b)
                }
            } else {
                format!(
                    "{} {} differ: byte {}, line {}",
                    name_a, name_b, byte_offset, line_number
                )
            }
        }
        CmpResult::Eof { shorter_file } => {
            format!("cmp: EOF on {}", shorter_file)
        }
    }
}

/// Format a byte as a printable character, or as an escape sequence.
fn format_byte_char(b: u8) -> String {
    if b.is_ascii_graphic() || b == b' ' {
        format!("{}", b as char)
    } else if b == b'\n' {
        "\\n".to_string()
    } else if b == b'\t' {
        "\\t".to_string()
    } else if b == b'\r' {
        "\\r".to_string()
    } else {
        format!("\\{:03o}", b)
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn identical_bytes() {
        let mut a = Cursor::new(b"hello");
        let mut b = Cursor::new(b"hello");
        let results = compare_readers(&mut a, &mut b, "a", "b", &CmpOptions::default()).unwrap();
        assert_eq!(results, vec![CmpResult::Identical]);
    }

    #[test]
    fn first_byte_differs() {
        let mut a = Cursor::new(b"abc");
        let mut b = Cursor::new(b"xbc");
        let results = compare_readers(&mut a, &mut b, "a", "b", &CmpOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            CmpResult::Differ { byte_offset, byte_a, byte_b, .. } => {
                assert_eq!(*byte_offset, 1);
                assert_eq!(*byte_a, b'a');
                assert_eq!(*byte_b, b'x');
            }
            _ => panic!("Expected Differ"),
        }
    }

    #[test]
    fn third_byte_differs() {
        let mut a = Cursor::new(b"abcde");
        let mut b = Cursor::new(b"abxde");
        let results = compare_readers(&mut a, &mut b, "a", "b", &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Differ { byte_offset, .. } => assert_eq!(*byte_offset, 3),
            _ => panic!("Expected Differ"),
        }
    }

    #[test]
    fn eof_shorter_file() {
        let mut a = Cursor::new(b"ab");
        let mut b = Cursor::new(b"abcd");
        let results = compare_readers(&mut a, &mut b, "short", "long", &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Eof { shorter_file } => assert_eq!(shorter_file, "short"),
            _ => panic!("Expected Eof"),
        }
    }

    #[test]
    fn verbose_reports_all_differences() {
        let mut a = Cursor::new(b"abc");
        let mut b = Cursor::new(b"xyx");
        let opts = CmpOptions { verbose: true, ..Default::default() };
        let results = compare_readers(&mut a, &mut b, "a", "b", &opts).unwrap();
        // All 3 bytes differ
        let diff_count = results.iter().filter(|r| matches!(r, CmpResult::Differ { .. })).count();
        assert_eq!(diff_count, 3);
    }

    #[test]
    fn max_bytes_limits_comparison() {
        let mut a = Cursor::new(b"abcXX");
        let mut b = Cursor::new(b"abcYY");
        let opts = CmpOptions { max_bytes: Some(3), ..Default::default() };
        let results = compare_readers(&mut a, &mut b, "a", "b", &opts).unwrap();
        assert_eq!(results, vec![CmpResult::Identical]);
    }

    #[test]
    fn line_number_tracking() {
        let mut a = Cursor::new(b"line1\nline2\nX");
        let mut b = Cursor::new(b"line1\nline2\nY");
        let results = compare_readers(&mut a, &mut b, "a", "b", &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Differ { line_number, .. } => assert_eq!(*line_number, 3),
            _ => panic!("Expected Differ"),
        }
    }

    #[test]
    fn format_differ_result() {
        let result = CmpResult::Differ {
            byte_offset: 5,
            line_number: 2,
            byte_a: b'a',
            byte_b: b'b',
        };
        let output = format_result(&result, "file1", "file2", &CmpOptions::default());
        assert!(output.contains("file1 file2 differ: byte 5, line 2"));
    }

    #[test]
    fn format_eof_result() {
        let result = CmpResult::Eof { shorter_file: "small.txt".to_string() };
        let output = format_result(&result, "small.txt", "big.txt", &CmpOptions::default());
        assert!(output.contains("EOF on small.txt"));
    }

    #[test]
    fn silent_mode_produces_no_output() {
        let result = CmpResult::Differ { byte_offset: 1, line_number: 1, byte_a: 0, byte_b: 1 };
        let opts = CmpOptions { silent: true, ..Default::default() };
        let output = format_result(&result, "a", "b", &opts);
        assert!(output.is_empty());
    }

    #[test]
    fn empty_files_are_identical() {
        let mut a = Cursor::new(b"");
        let mut b = Cursor::new(b"");
        let results = compare_readers(&mut a, &mut b, "a", "b", &CmpOptions::default()).unwrap();
        assert_eq!(results, vec![CmpResult::Identical]);
    }

    #[test]
    fn format_byte_char_printable() {
        assert_eq!(format_byte_char(b'A'), "A");
        assert_eq!(format_byte_char(b' '), " ");
    }

    #[test]
    fn format_byte_char_special() {
        assert_eq!(format_byte_char(b'\n'), "\\n");
        assert_eq!(format_byte_char(b'\t'), "\\t");
    }
}
