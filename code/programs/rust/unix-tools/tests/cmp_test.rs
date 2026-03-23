//! # Integration Tests for cmp
//!
//! These tests verify the byte-by-byte comparison logic using
//! real files on the filesystem.

use unix_tools::cmp_tool::*;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("cmp_integ_{}", name))
        .to_string_lossy()
        .into_owned()
}

fn cleanup(path: &str) {
    let _ = fs::remove_file(Path::new(path));
}

// ---------------------------------------------------------------------------
// Tests: Identical files
// ---------------------------------------------------------------------------

#[cfg(test)]
mod identical {
    use super::*;

    #[test]
    fn identical_files() {
        let a = temp_path("ident_a");
        let b = temp_path("ident_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "hello world").unwrap();
        fs::write(&b, "hello world").unwrap();

        let results = compare_files(&a, &b, &CmpOptions::default()).unwrap();
        assert_eq!(results, vec![CmpResult::Identical]);

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn empty_files_identical() {
        let a = temp_path("empty_a");
        let b = temp_path("empty_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "").unwrap();
        fs::write(&b, "").unwrap();

        let results = compare_files(&a, &b, &CmpOptions::default()).unwrap();
        assert_eq!(results, vec![CmpResult::Identical]);

        cleanup(&a);
        cleanup(&b);
    }
}

// ---------------------------------------------------------------------------
// Tests: Different files
// ---------------------------------------------------------------------------

#[cfg(test)]
mod different {
    use super::*;

    #[test]
    fn first_byte_difference() {
        let a = temp_path("first_a");
        let b = temp_path("first_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "abc").unwrap();
        fs::write(&b, "xbc").unwrap();

        let results = compare_files(&a, &b, &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Differ { byte_offset, .. } => assert_eq!(*byte_offset, 1),
            _ => panic!("Expected Differ"),
        }

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn middle_byte_difference() {
        let a = temp_path("mid_a");
        let b = temp_path("mid_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "abcde").unwrap();
        fs::write(&b, "abXde").unwrap();

        let results = compare_files(&a, &b, &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Differ { byte_offset, byte_a, byte_b, .. } => {
                assert_eq!(*byte_offset, 3);
                assert_eq!(*byte_a, b'c');
                assert_eq!(*byte_b, b'X');
            }
            _ => panic!("Expected Differ"),
        }

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn different_lengths_shorter_a() {
        let a = temp_path("short_a");
        let b = temp_path("short_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "ab").unwrap();
        fs::write(&b, "abcd").unwrap();

        let results = compare_files(&a, &b, &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Eof { shorter_file } => {
                assert!(shorter_file.contains("short_a"));
            }
            _ => panic!("Expected Eof"),
        }

        cleanup(&a);
        cleanup(&b);
    }
}

// ---------------------------------------------------------------------------
// Tests: Options
// ---------------------------------------------------------------------------

#[cfg(test)]
mod options {
    use super::*;

    #[test]
    fn verbose_reports_all_differences() {
        let a = temp_path("verbose_a");
        let b = temp_path("verbose_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "abc").unwrap();
        fs::write(&b, "xyz").unwrap();

        let opts = CmpOptions { verbose: true, ..Default::default() };
        let results = compare_files(&a, &b, &opts).unwrap();
        let diff_count = results.iter()
            .filter(|r| matches!(r, CmpResult::Differ { .. }))
            .count();
        assert_eq!(diff_count, 3);

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn max_bytes_limits_comparison() {
        let a = temp_path("max_a");
        let b = temp_path("max_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "abcXX").unwrap();
        fs::write(&b, "abcYY").unwrap();

        let opts = CmpOptions { max_bytes: Some(3), ..Default::default() };
        let results = compare_files(&a, &b, &opts).unwrap();
        assert_eq!(results, vec![CmpResult::Identical]);

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn silent_mode_formatting() {
        let result = CmpResult::Differ {
            byte_offset: 1,
            line_number: 1,
            byte_a: b'a',
            byte_b: b'b',
        };
        let opts = CmpOptions { silent: true, ..Default::default() };
        let output = format_result(&result, "a", "b", &opts);
        assert!(output.is_empty());
    }

    #[test]
    fn nonexistent_file_error() {
        let result = compare_files(
            "/tmp/cmp_integ_noexist",
            "/tmp/cmp_integ_noexist2",
            &CmpOptions::default(),
        );
        assert!(result.is_err());
    }
}

// ---------------------------------------------------------------------------
// Tests: Line number tracking
// ---------------------------------------------------------------------------

#[cfg(test)]
mod line_tracking {
    use super::*;

    #[test]
    fn tracks_line_numbers_correctly() {
        let a = temp_path("line_a");
        let b = temp_path("line_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "line1\nline2\nline3X").unwrap();
        fs::write(&b, "line1\nline2\nline3Y").unwrap();

        let results = compare_files(&a, &b, &CmpOptions::default()).unwrap();
        match &results[0] {
            CmpResult::Differ { line_number, .. } => assert_eq!(*line_number, 3),
            _ => panic!("Expected Differ"),
        }

        cleanup(&a);
        cleanup(&b);
    }
}

// ---------------------------------------------------------------------------
// Tests: Output formatting
// ---------------------------------------------------------------------------

#[cfg(test)]
mod formatting {
    use super::*;

    #[test]
    fn format_differ_message() {
        let result = CmpResult::Differ {
            byte_offset: 10,
            line_number: 3,
            byte_a: b'x',
            byte_b: b'y',
        };
        let output = format_result(&result, "file1.txt", "file2.txt", &CmpOptions::default());
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
        assert!(output.contains("byte 10"));
        assert!(output.contains("line 3"));
    }

    #[test]
    fn format_eof_message() {
        let result = CmpResult::Eof { shorter_file: "small.txt".into() };
        let output = format_result(&result, "small.txt", "big.txt", &CmpOptions::default());
        assert!(output.contains("EOF on small.txt"));
    }

    #[test]
    fn format_identical_is_empty() {
        let result = CmpResult::Identical;
        let output = format_result(&result, "a", "b", &CmpOptions::default());
        assert!(output.is_empty());
    }
}
