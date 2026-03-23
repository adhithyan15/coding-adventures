//! # split — Split a File into Pieces
//!
//! This module implements the business logic for the `split` command.
//! `split` breaks a single file into multiple smaller files, each
//! containing a fixed number of lines (or bytes).
//!
//! ## How It Works
//!
//! ```text
//!     Input file (10 lines):          Output files (3 lines each):
//!     ┌──────────────────┐            ┌──────────────┐
//!     │ line 1           │            │ xaa          │ ← lines 1-3
//!     │ line 2           │            │ xab          │ ← lines 4-6
//!     │ line 3           │            │ xac          │ ← lines 7-9
//!     │ line 4           │            │ xad          │ ← line 10
//!     │ line 5           │            └──────────────┘
//!     │ line 6           │
//!     │ line 7           │
//!     │ line 8           │
//!     │ line 9           │
//!     │ line 10          │
//!     └──────────────────┘
//! ```
//!
//! ## Naming Convention
//!
//! Output files are named `PREFIXaa`, `PREFIXab`, ..., `PREFIXzz`.
//! The default prefix is `x`, giving `xaa`, `xab`, etc.
//!
//! With `-d` (numeric suffixes): `x00`, `x01`, ..., `x99`.
//!
//! The suffix length can be changed with `-a N`.

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options that control how `split_by_lines` behaves.
///
/// ```text
///     Flag              Field              Effect
///     ──────────────    ─────────────────  ──────────────────────────────
///     -a N              suffix_length      Suffix length (default 2)
///     -d                numeric_suffixes   Use 00, 01, ... instead of aa, ab, ...
///     --additional-suffix SUFFIX  additional_suffix  Append to file names
/// ```
#[derive(Debug, Clone)]
pub struct SplitOptions {
    /// Length of the alphabetic/numeric suffix (default 2).
    pub suffix_length: usize,
    /// Use numeric suffixes (00, 01, ...) instead of alphabetic (aa, ab, ...).
    pub numeric_suffixes: bool,
    /// Additional suffix to append after the generated suffix.
    pub additional_suffix: String,
}

impl Default for SplitOptions {
    fn default() -> Self {
        SplitOptions {
            suffix_length: 2,
            numeric_suffixes: false,
            additional_suffix: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Split content into chunks of `n` lines each.
///
/// # Parameters
///
/// - `content`: the full text to split
/// - `n`: number of lines per output chunk
/// - `prefix`: the filename prefix for output files (e.g., "x")
/// - `opts`: additional options (suffix length, numeric vs alpha)
///
/// # Returns
///
/// A vector of `(filename, chunk_content)` pairs. Each pair represents
/// one output file. The filename is generated from the prefix and
/// a sequential suffix.
///
/// # Algorithm
///
/// ```text
///     split_by_lines(content, n=3, prefix="x"):
///         lines = content.split('\n')
///         chunks = group lines into groups of n
///         for (i, chunk) in chunks:
///             suffix = generate_suffix(i)
///             filename = prefix + suffix
///             content = chunk.join('\n')
///             emit (filename, content)
/// ```
///
/// # Example
///
/// ```text
///     content = "a\nb\nc\nd\ne"
///     n = 2, prefix = "x"
///     result = [
///         ("xaa", "a\nb"),
///         ("xab", "c\nd"),
///         ("xac", "e"),
///     ]
/// ```
pub fn split_by_lines(
    content: &str,
    n: usize,
    prefix: &str,
    opts: &SplitOptions,
) -> Vec<(String, String)> {
    if n == 0 {
        return Vec::new();
    }

    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut chunk_index = 0;

    // --- Group lines into chunks of n ---
    for chunk in lines.chunks(n) {
        let suffix = generate_suffix(chunk_index, opts.suffix_length, opts.numeric_suffixes);
        let filename = format!("{}{}{}", prefix, suffix, opts.additional_suffix);
        let chunk_content = chunk.join("\n");
        result.push((filename, chunk_content));
        chunk_index += 1;
    }

    result
}

/// Split content into chunks of `n` bytes each.
///
/// Similar to `split_by_lines`, but splits on byte boundaries
/// instead of line boundaries.
///
/// # Returns
///
/// A vector of `(filename, chunk_content)` pairs.
pub fn split_by_bytes(
    content: &str,
    n: usize,
    prefix: &str,
    opts: &SplitOptions,
) -> Vec<(String, String)> {
    if n == 0 || content.is_empty() {
        return Vec::new();
    }

    let bytes = content.as_bytes();
    let mut result = Vec::new();
    let mut chunk_index = 0;
    let mut offset = 0;

    while offset < bytes.len() {
        let end = std::cmp::min(offset + n, bytes.len());
        let chunk = String::from_utf8_lossy(&bytes[offset..end]).into_owned();
        let suffix = generate_suffix(chunk_index, opts.suffix_length, opts.numeric_suffixes);
        let filename = format!("{}{}{}", prefix, suffix, opts.additional_suffix);
        result.push((filename, chunk));
        offset = end;
        chunk_index += 1;
    }

    result
}

/// Split content into exactly `n` chunks (as equal as possible).
///
/// This implements the `split -n N` behavior: divide the content
/// into N roughly equal parts.
pub fn split_into_chunks(
    content: &str,
    n: usize,
    prefix: &str,
    opts: &SplitOptions,
) -> Vec<(String, String)> {
    if n == 0 || content.is_empty() {
        return Vec::new();
    }

    let total_bytes = content.len();
    let chunk_size = total_bytes / n;
    let remainder = total_bytes % n;

    let mut result = Vec::new();
    let mut offset = 0;

    for i in 0..n {
        // Distribute remainder bytes across the first 'remainder' chunks
        let this_chunk_size = chunk_size + if i < remainder { 1 } else { 0 };
        if offset >= content.len() {
            break;
        }
        let end = std::cmp::min(offset + this_chunk_size, content.len());
        let chunk = content[offset..end].to_string();
        let suffix = generate_suffix(i, opts.suffix_length, opts.numeric_suffixes);
        let filename = format!("{}{}{}", prefix, suffix, opts.additional_suffix);
        result.push((filename, chunk));
        offset = end;
    }

    result
}

// ---------------------------------------------------------------------------
// Suffix Generation
// ---------------------------------------------------------------------------

/// Generate a suffix string for the given chunk index.
///
/// ## Alphabetic Suffixes (default)
///
/// With suffix_length=2, the sequence is:
/// ```text
///     0 → "aa", 1 → "ab", ..., 25 → "az",
///     26 → "ba", 27 → "bb", ..., 675 → "zz"
/// ```
///
/// This is essentially a base-26 number system using letters a-z.
///
/// ## Numeric Suffixes (-d)
///
/// With suffix_length=2, the sequence is:
/// ```text
///     0 → "00", 1 → "01", ..., 99 → "99"
/// ```
///
/// This is a zero-padded decimal number.
fn generate_suffix(index: usize, length: usize, numeric: bool) -> String {
    if numeric {
        // --- Numeric suffix: zero-padded decimal ---
        format!("{:0>width$}", index, width = length)
    } else {
        // --- Alphabetic suffix: base-26 using a-z ---
        // Convert the index to a base-26 representation.
        //
        // Example with length=2:
        //   0 → (0, 0) → "aa"
        //   1 → (0, 1) → "ab"
        //  26 → (1, 0) → "ba"
        // 675 → (25, 25) → "zz"
        let mut result = Vec::with_capacity(length);
        let mut remaining = index;

        for _ in 0..length {
            let digit = remaining % 26;
            result.push((b'a' + digit as u8) as char);
            remaining /= 26;
        }

        // We built the suffix least-significant-first, so reverse it
        result.reverse();
        result.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Suffix generation ---

    #[test]
    fn alphabetic_suffix_basic() {
        assert_eq!(generate_suffix(0, 2, false), "aa");
        assert_eq!(generate_suffix(1, 2, false), "ab");
        assert_eq!(generate_suffix(25, 2, false), "az");
        assert_eq!(generate_suffix(26, 2, false), "ba");
    }

    #[test]
    fn numeric_suffix_basic() {
        assert_eq!(generate_suffix(0, 2, true), "00");
        assert_eq!(generate_suffix(1, 2, true), "01");
        assert_eq!(generate_suffix(42, 2, true), "42");
    }

    #[test]
    fn suffix_length_3() {
        assert_eq!(generate_suffix(0, 3, false), "aaa");
        assert_eq!(generate_suffix(0, 3, true), "000");
    }

    // --- split_by_lines ---

    #[test]
    fn split_basic() {
        let content = "a\nb\nc\nd\ne";
        let result = split_by_lines(content, 2, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("xaa".to_string(), "a\nb".to_string()));
        assert_eq!(result[1], ("xab".to_string(), "c\nd".to_string()));
        assert_eq!(result[2], ("xac".to_string(), "e".to_string()));
    }

    #[test]
    fn split_exact_division() {
        let content = "a\nb\nc\nd";
        let result = split_by_lines(content, 2, "x", &SplitOptions::default());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, "a\nb");
        assert_eq!(result[1].1, "c\nd");
    }

    #[test]
    fn split_one_line_per_file() {
        let content = "a\nb\nc";
        let result = split_by_lines(content, 1, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("xaa".to_string(), "a".to_string()));
        assert_eq!(result[1], ("xab".to_string(), "b".to_string()));
        assert_eq!(result[2], ("xac".to_string(), "c".to_string()));
    }

    #[test]
    fn split_all_in_one() {
        let content = "a\nb\nc";
        let result = split_by_lines(content, 100, "x", &SplitOptions::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "a\nb\nc");
    }

    #[test]
    fn split_empty_content() {
        let result = split_by_lines("", 5, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_zero_lines() {
        let result = split_by_lines("a\nb\nc", 0, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_custom_prefix() {
        let content = "a\nb";
        let result = split_by_lines(content, 1, "output_", &SplitOptions::default());
        assert_eq!(result[0].0, "output_aa");
        assert_eq!(result[1].0, "output_ab");
    }

    #[test]
    fn split_numeric_suffixes() {
        let content = "a\nb\nc";
        let opts = SplitOptions { numeric_suffixes: true, ..Default::default() };
        let result = split_by_lines(content, 1, "x", &opts);
        assert_eq!(result[0].0, "x00");
        assert_eq!(result[1].0, "x01");
        assert_eq!(result[2].0, "x02");
    }

    #[test]
    fn split_additional_suffix() {
        let content = "a\nb";
        let opts = SplitOptions {
            additional_suffix: ".txt".to_string(),
            ..Default::default()
        };
        let result = split_by_lines(content, 1, "x", &opts);
        assert_eq!(result[0].0, "xaa.txt");
        assert_eq!(result[1].0, "xab.txt");
    }

    #[test]
    fn split_suffix_length() {
        let content = "a\nb";
        let opts = SplitOptions { suffix_length: 3, ..Default::default() };
        let result = split_by_lines(content, 1, "x", &opts);
        assert_eq!(result[0].0, "xaaa");
        assert_eq!(result[1].0, "xaab");
    }

    // --- split_by_bytes ---

    #[test]
    fn split_bytes_basic() {
        let content = "abcdefgh";
        let result = split_by_bytes(content, 3, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("xaa".to_string(), "abc".to_string()));
        assert_eq!(result[1], ("xab".to_string(), "def".to_string()));
        assert_eq!(result[2], ("xac".to_string(), "gh".to_string()));
    }

    #[test]
    fn split_bytes_empty() {
        let result = split_by_bytes("", 5, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    // --- split_into_chunks ---

    #[test]
    fn split_chunks_basic() {
        let content = "abcdefghij"; // 10 bytes
        let result = split_into_chunks(content, 3, "x", &SplitOptions::default());
        assert_eq!(result.len(), 3);
        // 10 / 3 = 3 remainder 1, so first chunk gets 4 bytes
        assert_eq!(result[0].1.len(), 4); // "abcd"
        assert_eq!(result[1].1.len(), 3); // "efg"
        assert_eq!(result[2].1.len(), 3); // "hij"
    }

    #[test]
    fn split_chunks_empty() {
        let result = split_into_chunks("", 3, "x", &SplitOptions::default());
        assert!(result.is_empty());
    }

    #[test]
    fn split_chunks_single() {
        let content = "hello";
        let result = split_into_chunks(content, 1, "x", &SplitOptions::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, "hello");
    }
}
