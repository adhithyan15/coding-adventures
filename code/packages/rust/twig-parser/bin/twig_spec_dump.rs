//! # `twig-spec-dump` — emit Twig's language spec as JSON.
//!
//! ## Why this binary exists
//!
//! Downstream editor tooling — the VS Code extension generator (LS04),
//! treesitter wrappers, syntax-highlight theme generators, future
//! Neovim/Zed integrations — needs to know Twig's keywords, brackets,
//! comment markers, and grammar rule names.  Hand-coding that
//! information into each tool's config means three things have to
//! stay in sync as the language evolves: the canonical
//! `code/grammars/twig.tokens` file, the per-tool config, and any
//! Rust mirror like `TWIG_KEYWORD_NAMES` in `twig-lsp-bridge`.  The
//! sync inevitably drifts.
//!
//! This binary cuts the dependency on `twig.tokens` / `twig.grammar`
//! at runtime entirely.  Those files are build-time-only artifacts
//! — `twig-lexer/build.rs` compiles them into Rust source that
//! materialises a `TokenGrammar` literal, and `twig-parser/build.rs`
//! does the same for `ParserGrammar`.  The compiled artifacts are
//! the source of truth at runtime.
//!
//! `twig-spec-dump` reads those compiled grammars (no file I/O), runs
//! `grammar_tools::dump_spec::dump_language_spec`, and prints the
//! resulting JSON document to stdout (or to `--output <path>`).
//!
//! ## Why `parser` not `lexer`
//!
//! The parser crate already depends on the lexer crate, so the binary
//! lives here naturally — `bin/twig_spec_dump.rs` can pull from both
//! grammars without anything extra.  Putting it in the lexer crate
//! would mean we'd have to replicate the rule list lookup or skip
//! `rules` / `declarationRules` entirely.
//!
//! ## Usage
//!
//! ```bash
//! twig-spec-dump > twig.spec.json
//!
//! twig-spec-dump --output twig.spec.json
//!
//! # Override defaults (these are baked-in for Twig but a fork might want
//! # different display name / extensions):
//! twig-spec-dump --lang-name "Twig 2.0" --extensions twig,tw,t2 > twig.spec.json
//!
//! # Pipe straight into the VS Code extension generator (no temp file):
//! twig-spec-dump | tee twig.spec.json | (cd …/twig-vscode && \
//!     vscode-lang-extension-generator --language-spec /dev/stdin --output-dir .)
//! ```
//!
//! ## CLI surface
//!
//! All flags are optional — the binary has Twig-correct defaults
//! baked in (id, name, extensions, line comment, declaration rules).
//! Override only what differs from the canonical Twig spec:
//!
//! - `--output PATH`            write to file instead of stdout
//! - `--lang-id ID`             override `"twig"`
//! - `--lang-name NAME`         override `"Twig"`
//! - `--extensions LIST`        comma-separated, no leading dots
//! - `--line-comment STR`       override `";"`
//! - `--block-comment-start S`  set block-comment open
//! - `--block-comment-end S`    set block-comment close
//! - `--declaration-rules LIST` comma-separated grammar rule names
//!
//! Exit codes: 0 success, 1 I/O error, 2 bad argument.

use std::env;
use std::process;

use grammar_tools::dump_spec::{dump_language_spec, SpecMetadata};
use twig_lexer::twig_token_grammar_spec;
use twig_parser::twig_grammar;

/// Twig's canonical language identifier.
const DEFAULT_LANG_ID: &str = "twig";

/// Twig's canonical display name.
const DEFAULT_LANG_NAME: &str = "Twig";

/// Twig's canonical file extensions (no leading dots).
const DEFAULT_EXTENSIONS: &[&str] = &["twig", "tw"];

/// Twig's line-comment marker — `;` to end of line, like Scheme.
const DEFAULT_LINE_COMMENT: &str = ";";

/// Twig grammar rules that LSP "document symbols" should surface as
/// top-level declarations.  Mirrors `TWIG_DECLARATION_RULES` in
/// `twig-lsp-bridge`; consolidating them is a follow-up.
const DEFAULT_DECLARATION_RULES: &[&str] = &["define", "module_form"];

/// Parse a `--flag value`-style argument.  Returns `Some(value)` if the
/// caller's argv at `i` matches `name`, after advancing `*i`; `None`
/// otherwise.  Errors loudly on a flag that's missing its value.
fn take_flag(argv: &[String], i: &mut usize, name: &str) -> Option<String> {
    if argv[*i] != name {
        return None;
    }
    if *i + 1 >= argv.len() {
        eprintln!("twig-spec-dump: '{}' requires a value", name);
        process::exit(2);
    }
    *i += 1;
    let value = argv[*i].clone();
    *i += 1;
    Some(value)
}

fn main() {
    let argv: Vec<String> = env::args().collect();

    let mut output_path: Option<String> = None;
    let mut lang_id = DEFAULT_LANG_ID.to_string();
    let mut lang_name = DEFAULT_LANG_NAME.to_string();
    let mut extensions: Vec<String> =
        DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect();
    let mut line_comment = DEFAULT_LINE_COMMENT.to_string();
    let mut block_comment_start = String::new();
    let mut block_comment_end = String::new();
    let mut declaration_rules: Vec<String> = DEFAULT_DECLARATION_RULES
        .iter()
        .map(|s| s.to_string())
        .collect();

    // argv[0] is the binary name.  Walk the rest looking for known flags.
    let mut i = 1;
    while i < argv.len() {
        if argv[i] == "--help" || argv[i] == "-h" {
            print_help();
            return;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--output") {
            output_path = Some(v);
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--lang-id") {
            lang_id = v;
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--lang-name") {
            lang_name = v;
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--extensions") {
            extensions = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--line-comment") {
            line_comment = v;
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--block-comment-start") {
            block_comment_start = v;
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--block-comment-end") {
            block_comment_end = v;
            continue;
        }
        if let Some(v) = take_flag(&argv, &mut i, "--declaration-rules") {
            declaration_rules = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }

        eprintln!("twig-spec-dump: unknown argument: {}", argv[i]);
        eprintln!("Run with --help for the supported flags.");
        process::exit(2);
    }

    // Validation: extensions must not have leading dots — they're
    // supposed to be `"twig"`, not `".twig"`.  Catching this early
    // means downstream consumers (VS Code extension generator) see
    // the format they expect.
    for ext in &extensions {
        if ext.starts_with('.') {
            eprintln!(
                "twig-spec-dump: --extensions entries must NOT include a leading dot ('{}' is invalid; use '{}')",
                ext,
                &ext[1..]
            );
            process::exit(2);
        }
    }

    let meta = SpecMetadata {
        language_id: lang_id,
        language_name: lang_name,
        file_extensions: extensions,
        line_comment,
        block_comment_start,
        block_comment_end,
        declaration_rules,
    };

    // The two grammars come from the build-time-compiled artifacts in
    // twig-lexer and twig-parser.  No file I/O, no parsing — both are
    // OnceLock-wrapped statics that materialise on first access.
    let token_grammar = twig_token_grammar_spec();
    let parser_grammar = twig_grammar();

    let value = dump_language_spec(token_grammar, Some(parser_grammar), &meta);
    let pretty = match serde_json::to_string_pretty(&value) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("twig-spec-dump: JSON serialisation failed: {}", e);
            process::exit(1);
        }
    };

    match output_path {
        Some(p) => {
            if let Err(e) = std::fs::write(&p, format!("{}\n", pretty)) {
                eprintln!("twig-spec-dump: failed to write {}: {}", p, e);
                process::exit(1);
            }
        }
        None => {
            println!("{}", pretty);
        }
    }
}

fn print_help() {
    println!(
        r#"twig-spec-dump — emit the Twig language spec as JSON

Reads the build-time-compiled token and parser grammars baked into
twig-lexer and twig-parser, and prints a JSON document describing the
language for downstream editor tooling (VS Code extension generator,
treesitter, syntax highlighters).  No runtime file I/O — the canonical
twig.tokens / twig.grammar files are build-time-only and don't need
to be present at runtime.

USAGE
    twig-spec-dump [FLAGS]

FLAGS
    -h, --help                     Show this help and exit
        --output PATH              Write JSON to PATH (default: stdout)
        --lang-id ID               Language slug (default: "twig")
        --lang-name NAME           Display name  (default: "Twig")
        --extensions LIST          Comma-separated extensions, no leading
                                   dots (default: "twig,tw")
        --line-comment STR         Line-comment marker (default: ";")
        --block-comment-start S    Block-comment open  (default: none)
        --block-comment-end   S    Block-comment close (default: none)
        --declaration-rules LIST   Comma-separated grammar rule names that
                                   LSP "document symbols" should surface
                                   (default: "define,module_form")

EXAMPLES
    twig-spec-dump > twig.spec.json
    twig-spec-dump --output twig.spec.json
    twig-spec-dump --lang-name "Twig 2.0" > twig.spec.json
"#
    );
}
