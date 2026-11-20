//! Fail-closed source-export selection for refusal-only frontend evidence.
//!
//! The caller supplies the exact `warrant-src-v1` frame it hashes. This
//! module parses those bytes directly, so hashing and frontend probing cannot
//! accidentally observe different source snapshots.

use crate::lower_source_file;
use crate::rust_subset::{parse_rust_file, Lexer, Token, TokenKind};

const FRAME_MAGIC: &[u8] = b"warrant-src-v1\0";
const MAX_FRAME_FILES: usize = 4096;
const MAX_SOURCE_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOURCE_TREE_BYTES: usize = 64 * 1024 * 1024;
const MAX_FRAME_BYTES: usize = 128 * 1024 * 1024;
const MAX_FRAME_PATH_BYTES: usize = 16 * 1024;
pub const MAX_CARGO_MANIFEST_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendOutcome {
    Refused(String),
    Accepted,
}

fn u64_at(bytes: &[u8], pos: &mut usize) -> Result<u64, String> {
    let end = pos.checked_add(8).ok_or("source frame offset overflow")?;
    let raw = bytes
        .get(*pos..end)
        .ok_or("truncated source frame length")?;
    *pos = end;
    Ok(u64::from_le_bytes(
        raw.try_into().expect("eight-byte slice"),
    ))
}

pub fn crate_root_source_from_snapshot(frame: &[u8]) -> Result<&str, String> {
    if frame.len() > MAX_FRAME_BYTES {
        return Err("source frame exceeds supported byte limit".into());
    }
    if !frame.starts_with(FRAME_MAGIC) {
        return Err("source frame has an unsupported header".into());
    }
    let mut pos = FRAME_MAGIC.len();
    let count = usize::try_from(u64_at(frame, &mut pos)?)
        .map_err(|_| "source frame file count is too large")?;
    if count == 0 || count > MAX_FRAME_FILES {
        return Err("source frame file count is outside supported limits".into());
    }
    let mut previous: Option<&[u8]> = None;
    let mut found = None;
    let mut total_content = 0usize;
    let mut total_paths = 0usize;
    for _ in 0..count {
        let path_len = usize::try_from(u64_at(frame, &mut pos)?)
            .map_err(|_| "source frame path length is too large")?;
        if path_len > MAX_FRAME_PATH_BYTES {
            return Err("source frame path exceeds supported length".into());
        }
        total_paths = total_paths
            .checked_add(path_len)
            .ok_or("source frame path size overflow")?;
        if total_paths > MAX_SOURCE_TREE_BYTES {
            return Err("source frame exceeds aggregate path limit".into());
        }
        let path_end = pos
            .checked_add(path_len)
            .ok_or("source frame offset overflow")?;
        let path = frame
            .get(pos..path_end)
            .ok_or("truncated source frame path")?;
        pos = path_end;
        let path_text = std::str::from_utf8(path).map_err(|_| "source frame path is not UTF-8")?;
        if path_text.is_empty()
            || path_text.starts_with('/')
            || path_text.contains('\\')
            || path_text.contains(':')
            || path_text
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || !path_text.ends_with(".rs")
        {
            return Err("source frame contains an invalid Rust source path".into());
        }
        if previous.is_some_and(|old| old >= path) {
            return Err("source frame paths are not strictly sorted and unique".into());
        }
        previous = Some(path);
        let content_len = usize::try_from(u64_at(frame, &mut pos)?)
            .map_err(|_| "source frame content length is too large")?;
        if content_len > MAX_SOURCE_FILE_BYTES {
            return Err("source frame contains an oversized Rust file".into());
        }
        total_content = total_content
            .checked_add(content_len)
            .ok_or("source frame content size overflow")?;
        if total_content > MAX_SOURCE_TREE_BYTES {
            return Err("source frame exceeds aggregate content limit".into());
        }
        let content_end = pos
            .checked_add(content_len)
            .ok_or("source frame offset overflow")?;
        let content = frame
            .get(pos..content_end)
            .ok_or("truncated source frame content")?;
        pos = content_end;
        if path == b"lib.rs" {
            found = Some(content);
        }
    }
    if pos != frame.len() {
        return Err("source frame has trailing bytes".into());
    }
    let bytes = found.ok_or("source frame has no crate-root lib.rs")?;
    std::str::from_utf8(bytes).map_err(|_| "crate-root lib.rs is not UTF-8".into())
}

/// Accept only the Cargo default `src/lib.rs` target. This deliberately
/// supports the normal manifest form used by the fixed-point pilot and
/// rejects custom target declarations rather than approximating Cargo's TOML
/// interpretation.
fn validate_default_lib_target(cargo: &[u8]) -> Result<(), String> {
    if cargo.len() > MAX_CARGO_MANIFEST_BYTES {
        return Err("Cargo.toml exceeds the source-equivalence manifest limit".into());
    }
    let text = std::str::from_utf8(cargo).map_err(|_| "Cargo.toml is not UTF-8")?;
    let manifest: toml::Value = text
        .parse()
        .map_err(|_| "Cargo.toml is invalid TOML or exceeds the supported parser profile")?;
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or("Cargo.toml has no package table")?;
    package
        .get("name")
        .and_then(toml::Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .ok_or("Cargo.toml package name is missing or empty")?;
    if manifest.get("lib").is_some() {
        return Err("custom Cargo [lib] target is unsupported".into());
    }
    if let Some(autolib) = package.get("autolib") {
        if !autolib
            .as_bool()
            .ok_or("Cargo package.autolib must be a boolean")?
        {
            return Err("Cargo package.autolib=false disables the default library target".into());
        }
    }
    Ok(())
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().map_err(|e| {
            format!(
                "Rust-subset lexer refused at {}:{}: {}",
                e.span.line, e.span.col, e.message
            )
        })?;
        let done = token.kind == TokenKind::Eof;
        tokens.push(token);
        if done {
            return Ok(tokens);
        }
        if tokens.len() > 65_536 {
            return Err("Rust-subset lexer token limit exceeded".into());
        }
    }
}

fn close_group(tokens: &[Token], start: usize) -> Result<usize, String> {
    let close = match tokens.get(start).map(|t| &t.kind) {
        Some(TokenKind::LParen) => TokenKind::RParen,
        Some(TokenKind::LBrace) => TokenKind::RBrace,
        Some(TokenKind::LBracket) => TokenKind::RBracket,
        _ => return Err("expected Rust delimiter".into()),
    };
    let mut stack = vec![close];
    let mut i = start + 1;
    while i < tokens.len() {
        let kind = &tokens[i].kind;
        match kind {
            TokenKind::LParen => stack.push(TokenKind::RParen),
            TokenKind::LBrace => stack.push(TokenKind::RBrace),
            TokenKind::LBracket => stack.push(TokenKind::RBracket),
            TokenKind::RParen | TokenKind::RBrace | TokenKind::RBracket => {
                let expected = stack.pop().ok_or("unbalanced Rust delimiter")?;
                if *kind != expected {
                    return Err("mismatched Rust delimiters".into());
                }
                if stack.is_empty() {
                    return Ok(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err("unclosed Rust delimiter".into())
}

fn attr_is_conditional(tokens: &[Token], start: usize, end: usize) -> bool {
    start <= end
        && tokens[start..=end]
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Ident(n) if n == "cfg" || n == "cfg_attr"))
}

fn attr_is_supported(tokens: &[Token], start: usize, end: usize, inner: bool) -> bool {
    if start > end {
        return false;
    }
    const OUTER: &[&str] = &[
        "allow",
        "warn",
        "deny",
        "forbid",
        "expect",
        "inline",
        "cold",
        "must_use",
        "track_caller",
        "doc",
        "deprecated",
        "derive",
    ];
    const INNER: &[&str] = &[
        "allow",
        "warn",
        "deny",
        "forbid",
        "expect",
        "doc",
        "no_std",
        "no_main",
        "feature",
        "recursion_limit",
        "type_length_limit",
        "crate_name",
        "crate_type",
        "windows_subsystem",
    ];
    let Some(Token {
        kind: TokenKind::Ident(name),
        ..
    }) = tokens.get(start)
    else {
        return false;
    };
    let supported = if inner { INNER } else { OUTER };
    supported.contains(&name.as_str())
        && !tokens[start..=end]
            .iter()
            .any(|t| t.kind == TokenKind::ColonColon)
        && !tokens[start..=end]
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Ident(n) if n == "cfg" || n == "cfg_attr"))
}

/// Parse the deliberately small type grammar accepted in the pilot exports:
/// paths with optional one-level, comma-separated simple type arguments.
fn parse_simple_type(tokens: &[Token], pos: &mut usize) -> Result<(), String> {
    if !matches!(tokens.get(*pos).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
        return Err("unsupported public export signature type".into());
    }
    *pos += 1;
    while tokens
        .get(*pos)
        .is_some_and(|t| t.kind == TokenKind::ColonColon)
    {
        *pos += 1;
        if !matches!(tokens.get(*pos).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
            return Err("unsupported public export type path".into());
        }
        *pos += 1;
    }
    if tokens.get(*pos).is_some_and(|t| t.kind == TokenKind::Lt) {
        *pos += 1;
        let mut args = 0usize;
        loop {
            if tokens.get(*pos).is_some_and(|t| t.kind == TokenKind::Gt) {
                if args == 0 {
                    return Err("empty public export type arguments are unsupported".into());
                }
                *pos += 1;
                break;
            }
            if args > 0 {
                if !tokens.get(*pos).is_some_and(|t| t.kind == TokenKind::Comma) {
                    return Err("nested or unsupported public export type arguments".into());
                }
                *pos += 1;
            }
            if !matches!(tokens.get(*pos).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
                return Err("nested or unsupported public export type arguments".into());
            }
            *pos += 1;
            args += 1;
        }
    }
    Ok(())
}

fn validate_export_signature(
    tokens: &[Token],
    start: usize,
    export: &str,
) -> Result<usize, String> {
    let params_open = start + 3;
    if tokens.get(params_open).map(|t| &t.kind) != Some(&TokenKind::LParen) {
        return Err("generic or unsupported public export signatures are not eligible".into());
    }
    let params_close = close_group(tokens, params_open)?;
    let mut pos = params_open + 1;
    while pos < params_close {
        if !matches!(tokens.get(pos).map(|t| &t.kind), Some(TokenKind::Ident(_))) {
            return Err(format!("unsupported parameter in public export `{export}`"));
        }
        pos += 1;
        if tokens.get(pos).map(|t| &t.kind) != Some(&TokenKind::Colon) {
            return Err(format!(
                "unsupported parameter binding in public export `{export}`"
            ));
        }
        pos += 1;
        parse_simple_type(tokens, &mut pos)?;
        if pos == params_close {
            break;
        }
        if tokens.get(pos).map(|t| &t.kind) != Some(&TokenKind::Comma) {
            return Err(format!(
                "unsupported parameter list in public export `{export}`"
            ));
        }
        pos += 1;
    }
    pos = params_close + 1;
    if tokens.get(pos).is_some_and(|t| t.kind == TokenKind::Arrow) {
        pos += 1;
        parse_simple_type(tokens, &mut pos)?;
    }
    if tokens.get(pos).map(|t| &t.kind) != Some(&TokenKind::LBrace) {
        return Err(format!(
            "unsupported return or where clause in public export `{export}`"
        ));
    }
    Ok(pos)
}

/// Select exactly one unconditional, direct crate-root `pub fn export`.
/// Nested token groups and macro input are skipped as indivisible groups.
fn direct_export(source: &str, export: &str) -> Result<String, String> {
    let tokens = lex(source)?;
    let mut i = 0;
    let mut found: Option<(usize, usize)> = None;
    while i < tokens.len() && tokens[i].kind != TokenKind::Eof {
        // Consume item attributes. Inner conditional attributes affect the
        // entire crate; outer conditional attributes affect the next item.
        let mut conditional = false;
        while tokens[i].kind == TokenKind::Hash {
            let inner = tokens.get(i + 1).is_some_and(|t| t.kind == TokenKind::Bang);
            let bracket = i + if inner { 2 } else { 1 };
            if tokens.get(bracket).map(|t| &t.kind) != Some(&TokenKind::LBracket) {
                return Err("unrecognized Rust attribute syntax".into());
            }
            let end = close_group(&tokens, bracket)?;
            if attr_is_conditional(&tokens, bracket + 1, end.saturating_sub(1)) {
                if inner {
                    return Err("crate-level cfg/cfg_attr is unsupported".into());
                }
                conditional = true;
            }
            if !attr_is_supported(&tokens, bracket + 1, end.saturating_sub(1), inner) {
                return Err(
                    "unrecognized or potentially active Rust attribute is unsupported".into(),
                );
            }
            i = end + 1;
        }
        if tokens[i].kind == TokenKind::Eof {
            break;
        }
        let item_start = i;
        if !conditional
            && tokens[i].kind == TokenKind::Pub
            && tokens.get(i + 1).is_some_and(|t| t.kind == TokenKind::Fn)
            && tokens
                .get(i + 2)
                .is_some_and(|t| t.kind == TokenKind::Ident(export.to_owned()))
        {
            let body = validate_export_signature(&tokens, i, export)?;
            let end = close_group(&tokens, body)?;
            let span = tokens[item_start].span.start..tokens[end].span.end;
            source.get(span).ok_or("export source span is invalid")?;
            if found.replace((item_start, end)).is_some() {
                return Err(format!("ambiguous direct public export `{export}`"));
            }
            // Skip this item; a duplicate is still detected by the general
            // item scanner below, so continue from the closing brace.
            i = end + 1;
            continue;
        }
        // Skip to the end of this root item. Any delimiter group is atomic,
        // preventing fake `pub fn` text in macro arguments or nested modules.
        let mut j = i;
        while j < tokens.len() && tokens[j].kind != TokenKind::Eof {
            if tokens[j].kind == TokenKind::Bang {
                return Err(
                    "root-level macro invocation is unsupported for active-export resolution"
                        .into(),
                );
            }
            match tokens[j].kind {
                TokenKind::LParen | TokenKind::LBrace | TokenKind::LBracket => {
                    let end = close_group(&tokens, j)?;
                    if tokens[j].kind == TokenKind::LBrace {
                        j = end + 1;
                        break;
                    }
                    j = end + 1;
                }
                TokenKind::Semi => {
                    j += 1;
                    break;
                }
                _ => j += 1,
            }
        }
        if j == i {
            return Err("cannot advance Rust root-item scanner".into());
        }
        i = j;
    }
    let Some((start, end)) = found else {
        return Err(format!(
            "crate root has no unconditional direct public export `{export}`"
        ));
    };
    let span = tokens[start].span.start..tokens[end].span.end;
    Ok(source
        .get(span)
        .ok_or("export source span is invalid")?
        .to_owned())
}

/// Return the actual per-export frontend refusal for one captured source tree.
/// `Ok` means the current supported frontend refused; an accepted function is
/// an error because this function supplies no equivalence proof.
pub fn probe_export_from_snapshot(
    frame: &[u8],
    cargo_toml: &[u8],
    export: &str,
) -> Result<FrontendOutcome, String> {
    validate_default_lib_target(cargo_toml)?;
    let source = crate_root_source_from_snapshot(frame)?;
    let function = direct_export(source, export)?;
    let file = parse_rust_file(&function).map_err(|e| {
        let line = function
            .lines()
            .nth(e.span.line.saturating_sub(1))
            .unwrap_or("")
            .trim();
        format!(
            "parse refused at {}:{}: {} (line: `{}`)",
            e.span.line, e.span.col, e.message, line
        )
    });
    let reason = match file {
        Err(reason) => reason,
        Ok(file) => match lower_source_file(&file, &[]) {
            Ok(_) => return Ok(FrontendOutcome::Accepted),
            Err(e) => format!("WIR lowering refused: {}", e.message),
        },
    };
    Ok(FrontendOutcome::Refused(reason))
}

pub fn reproduce_refusal_from_snapshot(
    frame: &[u8],
    cargo_toml: &[u8],
    export: &str,
) -> Result<String, String> {
    match probe_export_from_snapshot(frame, cargo_toml, export)? {
        FrontendOutcome::Refused(reason) => Ok(reason),
        FrontendOutcome::Accepted => Err(format!(
            "source-equivalence fallback is stale: frontend now accepts direct crate-root export `{export}`"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_test_bootstrap::manifest::frame_sources;

    const CARGO: &[u8] = b"[package]\nname='probe'\nversion='0.1.0'\nedition='2021'\n";
    fn frame(source: &str) -> Vec<u8> {
        frame_sources(&[("lib.rs".into(), source.as_bytes().to_vec())])
    }
    fn selected(source: &str) -> Result<String, String> {
        direct_export(source, "run")
    }

    #[test]
    fn only_unconditional_direct_root_item_is_selected() {
        let source = "const X: () = ((pub fn run() {}));\nconst Y: [(); 1] = [pub fn run() {}];\n#[derive(Clone)]\nstruct S;\npub fn run() { unsupported!(); }\n";
        assert!(selected(source).unwrap().contains("pub fn run"));
    }

    #[test]
    fn conditional_and_non_root_candidates_fail_closed() {
        for source in [
            "#[cfg(any())]\npub fn run() {}",
            "#[cfg_attr(unix, allow(dead_code))]\npub fn run() {}",
            "#![cfg(unix)]\npub fn run() {}",
            "mod child { pub fn run() {} }",
            "m!((pub fn run() {}));",
            "m!([pub fn run() {}]);",
        ] {
            assert!(selected(source).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn duplicates_and_custom_cargo_targets_fail_closed() {
        assert!(selected("pub fn run() {}\npub fn run() {}").is_err());
        for cargo in [
            b"[lib]\npath='other.rs'\n".as_slice(),
            b"['lib']\npath='other.rs'\n",
            b"[\"lib\"]\npath='other.rs'\n",
            b"[package]\nautolib=false\n",
            b"package.autolib = false\n",
            b"lib.path = 'other.rs'\n",
            b"[workspace]\nmembers=[]\n",
            b"[package]\nname='probe'\ndescription='''\n[dependencies]\nname='x'\n'''\nautolib=false\n",
            b"[package]\nname=\"probe\"\ndescription=\"\"\"\n[dependencies]\nname=\"x\"\n\"\"\"\nautolib=false\n",
            b"[package]\nname=''\n",
        ] {
            assert!(
                reproduce_refusal_from_snapshot(&frame("pub fn run() {}"), cargo, "run").is_err()
            );
        }
    }

    #[test]
    fn conditional_and_unknown_candidate_attributes_are_rejected() {
        assert!(selected("#[cfg_attr(unix, allow(dead_code))]\npub fn run() {}").is_err());
        assert!(selected("#[plugin_macro]\npub fn run() {}").is_err());
        assert!(selected("#[allow::plugin_macro]\npub fn run() {}").is_err());
        assert!(selected("#![plugin_macro]\npub fn run() {}").is_err());
        assert!(selected("#![allow::plugin_macro]\npub fn run() {}").is_err());
        assert!(selected("#[derive(Clone)]\nstruct S;\npub fn run() {}").is_ok());
    }

    #[test]
    fn probe_uses_the_supplied_snapshot_bytes() {
        let captured = frame("pub fn run() { unsupported!(); }");
        // A different live/compile-time source is irrelevant: there is no path
        // or filesystem parameter to read it from.
        let refusal = reproduce_refusal_from_snapshot(&captured, CARGO, "run").unwrap();
        assert!(refusal.contains("refused"));
        assert!(matches!(
            probe_export_from_snapshot(&frame("pub fn run() {}"), CARGO, "run"),
            Ok(FrontendOutcome::Accepted)
        ));
        assert!(matches!(
            probe_export_from_snapshot(&captured, CARGO, "run"),
            Ok(FrontendOutcome::Refused(_))
        ));
        assert!(probe_export_from_snapshot(&captured, CARGO, "missing").is_err());
        assert!(probe_export_from_snapshot(&captured, b"[workspace]\nmembers=[]", "run").is_err());
    }

    #[test]
    fn generic_function_and_nested_generic_signatures_fail_closed() {
        let source =
            "struct Arr<const N: usize>;\npub fn run<const N: usize>() -> Arr<{N}> { Arr }";
        assert!(selected(source).is_err());
        assert!(selected("pub fn run() -> Option<Option<u8>> { None }").is_err());
        let one_level =
            selected("pub fn run(value: Q16) -> Result<Q16, Q16Error> { value }").unwrap();
        assert!(one_level.ends_with("-> Result<Q16, Q16Error> { value }"));
    }

    #[test]
    fn macro_return_type_is_not_truncated_into_a_fake_frontend_refusal() {
        assert!(direct_export("pub fn run() -> ty!{} { 1 }", "run").is_err());
    }

    #[test]
    fn malformed_or_mutated_frames_fail_closed_without_panicking() {
        let valid = frame("pub fn run() { unsupported!(); }");
        for end in 0..valid.len() {
            let truncated = &valid[..end];
            assert!(
                std::panic::catch_unwind(|| crate_root_source_from_snapshot(truncated)).is_ok()
            );
            assert!(crate_root_source_from_snapshot(truncated).is_err());
        }
        let changed = frame("pub fn run() {}");
        assert!(matches!(
            probe_export_from_snapshot(&changed, CARGO, "run"),
            Ok(FrontendOutcome::Accepted)
        ));

        let duplicate = frame_sources(&[
            ("lib.rs".into(), b"pub fn run() {}".to_vec()),
            ("lib.rs".into(), b"pub fn run() {}".to_vec()),
        ]);
        assert!(crate_root_source_from_snapshot(&duplicate).is_err());
        let invalid_path = frame_sources(&[("../lib.rs".into(), b"pub fn run() {}".to_vec())]);
        assert!(crate_root_source_from_snapshot(&invalid_path).is_err());
    }
}
