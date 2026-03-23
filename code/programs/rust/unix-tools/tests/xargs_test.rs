//! # Integration Tests for xargs
//!
//! These tests verify the xargs pipeline: input splitting, item
//! grouping, command building, and execution.

use unix_tools::xargs_tool::*;

// ---------------------------------------------------------------------------
// Tests: Input splitting
// ---------------------------------------------------------------------------

#[cfg(test)]
mod splitting {
    use super::*;

    #[test]
    fn split_whitespace() {
        let items = split_input("a b c", &XargsOptions::default());
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn split_newlines() {
        let items = split_input("a\nb\nc", &XargsOptions::default());
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn split_tabs() {
        let items = split_input("a\tb\tc", &XargsOptions::default());
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn split_mixed_whitespace() {
        let items = split_input("a  b\t\tc\n\nd", &XargsOptions::default());
        assert_eq!(items, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn split_null_delimited() {
        let opts = XargsOptions { null_delim: true, ..Default::default() };
        let items = split_input("a\0b\0c\0", &opts);
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn split_null_preserves_spaces() {
        let opts = XargsOptions { null_delim: true, ..Default::default() };
        let items = split_input("file with spaces\0another\0", &opts);
        assert_eq!(items, vec!["file with spaces", "another"]);
    }

    #[test]
    fn split_custom_delimiter() {
        let opts = XargsOptions { delimiter: Some(':'), ..Default::default() };
        let items = split_input("/usr/bin:/bin:/sbin", &opts);
        assert_eq!(items, vec!["/usr/bin", "/bin", "/sbin"]);
    }

    #[test]
    fn split_empty_input() {
        let items = split_input("", &XargsOptions::default());
        assert!(items.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Tests: Item grouping
// ---------------------------------------------------------------------------

#[cfg(test)]
mod grouping {
    use super::*;

    #[test]
    fn group_all_in_one() {
        let items = vec!["a".into(), "b".into(), "c".into()];
        let groups = group_items(items, &XargsOptions::default());
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn group_by_two() {
        let items = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let opts = XargsOptions { max_args: Some(2), ..Default::default() };
        let groups = group_items(items, &opts);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[1].len(), 2);
        assert_eq!(groups[2].len(), 1);
    }

    #[test]
    fn group_by_one() {
        let items = vec!["a".into(), "b".into(), "c".into()];
        let opts = XargsOptions { max_args: Some(1), ..Default::default() };
        let groups = group_items(items, &opts);
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn group_empty() {
        let items: Vec<String> = vec![];
        let groups = group_items(items, &XargsOptions::default());
        assert!(groups.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Tests: Command building
// ---------------------------------------------------------------------------

#[cfg(test)]
mod building {
    use super::*;

    #[test]
    fn build_append() {
        let cmd = vec!["echo".into()];
        let items = vec!["a".into(), "b".into()];
        let commands = build_command_args(&cmd, &items, &None);
        assert_eq!(commands, vec![vec!["echo", "a", "b"]]);
    }

    #[test]
    fn build_replace() {
        let cmd = vec!["echo".into(), "{}".into()];
        let items = vec!["hello".into(), "world".into()];
        let replace = Some("{}".to_string());
        let commands = build_command_args(&cmd, &items, &replace);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0], vec!["echo", "hello"]);
        assert_eq!(commands[1], vec!["echo", "world"]);
    }

    #[test]
    fn build_replace_in_multiple_args() {
        let cmd = vec!["mv".into(), "{}".into(), "{}.bak".into()];
        let items = vec!["file.txt".into()];
        let replace = Some("{}".to_string());
        let commands = build_command_args(&cmd, &items, &replace);
        assert_eq!(commands[0], vec!["mv", "file.txt", "file.txt.bak"]);
    }

    #[test]
    fn build_with_no_items() {
        let cmd = vec!["echo".into()];
        let items: Vec<String> = vec![];
        let commands = build_command_args(&cmd, &items, &None);
        // Should just be the command with no extra args
        assert_eq!(commands, vec![vec!["echo"]]);
    }
}

// ---------------------------------------------------------------------------
// Tests: Execution
// ---------------------------------------------------------------------------

#[cfg(test)]
mod execution {
    use super::*;

    #[test]
    fn execute_echo_succeeds() {
        let args = vec!["echo".into(), "test".into()];
        let result = execute_command(&args, false);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn execute_false_returns_nonzero() {
        let args = vec!["false".into()];
        let result = execute_command(&args, false);
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn execute_nonexistent_fails() {
        let args = vec!["not_a_command_xyzzy_123".into()];
        let result = execute_command(&args, false);
        assert!(result.is_err());
    }

    #[test]
    fn execute_empty_args_fails() {
        let args: Vec<String> = vec![];
        let result = execute_command(&args, false);
        assert!(result.is_err());
    }
}

// ---------------------------------------------------------------------------
// Tests: Full pipeline
// ---------------------------------------------------------------------------

#[cfg(test)]
mod pipeline {
    use super::*;

    #[test]
    fn run_xargs_echo() {
        let result = run_xargs("a b c", &["echo".into()], &XargsOptions::default());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_xargs_no_run_if_empty() {
        let opts = XargsOptions { no_run_if_empty: true, ..Default::default() };
        let result = run_xargs("", &["echo".into()], &opts);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_xargs_with_max_args() {
        let opts = XargsOptions { max_args: Some(1), ..Default::default() };
        let result = run_xargs("a b c", &["echo".into()], &opts);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_xargs_with_replace() {
        let opts = XargsOptions {
            replace_str: Some("{}".to_string()),
            max_args: Some(1),
            ..Default::default()
        };
        let result = run_xargs("hello world", &["echo".into(), "item: {}".into()], &opts);
        assert_eq!(result.unwrap(), 0);
    }
}
