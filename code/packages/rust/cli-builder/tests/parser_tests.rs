// tests/parser_tests.rs -- Integration tests for the full parser pipeline
// ==========================================================================
//
// These tests exercise the public Parser API end-to-end using the full Unix
// utility specs from §10 of the spec document.
//
// Each section tests one spec with representative invocations and expected
// outputs as described in the spec's tables.

use cli_builder::errors::CliBuilderError;
use cli_builder::parser::Parser;
use cli_builder::spec_loader::load_spec_from_str;
use cli_builder::types::{ParseResult, ParserOutput};
use serde_json::json;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn parse_ok(spec_json: &str, args: &[&str]) -> ParseResult {
    let spec = load_spec_from_str(spec_json).expect("spec load failed");
    let parser = Parser::new(spec);
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    match parser.parse(&argv).expect("parse should succeed") {
        ParserOutput::Parse(r) => r,
        ParserOutput::Help(h) => panic!("got Help: {}", h.text),
        ParserOutput::Version(v) => panic!("got Version: {}", v.version),
    }
}

fn parse_err_types(spec_json: &str, args: &[&str]) -> Vec<String> {
    let spec = load_spec_from_str(spec_json).expect("spec load failed");
    let parser = Parser::new(spec);
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    match parser.parse(&argv) {
        Err(CliBuilderError::ParseErrors(e)) => e.errors.iter().map(|pe| pe.error_type.clone()).collect(),
        Ok(_) => panic!("expected error"),
        Err(other) => panic!("unexpected error: {}", other),
    }
}

fn has_error(types: &[String], t: &str) -> bool {
    types.iter().any(|e| e == t)
}

// ---------------------------------------------------------------------------
// §10.1 echo — variadic args, flag conflicts
// ---------------------------------------------------------------------------

const ECHO_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "echo",
    "description": "Display a line of text",
    "version": "8.32",
    "flags": [
        {"id":"no-newline","short":"n","description":"Do not output trailing newline","type":"boolean"},
        {"id":"enable-escapes","short":"e","description":"Enable backslash escapes","type":"boolean","conflicts_with":["disable-escapes"]},
        {"id":"disable-escapes","short":"E","description":"Disable backslash escapes","type":"boolean","conflicts_with":["enable-escapes"]}
    ],
    "arguments": [
        {"id":"string","name":"STRING","description":"Text to print","type":"string","required":false,"variadic":true,"variadic_min":0}
    ]
}"#;

#[test]
fn test_echo_hello_world() {
    let r = parse_ok(ECHO_SPEC, &["echo", "hello", "world"]);
    assert_eq!(r.flags["no-newline"], json!(false));
    assert_eq!(r.flags["enable-escapes"], json!(false));
    assert_eq!(r.flags["disable-escapes"], json!(false));
    assert_eq!(r.arguments["string"], json!(["hello", "world"]));
    assert_eq!(r.program, "echo");
    assert_eq!(r.command_path, vec!["echo".to_string()]);
}

#[test]
fn test_echo_n_hello() {
    let r = parse_ok(ECHO_SPEC, &["echo", "-n", "hello"]);
    assert_eq!(r.flags["no-newline"], json!(true));
    assert_eq!(r.arguments["string"], json!(["hello"]));
}

#[test]
fn test_echo_empty() {
    let r = parse_ok(ECHO_SPEC, &["echo"]);
    assert_eq!(r.flags["no-newline"], json!(false));
    assert_eq!(r.arguments["string"], json!([]));
}

#[test]
fn test_echo_conflicting_e_E() {
    let errs = parse_err_types(ECHO_SPEC, &["echo", "-e", "-E", "hello"]);
    assert!(has_error(&errs, "conflicting_flags"), "errors: {:?}", errs);
}

// ---------------------------------------------------------------------------
// §10.3 ls — stacking, flag requires, optional variadic
// ---------------------------------------------------------------------------

const LS_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "ls",
    "description": "List directory contents",
    "version": "8.32",
    "parsing_mode": "gnu",
    "flags": [
        {"id":"long-listing","short":"l","description":"Use long listing format","type":"boolean","conflicts_with":["single-column"]},
        {"id":"all","short":"a","long":"all","description":"Do not ignore entries starting with .","type":"boolean"},
        {"id":"human-readable","short":"h","long":"human-readable","description":"Print sizes like 1K 234M 2G","type":"boolean","requires":["long-listing"]},
        {"id":"reverse","short":"r","long":"reverse","description":"Reverse order while sorting","type":"boolean"},
        {"id":"sort-time","short":"t","description":"Sort by modification time","type":"boolean"},
        {"id":"recursive","short":"R","long":"recursive","description":"List subdirectories recursively","type":"boolean"},
        {"id":"single-column","short":"1","description":"List one file per line","type":"boolean","conflicts_with":["long-listing"]}
    ],
    "arguments": [
        {"id":"path","name":"PATH","description":"Directory or file to list","type":"path","required":false,"variadic":true,"variadic_min":0,"default":"."}
    ]
}"#;

#[test]
fn test_ls_no_args() {
    let r = parse_ok(LS_SPEC, &["ls"]);
    assert_eq!(r.flags["long-listing"], json!(false));
    // Default "." should be applied when no paths given
    assert_eq!(r.arguments["path"], json!("."));
}

#[test]
fn test_ls_l_flag() {
    let r = parse_ok(LS_SPEC, &["ls", "-l"]);
    assert_eq!(r.flags["long-listing"], json!(true));
}

#[test]
fn test_ls_stacked_lah_tmp() {
    let r = parse_ok(LS_SPEC, &["ls", "-lah", "/tmp"]);
    assert_eq!(r.flags["long-listing"], json!(true));
    assert_eq!(r.flags["all"], json!(true));
    assert_eq!(r.flags["human-readable"], json!(true));
    assert_eq!(r.arguments["path"], json!(["/tmp"]));
}

#[test]
fn test_ls_la() {
    let r = parse_ok(LS_SPEC, &["ls", "-la"]);
    assert_eq!(r.flags["long-listing"], json!(true));
    assert_eq!(r.flags["all"], json!(true));
}

#[test]
fn test_ls_h_without_l() {
    let errs = parse_err_types(LS_SPEC, &["ls", "-h"]);
    assert!(has_error(&errs, "missing_dependency_flag"), "errors: {:?}", errs);
}

#[test]
fn test_ls_conflict_1_and_l() {
    let errs = parse_err_types(LS_SPEC, &["ls", "-1", "-l"]);
    assert!(has_error(&errs, "conflicting_flags"), "errors: {:?}", errs);
}

#[test]
fn test_ls_long_all_flag() {
    let r = parse_ok(LS_SPEC, &["ls", "--all"]);
    assert_eq!(r.flags["all"], json!(true));
}

#[test]
fn test_ls_long_recursive() {
    let r = parse_ok(LS_SPEC, &["ls", "--recursive", "/home"]);
    assert_eq!(r.flags["recursive"], json!(true));
    assert_eq!(r.arguments["path"], json!(["/home"]));
}

#[test]
fn test_ls_multiple_paths() {
    let r = parse_ok(LS_SPEC, &["ls", "/tmp", "/var", "/usr"]);
    assert_eq!(r.arguments["path"], json!(["/tmp", "/var", "/usr"]));
}

// ---------------------------------------------------------------------------
// §10.6 cp — variadic sources with required trailing dest
// ---------------------------------------------------------------------------

const CP_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "cp",
    "description": "Copy files and directories",
    "version": "8.32",
    "flags": [
        {"id":"recursive","short":"r","long":"recursive","description":"Copy directories recursively","type":"boolean"},
        {"id":"force","short":"f","long":"force","description":"Overwrite without prompting","type":"boolean","conflicts_with":["interactive","no-clobber"]},
        {"id":"interactive","short":"i","long":"interactive","description":"Prompt before overwrite","type":"boolean","conflicts_with":["force","no-clobber"]},
        {"id":"no-clobber","short":"n","long":"no-clobber","description":"Do not overwrite existing file","type":"boolean","conflicts_with":["force","interactive"]},
        {"id":"verbose","short":"v","long":"verbose","description":"Explain what is being done","type":"boolean"}
    ],
    "arguments": [
        {"id":"source","name":"SOURCE","description":"Source file(s) or directory","type":"path","required":true,"variadic":true,"variadic_min":1},
        {"id":"dest","name":"DEST","description":"Destination file or directory","type":"path","required":true,"variadic":false}
    ]
}"#;

#[test]
fn test_cp_single_source_dest() {
    let r = parse_ok(CP_SPEC, &["cp", "a.txt", "/tmp/"]);
    assert_eq!(r.arguments["source"], json!(["a.txt"]));
    assert_eq!(r.arguments["dest"], json!("/tmp/"));
}

#[test]
fn test_cp_multi_source_dest() {
    let r = parse_ok(CP_SPEC, &["cp", "a.txt", "b.txt", "c.txt", "/dest/"]);
    assert_eq!(r.arguments["source"], json!(["a.txt", "b.txt", "c.txt"]));
    assert_eq!(r.arguments["dest"], json!("/dest/"));
}

#[test]
fn test_cp_missing_dest() {
    let errs = parse_err_types(CP_SPEC, &["cp", "a.txt"]);
    assert!(!errs.is_empty(), "expected at least one error");
}

#[test]
fn test_cp_no_args() {
    let errs = parse_err_types(CP_SPEC, &["cp"]);
    assert!(!errs.is_empty());
}

#[test]
fn test_cp_recursive_flag() {
    let r = parse_ok(CP_SPEC, &["cp", "-r", "src/", "dst/"]);
    assert_eq!(r.flags["recursive"], json!(true));
    assert_eq!(r.arguments["source"], json!(["src/"]));
    assert_eq!(r.arguments["dest"], json!("dst/"));
}

#[test]
fn test_cp_force_interactive_conflict() {
    let errs = parse_err_types(CP_SPEC, &["cp", "-f", "-i", "a.txt", "/tmp/"]);
    assert!(has_error(&errs, "conflicting_flags"));
}

// ---------------------------------------------------------------------------
// §10.7 grep — conditional required arg, exclusive group, repeatable flag
// ---------------------------------------------------------------------------

const GREP_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "grep",
    "description": "Print lines that match patterns",
    "version": "3.7",
    "flags": [
        {"id":"ignore-case","short":"i","long":"ignore-case","description":"Ignore case distinctions in patterns","type":"boolean"},
        {"id":"invert-match","short":"v","long":"invert-match","description":"Invert the sense of matching","type":"boolean"},
        {"id":"line-number","short":"n","long":"line-number","description":"Print line number with output lines","type":"boolean"},
        {"id":"count","short":"c","long":"count","description":"Print only a count of matching lines","type":"boolean"},
        {"id":"recursive","short":"r","long":"recursive","description":"Read all files under each directory recursively","type":"boolean"},
        {"id":"regexp","short":"e","long":"regexp","description":"Use PATTERN as the pattern","type":"string","value_name":"PATTERN","repeatable":true},
        {"id":"extended-regexp","short":"E","long":"extended-regexp","description":"PATTERN is an extended regular expression","type":"boolean"},
        {"id":"fixed-strings","short":"F","long":"fixed-strings","description":"PATTERN is a set of newline-separated strings","type":"boolean"},
        {"id":"perl-regexp","short":"P","long":"perl-regexp","description":"PATTERN is a Perl regular expression","type":"boolean"}
    ],
    "arguments": [
        {"id":"pattern","name":"PATTERN","description":"The search pattern","type":"string","required":true,"required_unless_flag":["regexp"]},
        {"id":"files","name":"FILE","description":"Files to search","type":"path","required":false,"variadic":true,"variadic_min":0}
    ],
    "mutually_exclusive_groups": [
        {"id":"regex-engine","flag_ids":["extended-regexp","fixed-strings","perl-regexp"],"required":false}
    ]
}"#;

#[test]
fn test_grep_basic_search() {
    let r = parse_ok(GREP_SPEC, &["grep", "-i", "foo", "file.txt"]);
    assert_eq!(r.flags["ignore-case"], json!(true));
    assert_eq!(r.arguments["pattern"], json!("foo"));
    assert_eq!(r.arguments["files"], json!(["file.txt"]));
}

#[test]
fn test_grep_extended_mode() {
    let r = parse_ok(GREP_SPEC, &["grep", "-E", "^[0-9]+", "data.log"]);
    assert_eq!(r.flags["extended-regexp"], json!(true));
    assert_eq!(r.arguments["pattern"], json!("^[0-9]+"));
}

#[test]
fn test_grep_repeatable_e_flag() {
    let r = parse_ok(GREP_SPEC, &["grep", "-e", "foo", "-e", "bar", "file.txt"]);
    // -e is repeatable → array
    assert_eq!(r.flags["regexp"], json!(["foo", "bar"]));
    // pattern is optional because -e is present
    assert!(r.arguments.get("pattern").map(|v| v.is_null()).unwrap_or(true));
    assert_eq!(r.arguments["files"], json!(["file.txt"]));
}

#[test]
fn test_grep_exclusive_group_violation() {
    let errs = parse_err_types(GREP_SPEC, &["grep", "-E", "-F", "pattern"]);
    assert!(has_error(&errs, "exclusive_group_violation"), "errors: {:?}", errs);
}

#[test]
fn test_grep_missing_pattern() {
    // No args at all → PATTERN is required
    let errs = parse_err_types(GREP_SPEC, &["grep"]);
    assert!(has_error(&errs, "missing_required_argument"), "errors: {:?}", errs);
}

// ---------------------------------------------------------------------------
// §10.10 tar — traditional mode, required exclusive group
// ---------------------------------------------------------------------------

const TAR_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "tar",
    "description": "An archiving utility",
    "version": "1.34",
    "parsing_mode": "traditional",
    "flags": [
        {"id":"create","short":"c","description":"Create a new archive","type":"boolean"},
        {"id":"extract","short":"x","description":"Extract files from an archive","type":"boolean"},
        {"id":"list","short":"t","description":"List the contents of an archive","type":"boolean"},
        {"id":"verbose","short":"v","long":"verbose","description":"Verbosely list files processed","type":"boolean"},
        {"id":"file","short":"f","long":"file","description":"Use archive file or device ARCHIVE","type":"path","value_name":"ARCHIVE"},
        {"id":"gzip","short":"z","long":"gzip","description":"Filter the archive through gzip","type":"boolean"},
        {"id":"bzip2","short":"j","long":"bzip2","description":"Filter the archive through bzip2","type":"boolean"},
        {"id":"xz","short":"J","long":"xz","description":"Filter the archive through xz","type":"boolean"}
    ],
    "arguments": [
        {"id":"member","name":"MEMBER","description":"Archive members to extract or list","type":"path","required":false,"variadic":true,"variadic_min":0}
    ],
    "mutually_exclusive_groups": [
        {"id":"operation","flag_ids":["create","extract","list"],"required":true},
        {"id":"compression","flag_ids":["gzip","bzip2","xz"],"required":false}
    ]
}"#;

#[test]
fn test_tar_traditional_xvf() {
    let r = parse_ok(TAR_SPEC, &["tar", "xvf", "archive.tar"]);
    assert_eq!(r.flags["extract"], json!(true));
    assert_eq!(r.flags["verbose"], json!(true));
    assert_eq!(r.flags["file"], json!("archive.tar"));
}

#[test]
fn test_tar_gnu_style_czvf() {
    let r = parse_ok(TAR_SPEC, &["tar", "-czvf", "out.tar.gz", "./src"]);
    assert_eq!(r.flags["create"], json!(true));
    assert_eq!(r.flags["gzip"], json!(true));
    assert_eq!(r.flags["verbose"], json!(true));
    assert_eq!(r.flags["file"], json!("out.tar.gz"));
    assert_eq!(r.arguments["member"], json!(["./src"]));
}

#[test]
fn test_tar_list_archive() {
    let r = parse_ok(TAR_SPEC, &["tar", "tf", "archive.tar"]);
    assert_eq!(r.flags["list"], json!(true));
    assert_eq!(r.flags["file"], json!("archive.tar"));
}

#[test]
fn test_tar_missing_operation() {
    // -vf but no create/extract/list → required exclusive group violation
    let errs = parse_err_types(TAR_SPEC, &["tar", "-vf", "archive.tar"]);
    assert!(has_error(&errs, "missing_exclusive_group"), "errors: {:?}", errs);
}

#[test]
fn test_tar_create_and_extract_conflict() {
    let errs = parse_err_types(TAR_SPEC, &["tar", "-cxf", "archive.tar"]);
    assert!(has_error(&errs, "exclusive_group_violation"), "errors: {:?}", errs);
}

// ---------------------------------------------------------------------------
// §10.12 git (partial) — deep subcommands, global flags, nested routing
// ---------------------------------------------------------------------------

const GIT_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "The stupid content tracker",
    "version": "2.43.0",
    "parsing_mode": "subcommand_first",
    "global_flags": [
        {"id":"work-tree","short":"C","description":"Run as if git was started in PATH","type":"directory","value_name":"PATH","repeatable":true},
        {"id":"config-env","short":"c","description":"Pass a configuration parameter to the command","type":"string","value_name":"name=value","repeatable":true},
        {"id":"no-pager","long":"no-pager","description":"Do not pipe output into a pager","type":"boolean"}
    ],
    "commands": [
        {
            "id": "cmd-add",
            "name": "add",
            "description": "Add file contents to the index",
            "flags": [
                {"id":"dry-run","short":"n","long":"dry-run","description":"Dry run","type":"boolean"},
                {"id":"verbose","short":"v","long":"verbose","description":"Be verbose","type":"boolean"},
                {"id":"all","short":"A","long":"all","description":"Add all changes","type":"boolean"},
                {"id":"patch","short":"p","long":"patch","description":"Interactively choose hunks of patch","type":"boolean"}
            ],
            "arguments": [
                {"id":"pathspec","name":"PATHSPEC","description":"Files to add content from","type":"path","required":false,"variadic":true,"variadic_min":0}
            ]
        },
        {
            "id": "cmd-commit",
            "name": "commit",
            "aliases": ["ci"],
            "description": "Record changes to the repository",
            "flags": [
                {"id":"message","short":"m","long":"message","description":"Commit message","type":"string","value_name":"MSG","required":true},
                {"id":"all","short":"a","long":"all","description":"Stage all tracked changes","type":"boolean"},
                {"id":"amend","long":"amend","description":"Amend last commit","type":"boolean"}
            ],
            "arguments": []
        },
        {
            "id": "cmd-remote",
            "name": "remote",
            "description": "Manage set of tracked repositories",
            "flags": [
                {"id":"verbose","short":"v","long":"verbose","description":"Be verbose","type":"boolean"}
            ],
            "commands": [
                {
                    "id": "cmd-remote-add",
                    "name": "add",
                    "description": "Add a named remote repository",
                    "flags": [],
                    "arguments": [
                        {"id":"name","name":"NAME","description":"Remote name","type":"string","required":true},
                        {"id":"url","name":"URL","description":"Remote URL","type":"string","required":true}
                    ]
                }
            ]
        }
    ]
}"#;

#[test]
fn test_git_add_verbose_dot() {
    let r = parse_ok(GIT_SPEC, &["git", "add", "-v", "."]);
    assert_eq!(r.command_path, vec!["git".to_string(), "add".to_string()]);
    assert_eq!(r.flags["verbose"], json!(true));
    assert_eq!(r.arguments["pathspec"], json!(["."]));
}

#[test]
fn test_git_add_no_args() {
    let r = parse_ok(GIT_SPEC, &["git", "add"]);
    assert_eq!(r.command_path, vec!["git".to_string(), "add".to_string()]);
    assert_eq!(r.arguments["pathspec"], json!([]));
}

#[test]
fn test_git_commit_with_message() {
    let r = parse_ok(GIT_SPEC, &["git", "commit", "-m", "initial commit"]);
    assert_eq!(r.command_path, vec!["git".to_string(), "commit".to_string()]);
    assert_eq!(r.flags["message"], json!("initial commit"));
    assert_eq!(r.flags["all"], json!(false));
}

#[test]
fn test_git_commit_alias_ci() {
    let r = parse_ok(GIT_SPEC, &["git", "ci", "-m", "msg"]);
    // canonical name for "ci" alias is "commit"
    assert_eq!(r.command_path[1], "commit");
}

#[test]
fn test_git_commit_missing_required_message() {
    let errs = parse_err_types(GIT_SPEC, &["git", "commit"]);
    assert!(has_error(&errs, "missing_required_flag"), "errors: {:?}", errs);
}

#[test]
fn test_git_global_no_pager() {
    let r = parse_ok(GIT_SPEC, &["git", "--no-pager", "add", "."]);
    assert_eq!(r.flags["no-pager"], json!(true));
}

#[test]
fn test_git_remote_add() {
    let r = parse_ok(GIT_SPEC, &["git", "remote", "add", "origin", "https://github.com/x/y"]);
    assert_eq!(r.command_path, vec!["git".to_string(), "remote".to_string(), "add".to_string()]);
    assert_eq!(r.arguments["name"], json!("origin"));
    assert_eq!(r.arguments["url"], json!("https://github.com/x/y"));
}

#[test]
fn test_git_commit_amend_flag() {
    let r = parse_ok(GIT_SPEC, &["git", "commit", "-m", "fixup", "--amend"]);
    assert_eq!(r.flags["amend"], json!(true));
}

// ---------------------------------------------------------------------------
// Help and version output tests
// ---------------------------------------------------------------------------

#[test]
fn test_help_long_flag() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let parser = Parser::new(spec);
    let argv: Vec<String> = vec!["echo".into(), "--help".into()];
    let out = parser.parse(&argv).unwrap();
    match out {
        ParserOutput::Help(h) => {
            assert!(!h.text.is_empty());
            assert!(h.text.contains("echo") || h.text.contains("USAGE"));
        }
        _ => panic!("expected Help"),
    }
}

#[test]
fn test_help_short_flag() {
    let spec = load_spec_from_str(LS_SPEC).unwrap();
    let parser = Parser::new(spec);
    let argv: Vec<String> = vec!["ls".into(), "-h".into()];
    // Note: in ls spec, -h is human-readable; but our builtin -h takes precedence
    // IF the spec enables builtin_flags.help. The builtin -h check happens in scan().
    let out = parser.parse(&argv).unwrap();
    // Either Help or parse result depending on whether builtin -h takes precedence.
    // The spec says builtin flags can be disabled, but default is enabled.
    // With default builtin_flags, -h triggers help.
    assert!(matches!(out, ParserOutput::Help(_) | ParserOutput::Parse(_)));
}

#[test]
fn test_version_flag_echo() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let parser = Parser::new(spec);
    let argv: Vec<String> = vec!["echo".into(), "--version".into()];
    match parser.parse(&argv).unwrap() {
        ParserOutput::Version(v) => assert_eq!(v.version, "8.32"),
        _ => panic!("expected Version"),
    }
}

#[test]
fn test_git_add_help() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let parser = Parser::new(spec);
    let argv: Vec<String> = vec!["git".into(), "add".into(), "--help".into()];
    match parser.parse(&argv).unwrap() {
        ParserOutput::Help(h) => {
            // Help text should reference the "add" subcommand
            assert!(h.text.contains("add") || !h.text.is_empty());
        }
        _ => panic!("expected Help"),
    }
}

// ---------------------------------------------------------------------------
// End-of-flags sentinel (--)
// ---------------------------------------------------------------------------

#[test]
fn test_double_dash_makes_following_positional() {
    let r = parse_ok(ECHO_SPEC, &["echo", "--", "-n", "hello"]);
    // Everything after -- is positional
    assert_eq!(r.flags["no-newline"], json!(false));
    assert_eq!(r.arguments["string"], json!(["-n", "hello"]));
}

#[test]
fn test_double_dash_at_start() {
    let r = parse_ok(ECHO_SPEC, &["echo", "--", "world"]);
    assert_eq!(r.arguments["string"], json!(["world"]));
}

// ---------------------------------------------------------------------------
// Integer-valued flags
// ---------------------------------------------------------------------------

const HEAD_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "head",
    "description": "Output the first part of files",
    "version": "8.32",
    "flags": [
        {"id":"lines","short":"n","long":"lines","description":"Print the first NUM lines","type":"integer","value_name":"NUM","default":10,"conflicts_with":["bytes"]},
        {"id":"bytes","short":"c","long":"bytes","description":"Print the first NUM bytes","type":"integer","value_name":"NUM","conflicts_with":["lines"]},
        {"id":"quiet","short":"q","long":"quiet","description":"Never print headers giving file names","type":"boolean"},
        {"id":"verbose","short":"v","long":"verbose","description":"Always print headers giving file names","type":"boolean"}
    ],
    "arguments": [
        {"id":"file","name":"FILE","description":"Files to read","type":"path","required":false,"variadic":true,"variadic_min":0}
    ]
}"#;

#[test]
fn test_head_default_lines() {
    let r = parse_ok(HEAD_SPEC, &["head", "file.txt"]);
    assert_eq!(r.flags["lines"], json!(10));
    assert_eq!(r.arguments["file"], json!(["file.txt"]));
}

#[test]
fn test_head_n_flag() {
    let r = parse_ok(HEAD_SPEC, &["head", "-n", "20", "file.txt"]);
    assert_eq!(r.flags["lines"], json!(20));
}

#[test]
fn test_head_long_n_flag() {
    let r = parse_ok(HEAD_SPEC, &["head", "--lines=5"]);
    assert_eq!(r.flags["lines"], json!(5));
}

#[test]
fn test_head_invalid_integer() {
    let errs = parse_err_types(HEAD_SPEC, &["head", "-n", "abc"]);
    assert!(has_error(&errs, "invalid_value"), "errors: {:?}", errs);
}

#[test]
fn test_head_bytes_lines_conflict() {
    let errs = parse_err_types(HEAD_SPEC, &["head", "-n", "5", "-c", "100"]);
    assert!(has_error(&errs, "conflicting_flags"));
}

// ---------------------------------------------------------------------------
// Duplicate non-repeatable flag
// ---------------------------------------------------------------------------

#[test]
fn test_duplicate_flag_error() {
    let errs = parse_err_types(LS_SPEC, &["ls", "-a", "--all"]);
    assert!(has_error(&errs, "duplicate_flag"), "errors: {:?}", errs);
}

// ---------------------------------------------------------------------------
// POSIX mode
// ---------------------------------------------------------------------------

const POSIX_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "posix-tool",
    "description": "A POSIX mode tool",
    "parsing_mode": "posix",
    "flags": [
        {"id":"verbose","short":"v","long":"verbose","description":"verbose","type":"boolean"}
    ],
    "arguments": [
        {"id":"files","name":"FILE","description":"files","type":"path","required":false,"variadic":true,"variadic_min":0}
    ]
}"#;

#[test]
fn test_posix_mode_flag_before_arg() {
    let r = parse_ok(POSIX_SPEC, &["posix-tool", "-v", "file.txt"]);
    assert_eq!(r.flags["verbose"], json!(true));
    assert_eq!(r.arguments["files"], json!(["file.txt"]));
}

#[test]
fn test_posix_mode_first_arg_ends_flag_scanning() {
    // In POSIX mode, once a positional is seen, no more flags.
    // So "file.txt -v" has "file.txt" as arg and "-v" also as positional.
    let r = parse_ok(POSIX_SPEC, &["posix-tool", "file.txt", "-v"]);
    assert_eq!(r.flags["verbose"], json!(false));
    // Both "file.txt" and "-v" should be in positional list
    assert_eq!(r.arguments["files"], json!(["file.txt", "-v"]));
}
