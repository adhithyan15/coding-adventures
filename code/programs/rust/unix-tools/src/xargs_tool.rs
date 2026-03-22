//! # xargs — Build and Execute Commands from Standard Input
//!
//! This module implements the business logic for the `xargs` command.
//! `xargs` reads items from standard input and executes a command with
//! those items as arguments.
//!
//! ## How It Works
//!
//! ```text
//!     Input:   "file1.txt\nfile2.txt\nfile3.txt"
//!     Command: rm
//!     Result:  rm file1.txt file2.txt file3.txt
//! ```
//!
//! The key idea is that many commands accept multiple arguments, but
//! sometimes you have those arguments as lines in a file or piped from
//! another command. `xargs` bridges this gap.
//!
//! ## Splitting Modes
//!
//! ```text
//!     Mode             Flag    Delimiter
//!     ───────────────  ─────   ────────────────────────────
//!     Whitespace       (none)  Spaces, tabs, newlines
//!     Null-terminated  -0      NUL byte (\0)
//!     Custom           -d X    Any single character X
//! ```
//!
//! ## Item Grouping
//!
//! ```text
//!     Flag    Behavior
//!     ─────   ────────────────────────────────────────
//!     -n N    Pass at most N items per command invocation
//!     -I {}   Replace {} in the command with each item
//! ```
//!
//! ## Flags
//!
//! ```text
//!     Flag              Field           Effect
//!     ────────────────  ──────────────  ──────────────────────────────
//!     -0, --null        null_delim      Use NUL as input delimiter
//!     -d, --delimiter   delimiter       Use custom delimiter
//!     -n, --max-args    max_args        Max items per command invocation
//!     -I, --replace     replace_str     Replace string in command template
//!     -t, --verbose     verbose         Print command before executing
//!     -r, --no-run-if-empty  no_run_if_empty  Don't run if input is empty
//! ```

use std::process::Command;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling how `xargs` reads input and invokes commands.
#[derive(Debug, Clone)]
pub struct XargsOptions {
    /// Use NUL byte as the input delimiter (-0).
    pub null_delim: bool,
    /// Custom delimiter character (-d).
    pub delimiter: Option<char>,
    /// Maximum number of arguments per command invocation (-n).
    pub max_args: Option<usize>,
    /// String in the command template to replace with each item (-I).
    pub replace_str: Option<String>,
    /// Print each command to stderr before executing (-t).
    pub verbose: bool,
    /// Don't run the command if the input is empty (-r).
    pub no_run_if_empty: bool,
}

impl Default for XargsOptions {
    fn default() -> Self {
        XargsOptions {
            null_delim: false,
            delimiter: None,
            max_args: None,
            replace_str: None,
            verbose: false,
            no_run_if_empty: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Input Parsing
// ---------------------------------------------------------------------------

/// Split input into items based on the delimiter configuration.
///
/// The splitting strategy depends on the flags:
///
/// ```text
///     -0:       Split on NUL bytes (for find -print0 output)
///     -d X:     Split on character X
///     (default) Split on whitespace (spaces, tabs, newlines)
/// ```
///
/// ## Why NUL-delimited input matters
///
/// Filenames can contain spaces, tabs, and even newlines. The only
/// character that cannot appear in a Unix filename is NUL (\0). So
/// `find -print0 | xargs -0` is the safe way to process arbitrary
/// filenames.
pub fn split_input(input: &str, opts: &XargsOptions) -> Vec<String> {
    if opts.null_delim {
        // --- NUL-delimited mode ---
        input
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else if let Some(delim) = opts.delimiter {
        // --- Custom delimiter mode ---
        input
            .split(delim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        // --- Default: whitespace splitting ---
        // This handles spaces, tabs, and newlines.
        input
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
}

/// Group items into batches according to -n (max_args).
///
/// ```text
///     Items: [a, b, c, d, e]
///     max_args: 2
///     Batches: [[a, b], [c, d], [e]]
/// ```
///
/// Without -n, all items go in a single batch.
pub fn group_items(items: Vec<String>, opts: &XargsOptions) -> Vec<Vec<String>> {
    match opts.max_args {
        Some(n) if n > 0 => {
            items.chunks(n).map(|chunk| chunk.to_vec()).collect()
        }
        _ => {
            if items.is_empty() {
                vec![]
            } else {
                vec![items]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Command Building
// ---------------------------------------------------------------------------

/// Build the command line for a batch of items.
///
/// Two modes:
///
/// ```text
///     Mode        Command Template    Items     Result
///     ──────────  ─────────────────   ────────  ─────────────────────
///     Append      ["echo"]            [a, b]    ["echo", "a", "b"]
///     Replace     ["echo", "{}"]      [a]       ["echo", "a"]
///                                     [b]       ["echo", "b"]
/// ```
///
/// With -I, each item is processed separately — the replace string
/// is substituted with the item in every occurrence.
pub fn build_command_args(
    command: &[String],
    items: &[String],
    replace_str: &Option<String>,
) -> Vec<Vec<String>> {
    match replace_str {
        Some(repl) => {
            // --- Replace mode ---
            // Each item gets its own command invocation.
            items
                .iter()
                .map(|item| {
                    command
                        .iter()
                        .map(|arg| arg.replace(repl, item))
                        .collect()
                })
                .collect()
        }
        None => {
            // --- Append mode ---
            // All items are appended as extra arguments.
            let mut args: Vec<String> = command.to_vec();
            args.extend(items.iter().cloned());
            vec![args]
        }
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Execute a command with the given arguments.
///
/// Uses `std::process::Command` to spawn a child process and wait
/// for it to complete. Returns the exit code.
///
/// ```text
///     Command: ["echo", "hello", "world"]
///              ───────  ─────────────────
///              program  arguments
/// ```
pub fn execute_command(args: &[String], verbose: bool) -> Result<i32, String> {
    if args.is_empty() {
        return Err("xargs: no command specified".to_string());
    }

    // --- Print command if verbose ---
    if verbose {
        eprintln!("{}", args.join(" "));
    }

    let program = &args[0];
    let cmd_args = &args[1..];

    let status = Command::new(program)
        .args(cmd_args)
        .status()
        .map_err(|e| format!("xargs: {}: {}", program, e))?;

    Ok(status.code().unwrap_or(1))
}

/// Run the full xargs pipeline: parse input, group, build commands, execute.
///
/// # Pipeline
///
/// ```text
///     Input string
///         │
///         ▼
///     split_input()       Split into items
///         │
///         ▼
///     group_items()       Batch by -n
///         │
///         ▼
///     build_command_args() Build command lines
///         │
///         ▼
///     execute_command()   Run each command
/// ```
pub fn run_xargs(
    input: &str,
    command: &[String],
    opts: &XargsOptions,
) -> Result<i32, String> {
    // --- Step 1: Split input into items ---
    let items = split_input(input, opts);

    // --- Step 2: Handle empty input ---
    if items.is_empty() {
        if opts.no_run_if_empty {
            return Ok(0);
        }
        // Default behavior with no items: still run the command once
        // with no extra arguments (matches GNU xargs).
        if command.is_empty() {
            return Ok(0);
        }
        return execute_command(command, opts.verbose);
    }

    // --- Step 3: Group items into batches ---
    let batches = group_items(items, opts);

    // --- Step 4: Build and execute commands ---
    let mut last_exit = 0;

    for batch in batches {
        let commands = build_command_args(command, &batch, &opts.replace_str);
        for cmd_args in commands {
            let exit_code = execute_command(&cmd_args, opts.verbose)?;
            if exit_code != 0 {
                last_exit = exit_code;
            }
        }
    }

    Ok(last_exit)
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- split_input tests ---

    #[test]
    fn split_whitespace_default() {
        let items = split_input("hello world\tfoo\nbar", &XargsOptions::default());
        assert_eq!(items, vec!["hello", "world", "foo", "bar"]);
    }

    #[test]
    fn split_null_delimited() {
        let opts = XargsOptions { null_delim: true, ..Default::default() };
        let items = split_input("file1\0file2\0file3\0", &opts);
        assert_eq!(items, vec!["file1", "file2", "file3"]);
    }

    #[test]
    fn split_custom_delimiter() {
        let opts = XargsOptions { delimiter: Some(','), ..Default::default() };
        let items = split_input("a,b,c", &opts);
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn split_empty_input() {
        let items = split_input("", &XargsOptions::default());
        assert!(items.is_empty());
    }

    #[test]
    fn split_whitespace_only() {
        let items = split_input("   \n\t  ", &XargsOptions::default());
        assert!(items.is_empty());
    }

    // --- group_items tests ---

    #[test]
    fn group_all_in_one_batch() {
        let items = vec!["a".into(), "b".into(), "c".into()];
        let groups = group_items(items, &XargsOptions::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn group_with_max_args() {
        let items = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let opts = XargsOptions { max_args: Some(2), ..Default::default() };
        let groups = group_items(items, &opts);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0], vec!["a", "b"]);
        assert_eq!(groups[1], vec!["c", "d"]);
        assert_eq!(groups[2], vec!["e"]);
    }

    #[test]
    fn group_empty_items() {
        let items: Vec<String> = vec![];
        let groups = group_items(items, &XargsOptions::default());
        assert!(groups.is_empty());
    }

    // --- build_command_args tests ---

    #[test]
    fn build_append_mode() {
        let cmd = vec!["echo".into()];
        let items = vec!["hello".into(), "world".into()];
        let commands = build_command_args(&cmd, &items, &None);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], vec!["echo", "hello", "world"]);
    }

    #[test]
    fn build_replace_mode() {
        let cmd = vec!["echo".into(), "{}".into()];
        let items = vec!["hello".into(), "world".into()];
        let replace = Some("{}".to_string());
        let commands = build_command_args(&cmd, &items, &replace);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0], vec!["echo", "hello"]);
        assert_eq!(commands[1], vec!["echo", "world"]);
    }

    #[test]
    fn build_replace_multiple_occurrences() {
        let cmd = vec!["cp".into(), "{}".into(), "{}.bak".into()];
        let items = vec!["file.txt".into()];
        let replace = Some("{}".to_string());
        let commands = build_command_args(&cmd, &items, &replace);
        assert_eq!(commands[0], vec!["cp", "file.txt", "file.txt.bak"]);
    }

    // --- execute_command tests ---

    #[test]
    fn execute_echo() {
        let args = vec!["echo".into(), "test".into()];
        let result = execute_command(&args, false);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn execute_nonexistent_command() {
        let args = vec!["definitely_not_a_real_command_12345".into()];
        let result = execute_command(&args, false);
        assert!(result.is_err());
    }

    #[test]
    fn execute_empty_args() {
        let args: Vec<String> = vec![];
        let result = execute_command(&args, false);
        assert!(result.is_err());
    }

    // --- run_xargs tests ---

    #[test]
    fn run_xargs_with_echo() {
        let input = "hello world";
        let command = vec!["echo".into()];
        let result = run_xargs(input, &command, &XargsOptions::default());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn run_xargs_no_run_if_empty() {
        let input = "";
        let command = vec!["echo".into()];
        let opts = XargsOptions { no_run_if_empty: true, ..Default::default() };
        let result = run_xargs(input, &command, &opts);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn null_delim_handles_spaces_in_names() {
        let opts = XargsOptions { null_delim: true, ..Default::default() };
        let items = split_input("file with spaces\0another file\0", &opts);
        assert_eq!(items, vec!["file with spaces", "another file"]);
    }
}
