//! # Integration Tests for diff
//!
//! These tests verify that the diff business logic correctly compares
//! files, handles various options, and produces correct output formats.

use unix_tools::diff_tool::*;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("diff_integ_{}", name))
        .to_string_lossy()
        .into_owned()
}

fn cleanup(path: &str) {
    let p = Path::new(path);
    if p.is_dir() {
        let _ = fs::remove_dir_all(p);
    } else {
        let _ = fs::remove_file(p);
    }
}

// ---------------------------------------------------------------------------
// Tests: Basic diff
// ---------------------------------------------------------------------------

#[cfg(test)]
mod basic_diff {
    use super::*;

    #[test]
    fn identical_files_return_none() {
        let a = temp_path("ident_a");
        let b = temp_path("ident_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "line1\nline2\nline3\n").unwrap();
        fs::write(&b, "line1\nline2\nline3\n").unwrap();

        let result = diff_files(&a, &b, &DiffOptions::default()).unwrap();
        assert!(result.is_none());

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn different_files_return_some() {
        let a = temp_path("diff_a");
        let b = temp_path("diff_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "hello\n").unwrap();
        fs::write(&b, "world\n").unwrap();

        let result = diff_files(&a, &b, &DiffOptions::default()).unwrap();
        assert!(result.is_some());

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let result = diff_files("/tmp/diff_integ_noexist_a", "/tmp/diff_integ_noexist_b", &DiffOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn one_line_added() {
        let a = temp_path("add_a");
        let b = temp_path("add_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "line1\nline2\n").unwrap();
        fs::write(&b, "line1\nline2\nline3\n").unwrap();

        let result = diff_files(&a, &b, &DiffOptions::default()).unwrap();
        let output = result.unwrap();
        assert!(output.contains("> line3"));

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn one_line_removed() {
        let a = temp_path("rem_a");
        let b = temp_path("rem_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "line1\nline2\nline3\n").unwrap();
        fs::write(&b, "line1\nline2\n").unwrap();

        let result = diff_files(&a, &b, &DiffOptions::default()).unwrap();
        let output = result.unwrap();
        assert!(output.contains("< line3"));

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
    fn ignore_case() {
        let a = temp_path("icase_a");
        let b = temp_path("icase_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "Hello World\n").unwrap();
        fs::write(&b, "hello world\n").unwrap();

        let opts = DiffOptions { ignore_case: true, ..Default::default() };
        let result = diff_files(&a, &b, &opts).unwrap();
        assert!(result.is_none());

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn ignore_blank_lines() {
        let a = temp_path("blank_a");
        let b = temp_path("blank_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "a\n\n\nb\n").unwrap();
        fs::write(&b, "a\nb\n").unwrap();

        let opts = DiffOptions { ignore_blank_lines: true, ..Default::default() };
        let result = diff_files(&a, &b, &opts).unwrap();
        assert!(result.is_none());

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn brief_mode_reports_difference() {
        let a = temp_path("brief_a");
        let b = temp_path("brief_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "aaa\n").unwrap();
        fs::write(&b, "bbb\n").unwrap();

        let opts = DiffOptions { brief: true, ..Default::default() };
        let result = diff_files(&a, &b, &opts).unwrap();
        let output = result.unwrap();
        assert!(output.contains("differ"));

        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn unified_format() {
        let a = temp_path("uni_a");
        let b = temp_path("uni_b");
        cleanup(&a);
        cleanup(&b);

        fs::write(&a, "a\nb\nc\n").unwrap();
        fs::write(&b, "a\nx\nc\n").unwrap();

        let opts = DiffOptions { unified_context: Some(3), ..Default::default() };
        let result = diff_files(&a, &b, &opts).unwrap();
        let output = result.unwrap();
        assert!(output.contains("---"));
        assert!(output.contains("+++"));
        assert!(output.contains("@@"));

        cleanup(&a);
        cleanup(&b);
    }
}

// ---------------------------------------------------------------------------
// Tests: Directory diff
// ---------------------------------------------------------------------------

#[cfg(test)]
mod dir_diff {
    use super::*;

    #[test]
    fn diff_identical_directories() {
        let dir_a = temp_path("dir_ident_a");
        let dir_b = temp_path("dir_ident_b");
        cleanup(&dir_a);
        cleanup(&dir_b);

        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        fs::write(format!("{}/f.txt", dir_a), "same\n").unwrap();
        fs::write(format!("{}/f.txt", dir_b), "same\n").unwrap();

        let opts = DiffOptions { recursive: true, ..Default::default() };
        let result = diff_directories(&dir_a, &dir_b, &opts).unwrap();
        assert!(result.is_empty());

        cleanup(&dir_a);
        cleanup(&dir_b);
    }

    #[test]
    fn diff_directories_file_only_in_one() {
        let dir_a = temp_path("dir_only_a");
        let dir_b = temp_path("dir_only_b");
        cleanup(&dir_a);
        cleanup(&dir_b);

        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        fs::write(format!("{}/unique.txt", dir_a), "data\n").unwrap();

        let opts = DiffOptions { recursive: true, ..Default::default() };
        let result = diff_directories(&dir_a, &dir_b, &opts).unwrap();
        assert!(result.contains("Only in"));

        cleanup(&dir_a);
        cleanup(&dir_b);
    }

    #[test]
    fn diff_not_a_directory() {
        let file = temp_path("not_dir");
        cleanup(&file);
        fs::write(&file, "data").unwrap();

        let result = diff_directories(&file, "/tmp", &DiffOptions::default());
        assert!(result.is_err());

        cleanup(&file);
    }
}

// ---------------------------------------------------------------------------
// Tests: Edit operations
// ---------------------------------------------------------------------------

#[cfg(test)]
mod edits {
    use super::*;

    #[test]
    fn compute_diff_empty_inputs() {
        let edits = compute_diff(&[], &[], &DiffOptions::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn compute_diff_all_inserts() {
        let edits = compute_diff(&[], &["a", "b"], &DiffOptions::default());
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|e| matches!(e, Edit::Insert(_))));
    }

    #[test]
    fn compute_diff_all_deletes() {
        let edits = compute_diff(&["a", "b"], &[], &DiffOptions::default());
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|e| matches!(e, Edit::Delete(_))));
    }

    #[test]
    fn compute_diff_mixed_changes() {
        let edits = compute_diff(&["a", "b", "c"], &["a", "x", "c"], &DiffOptions::default());
        // Should have: Equal(a), Delete(b), Insert(x), Equal(c)
        assert!(edits.iter().any(|e| matches!(e, Edit::Equal(_))));
        assert!(edits.iter().any(|e| matches!(e, Edit::Delete(_))));
        assert!(edits.iter().any(|e| matches!(e, Edit::Insert(_))));
    }
}
