//! Comprehensive tests for the ls00 LSP framework.
//!
//! # Test Strategy
//!
//! We test the framework with mock bridges that implement various subsets
//! of the `LanguageBridge` trait. This lets us exercise every code path
//! without needing a real language implementation.
//!
//! # Test Coverage Areas
//!
//!  1. UTF-16 offset conversion (critical for correctness)
//!  2. DocumentManager open/change/close operations
//!  3. ParseCache hit/miss behavior
//!  4. Semantic token encoding (the delta format)
//!  5. Capabilities advertisement (only what the bridge supports)
//!  6. Full LSP lifecycle via JSON-RPC round-trips

use coding_adventures_json_rpc::message::{parse_message, Message};
use coding_adventures_ls00::capabilities::*;
use coding_adventures_ls00::document_manager::*;
use coding_adventures_ls00::language_bridge::LanguageBridge;
use coding_adventures_ls00::lsp_errors::*;
use coding_adventures_ls00::parse_cache::*;
use coding_adventures_ls00::server::LspServer;
use coding_adventures_ls00::types::*;
use serde_json::{json, Value};
use std::any::Any;
use std::io::{BufReader, Cursor};

// ============================================================================
// Mock Bridges
// ============================================================================

/// MockBridge implements the required `LanguageBridge` methods plus hover
/// and document symbols. Used to test basic capability advertisement.
struct MockBridge {
    hover_result: Option<HoverResult>,
}

impl LanguageBridge for MockBridge {
    fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        let mut col = 1;
        for word in source.split_whitespace() {
            tokens.push(Token {
                token_type: "WORD".to_string(),
                value: word.to_string(),
                line: 1,
                column: col,
            });
            col += word.len() as i32 + 1;
        }
        Ok(tokens)
    }

    fn parse(
        &self,
        source: &str,
    ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
        let mut diags = Vec::new();
        if source.contains("ERROR") {
            diags.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
                severity: DiagnosticSeverity::Error,
                message: "syntax error: unexpected ERROR token".to_string(),
                code: None,
            });
        }
        Ok((Box::new(source.to_string()), diags))
    }

    fn hover(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Option<HoverResult>, String>> {
        Some(Ok(self.hover_result.clone()))
    }

    fn supports_hover(&self) -> bool {
        true
    }

    fn document_symbols(
        &self,
        _ast: &dyn Any,
    ) -> Option<Result<Vec<DocumentSymbol>, String>> {
        Some(Ok(vec![DocumentSymbol {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 1,
                },
            },
            selection_range: Range {
                start: Position {
                    line: 0,
                    character: 9,
                },
                end: Position {
                    line: 0,
                    character: 13,
                },
            },
            children: vec![DocumentSymbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                range: Range {
                    start: Position {
                        line: 1,
                        character: 4,
                    },
                    end: Position {
                        line: 1,
                        character: 12,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: 1,
                        character: 8,
                    },
                    end: Position {
                        line: 1,
                        character: 9,
                    },
                },
                children: vec![],
            }],
        }]))
    }

    fn supports_document_symbols(&self) -> bool {
        true
    }
}

/// MinimalBridge implements ONLY the required LanguageBridge methods.
/// Used to test that optional capabilities are NOT advertised.
struct MinimalBridge;

impl LanguageBridge for MinimalBridge {
    fn tokenize(&self, _source: &str) -> Result<Vec<Token>, String> {
        Ok(Vec::new())
    }

    fn parse(
        &self,
        source: &str,
    ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
        Ok((Box::new(source.to_string()), Vec::new()))
    }
}

/// FullMockBridge implements all optional interfaces.
struct FullMockBridge;

impl LanguageBridge for FullMockBridge {
    fn tokenize(&self, source: &str) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        let mut col = 1;
        for word in source.split_whitespace() {
            tokens.push(Token {
                token_type: "WORD".to_string(),
                value: word.to_string(),
                line: 1,
                column: col,
            });
            col += word.len() as i32 + 1;
        }
        Ok(tokens)
    }

    fn parse(
        &self,
        source: &str,
    ) -> Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String> {
        let mut diags = Vec::new();
        if source.contains("ERROR") {
            diags.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
                severity: DiagnosticSeverity::Error,
                message: "syntax error".to_string(),
                code: None,
            });
        }
        Ok((Box::new(source.to_string()), diags))
    }

    fn hover(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Option<HoverResult>, String>> {
        Some(Ok(Some(HoverResult {
            contents: "**hover** info".to_string(),
            range: None,
        })))
    }
    fn supports_hover(&self) -> bool {
        true
    }

    fn definition(
        &self,
        _ast: &dyn Any,
        pos: Position,
        uri: &str,
    ) -> Option<Result<Option<Location>, String>> {
        Some(Ok(Some(Location {
            uri: uri.to_string(),
            range: Range {
                start: pos.clone(),
                end: pos,
            },
        })))
    }
    fn supports_definition(&self) -> bool {
        true
    }

    fn references(
        &self,
        _ast: &dyn Any,
        pos: Position,
        uri: &str,
        _include_decl: bool,
    ) -> Option<Result<Vec<Location>, String>> {
        Some(Ok(vec![Location {
            uri: uri.to_string(),
            range: Range {
                start: pos.clone(),
                end: pos,
            },
        }]))
    }
    fn supports_references(&self) -> bool {
        true
    }

    fn completion(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Vec<CompletionItem>, String>> {
        Some(Ok(vec![CompletionItem {
            label: "foo".to_string(),
            kind: Some(CompletionItemKind::Function),
            detail: Some("() void".to_string()),
            documentation: None,
            insert_text: None,
            insert_text_format: None,
        }]))
    }
    fn supports_completion(&self) -> bool {
        true
    }

    fn rename(
        &self,
        _ast: &dyn Any,
        pos: Position,
        new_name: &str,
    ) -> Option<Result<Option<WorkspaceEdit>, String>> {
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            "file:///test.txt".to_string(),
            vec![TextEdit {
                range: Range {
                    start: pos.clone(),
                    end: pos,
                },
                new_text: new_name.to_string(),
            }],
        );
        Some(Ok(Some(WorkspaceEdit { changes })))
    }
    fn supports_rename(&self) -> bool {
        true
    }

    fn semantic_tokens(
        &self,
        _source: &str,
        tokens: &[Token],
    ) -> Option<Result<Vec<SemanticToken>, String>> {
        let result: Vec<SemanticToken> = tokens
            .iter()
            .map(|tok| SemanticToken {
                line: tok.line - 1,
                character: tok.column - 1,
                length: tok.value.len() as i32,
                token_type: "variable".to_string(),
                modifiers: vec![],
            })
            .collect();
        Some(Ok(result))
    }
    fn supports_semantic_tokens(&self) -> bool {
        true
    }

    fn document_symbols(
        &self,
        _ast: &dyn Any,
    ) -> Option<Result<Vec<DocumentSymbol>, String>> {
        Some(Ok(vec![DocumentSymbol {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 10, character: 1 },
            },
            selection_range: Range {
                start: Position { line: 0, character: 9 },
                end: Position { line: 0, character: 13 },
            },
            children: vec![],
        }]))
    }
    fn supports_document_symbols(&self) -> bool {
        true
    }

    fn folding_ranges(
        &self,
        _ast: &dyn Any,
    ) -> Option<Result<Vec<FoldingRange>, String>> {
        Some(Ok(vec![FoldingRange {
            start_line: 0,
            end_line: 5,
            kind: Some("region".to_string()),
        }]))
    }
    fn supports_folding_ranges(&self) -> bool {
        true
    }

    fn signature_help(
        &self,
        _ast: &dyn Any,
        _pos: Position,
    ) -> Option<Result<Option<SignatureHelpResult>, String>> {
        Some(Ok(Some(SignatureHelpResult {
            signatures: vec![SignatureInformation {
                label: "foo(a int, b string)".to_string(),
                documentation: None,
                parameters: vec![
                    ParameterInformation {
                        label: "a int".to_string(),
                        documentation: None,
                    },
                    ParameterInformation {
                        label: "b string".to_string(),
                        documentation: None,
                    },
                ],
            }],
            active_signature: 0,
            active_parameter: 0,
        })))
    }
    fn supports_signature_help(&self) -> bool {
        true
    }

    fn format(&self, source: &str) -> Option<Result<Vec<TextEdit>, String>> {
        Some(Ok(vec![TextEdit {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 999, character: 0 },
            },
            new_text: source.to_string(),
        }]))
    }
    fn supports_format(&self) -> bool {
        true
    }
}

// ============================================================================
// UTF-16 Offset Conversion Tests
// ============================================================================

/// Verifies the critical UTF-16 -> byte conversion.
///
/// This is the most important correctness test in the entire package. If this
/// function is wrong, every feature that depends on cursor position will be
/// wrong: hover, go-to-definition, references, completion, rename.
#[test]
fn test_convert_utf16_offset_ascii_simple() {
    let text = "hello world";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 0, 6);
    assert_eq!(byte_off, 6);
}

#[test]
fn test_convert_utf16_offset_start_of_file() {
    let byte_off = convert_utf16_offset_to_byte_offset("abc", 0, 0);
    assert_eq!(byte_off, 0);
}

#[test]
fn test_convert_utf16_offset_end_of_short_string() {
    let byte_off = convert_utf16_offset_to_byte_offset("abc", 0, 3);
    assert_eq!(byte_off, 3);
}

#[test]
fn test_convert_utf16_offset_second_line() {
    // "hello\nworld" -- line 1 starts at byte 6
    let byte_off = convert_utf16_offset_to_byte_offset("hello\nworld", 1, 0);
    assert_eq!(byte_off, 6);
}

#[test]
fn test_convert_utf16_offset_emoji() {
    // "A🎸B"
    // UTF-8 bytes:  A (1) + 🎸 (4) + B (1) = 6 bytes
    // UTF-16 units: A (1) + 🎸 (2) + B (1) = 4 units
    // "B" is at UTF-16 character 3, byte offset 5.
    let text = "A\u{1F3B8}B";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 0, 3);
    assert_eq!(byte_off, 5);
}

#[test]
fn test_convert_utf16_offset_emoji_at_start() {
    // "🎸hello" -- 🎸 = 2 UTF-16 units, 4 UTF-8 bytes
    // "h" is at UTF-16 char 2, byte offset 4
    let text = "\u{1F3B8}hello";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 0, 2);
    assert_eq!(byte_off, 4);
}

#[test]
fn test_convert_utf16_offset_2byte_utf8_bmp() {
    // "cafe!" -- e with accent is U+00E9
    // UTF-8: 2 bytes. UTF-16: 1 code unit.
    // UTF-16 char 4 = byte offset 5 (c=1, a=1, f=1, e_accent=2 bytes)
    let text = "caf\u{00e9}!";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 0, 4);
    assert_eq!(byte_off, 5);
}

#[test]
fn test_convert_utf16_offset_multiline_with_emoji() {
    // line 0: "A🎸B\n" (1+4+1+1 = 7 bytes)
    // line 1: "hello"
    // "hello" starts at byte 7, char 0 on line 1
    let text = "A\u{1F3B8}B\nhello";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 1, 0);
    assert_eq!(byte_off, 7);
}

#[test]
fn test_convert_utf16_offset_beyond_line_end() {
    // Character past end of line should clamp to the newline.
    let text = "ab\ncd";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 0, 100);
    assert_eq!(byte_off, 2);
}

#[test]
fn test_convert_utf16_chinese_character() {
    // "中文" -- each Chinese char is 3 UTF-8 bytes, 1 UTF-16 code unit.
    // "文" is at UTF-16 char 1, byte offset 3.
    let text = "\u{4e2d}\u{6587}";
    let byte_off = convert_utf16_offset_to_byte_offset(text, 0, 1);
    assert_eq!(byte_off, 3);
}

// ============================================================================
// DocumentManager Tests
// ============================================================================

#[test]
fn test_document_manager_open() {
    let mut dm = DocumentManager::new();
    dm.open("file:///test.txt", "hello world", 1);

    let doc = dm.get("file:///test.txt").unwrap();
    assert_eq!(doc.text, "hello world");
    assert_eq!(doc.version, 1);
}

#[test]
fn test_document_manager_get_missing() {
    let dm = DocumentManager::new();
    assert!(dm.get("file:///nonexistent.txt").is_none());
}

#[test]
fn test_document_manager_close() {
    let mut dm = DocumentManager::new();
    dm.open("file:///test.txt", "hello", 1);
    dm.close("file:///test.txt");
    assert!(dm.get("file:///test.txt").is_none());
}

#[test]
fn test_document_manager_apply_changes_full_replacement() {
    let mut dm = DocumentManager::new();
    dm.open("file:///test.txt", "hello world", 1);

    dm.apply_changes(
        "file:///test.txt",
        &[TextChange {
            range: None,
            new_text: "goodbye world".to_string(),
        }],
        2,
    )
    .unwrap();

    let doc = dm.get("file:///test.txt").unwrap();
    assert_eq!(doc.text, "goodbye world");
    assert_eq!(doc.version, 2);
}

#[test]
fn test_document_manager_apply_changes_incremental() {
    let mut dm = DocumentManager::new();
    dm.open("file:///test.txt", "hello world", 1);

    // Replace "world" with "Go"
    dm.apply_changes(
        "file:///test.txt",
        &[TextChange {
            range: Some(Range {
                start: Position { line: 0, character: 6 },
                end: Position { line: 0, character: 11 },
            }),
            new_text: "Go".to_string(),
        }],
        2,
    )
    .unwrap();

    let doc = dm.get("file:///test.txt").unwrap();
    assert_eq!(doc.text, "hello Go");
}

#[test]
fn test_document_manager_apply_changes_not_open() {
    let mut dm = DocumentManager::new();
    let result = dm.apply_changes(
        "file:///notopen.txt",
        &[TextChange {
            range: None,
            new_text: "x".to_string(),
        }],
        1,
    );
    assert!(result.is_err());
}

#[test]
fn test_document_manager_incremental_with_emoji() {
    // "A🎸B" -- emoji is 4 UTF-8 bytes, 2 UTF-16 code units
    // Replace "B" (UTF-16 char 3) with "X"
    let mut dm = DocumentManager::new();
    dm.open("file:///test.txt", "A\u{1F3B8}B", 1);

    dm.apply_changes(
        "file:///test.txt",
        &[TextChange {
            range: Some(Range {
                start: Position { line: 0, character: 3 },
                end: Position { line: 0, character: 4 },
            }),
            new_text: "X".to_string(),
        }],
        2,
    )
    .unwrap();

    let doc = dm.get("file:///test.txt").unwrap();
    assert_eq!(doc.text, "A\u{1F3B8}X");
}

#[test]
fn test_document_manager_incremental_multi_change() {
    let mut dm = DocumentManager::new();
    dm.open("uri", "hello world", 1);

    dm.apply_changes(
        "uri",
        &[TextChange {
            range: Some(Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            }),
            new_text: "hi".to_string(),
        }],
        2,
    )
    .unwrap();

    let doc = dm.get("uri").unwrap();
    assert_eq!(doc.text, "hi world");
}

// ============================================================================
// ParseCache Tests
// ============================================================================

#[test]
fn test_parse_cache_hit_and_miss() {
    let bridge = MockBridge { hover_result: None };
    let mut cache = ParseCache::new();

    // First call -- cache miss -> parses
    let r1 = cache.get_or_parse("file:///a.txt", 1, "hello", &bridge);
    assert!(r1.ast.is_some());
    assert!(r1.diagnostics.is_empty());

    // Second call same version -- cache hit (returns same result)
    let r2 = cache.get_or_parse("file:///a.txt", 1, "hello", &bridge);
    assert!(r2.ast.is_some());
    assert!(r2.diagnostics.is_empty());

    // Different version with ERROR source -- cache miss -> new result with diagnostics
    let r3 = cache.get_or_parse("file:///a.txt", 2, "hello ERROR world", &bridge);
    assert!(!r3.diagnostics.is_empty());
}

#[test]
fn test_parse_cache_evict() {
    let bridge = MockBridge { hover_result: None };
    let mut cache = ParseCache::new();

    // Parse a source with ERROR to get diagnostics
    let r1 = cache.get_or_parse("file:///a.txt", 1, "hello ERROR", &bridge);
    assert!(!r1.diagnostics.is_empty());

    cache.evict("file:///a.txt");

    // After eviction, same (uri, version) but different source should re-parse
    // (the eviction cleared the cache, so it will parse the new source)
    let r2 = cache.get_or_parse("file:///a.txt", 1, "hello", &bridge);
    assert!(r2.diagnostics.is_empty()); // no ERROR in new source -> no diagnostics
}

#[test]
fn test_parse_cache_diagnostics_populated() {
    let bridge = MockBridge { hover_result: None };
    let mut cache = ParseCache::new();

    let result = cache.get_or_parse("file:///a.txt", 1, "source with ERROR token", &bridge);
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn test_parse_cache_no_diagnostics_for_clean_source() {
    let bridge = MockBridge { hover_result: None };
    let mut cache = ParseCache::new();

    let result = cache.get_or_parse("file:///clean.txt", 1, "hello world", &bridge);
    assert!(result.diagnostics.is_empty());
}

// ============================================================================
// Capabilities Tests
// ============================================================================

#[test]
fn test_build_capabilities_minimal_bridge() {
    let bridge = MinimalBridge;
    let caps = build_capabilities(&bridge);

    assert_eq!(caps["textDocumentSync"], 2);

    let optional = [
        "hoverProvider",
        "definitionProvider",
        "referencesProvider",
        "completionProvider",
        "renameProvider",
        "documentSymbolProvider",
        "foldingRangeProvider",
        "signatureHelpProvider",
        "documentFormattingProvider",
        "semanticTokensProvider",
    ];
    for cap in &optional {
        assert!(caps.get(cap).is_none(), "minimal bridge should not advertise {}", cap);
    }
}

#[test]
fn test_build_capabilities_mock_bridge() {
    let bridge = MockBridge { hover_result: None };
    let caps = build_capabilities(&bridge);

    assert!(caps.get("hoverProvider").is_some());
    assert!(caps.get("documentSymbolProvider").is_some());
    // MockBridge doesn't support these
    assert!(caps.get("definitionProvider").is_none());
    assert!(caps.get("completionProvider").is_none());
}

#[test]
fn test_build_capabilities_full_bridge() {
    let bridge = FullMockBridge;
    let caps = build_capabilities(&bridge);

    let expected = [
        "textDocumentSync",
        "hoverProvider",
        "definitionProvider",
        "referencesProvider",
        "completionProvider",
        "renameProvider",
        "documentSymbolProvider",
        "foldingRangeProvider",
        "signatureHelpProvider",
        "documentFormattingProvider",
        "semanticTokensProvider",
    ];
    for cap in &expected {
        assert!(
            caps.get(cap).is_some(),
            "full bridge should advertise {}",
            cap
        );
    }
}

#[test]
fn test_build_capabilities_semantic_tokens_provider() {
    let bridge = FullMockBridge;
    let caps = build_capabilities(&bridge);

    let stp = &caps["semanticTokensProvider"];
    assert_eq!(stp["full"], true);
    assert!(stp["legend"]["tokenTypes"].is_array());
}

#[test]
fn test_semantic_token_legend_consistency() {
    let legend = semantic_token_legend();

    assert!(!legend.token_types.is_empty());
    assert!(!legend.token_modifiers.is_empty());

    let required = ["keyword", "string", "number", "variable", "function"];
    for rt in &required {
        assert!(
            legend.token_types.contains(&rt.to_string()),
            "legend missing required type {}",
            rt
        );
    }
}

// ============================================================================
// Semantic Token Encoding Tests
// ============================================================================

#[test]
fn test_encode_semantic_tokens_empty() {
    let data = encode_semantic_tokens(&[]);
    assert!(data.is_empty());
}

#[test]
fn test_encode_semantic_tokens_single_token() {
    let tokens = vec![SemanticToken {
        line: 0,
        character: 0,
        length: 5,
        token_type: "keyword".to_string(),
        modifiers: vec![],
    }];
    let data = encode_semantic_tokens(&tokens);

    assert_eq!(data.len(), 5);
    assert_eq!(data[0], 0); // deltaLine
    assert_eq!(data[1], 0); // deltaChar
    assert_eq!(data[2], 5); // length
    assert_eq!(data[3], 15); // keyword index
    assert_eq!(data[4], 0); // modifiers
}

#[test]
fn test_encode_semantic_tokens_multiple_same_line() {
    let tokens = vec![
        SemanticToken {
            line: 0,
            character: 0,
            length: 3,
            token_type: "keyword".to_string(),
            modifiers: vec![],
        },
        SemanticToken {
            line: 0,
            character: 4,
            length: 4,
            token_type: "function".to_string(),
            modifiers: vec!["declaration".to_string()],
        },
    ];
    let data = encode_semantic_tokens(&tokens);

    assert_eq!(data.len(), 10);
    // Token A
    assert_eq!(&data[0..5], &[0, 0, 3, 15, 0]);
    // Token B: deltaLine=0, deltaChar=4, length=4, function(12), declaration(bit0=1)
    assert_eq!(&data[5..10], &[0, 4, 4, 12, 1]);
}

#[test]
fn test_encode_semantic_tokens_multiple_lines() {
    let tokens = vec![
        SemanticToken {
            line: 0,
            character: 0,
            length: 3,
            token_type: "keyword".to_string(),
            modifiers: vec![],
        },
        SemanticToken {
            line: 2,
            character: 4,
            length: 5,
            token_type: "number".to_string(),
            modifiers: vec![],
        },
    ];
    let data = encode_semantic_tokens(&tokens);

    assert_eq!(data.len(), 10);
    // Token B: deltaLine=2, deltaChar=4 (absolute on new line), number=19
    assert_eq!(data[5], 2);
    assert_eq!(data[6], 4);
    assert_eq!(data[8], 19);
}

#[test]
fn test_encode_semantic_tokens_unsorted_input() {
    let tokens = vec![
        SemanticToken {
            line: 1,
            character: 0,
            length: 2,
            token_type: "number".to_string(),
            modifiers: vec![],
        },
        SemanticToken {
            line: 0,
            character: 0,
            length: 3,
            token_type: "keyword".to_string(),
            modifiers: vec![],
        },
    ];
    let data = encode_semantic_tokens(&tokens);

    assert_eq!(data.len(), 10);
    assert_eq!(data[3], 15); // first token should be keyword
    assert_eq!(data[8], 19); // second should be number
}

#[test]
fn test_encode_semantic_tokens_unknown_type() {
    let tokens = vec![
        SemanticToken {
            line: 0,
            character: 0,
            length: 3,
            token_type: "unknownType".to_string(),
            modifiers: vec![],
        },
        SemanticToken {
            line: 0,
            character: 4,
            length: 2,
            token_type: "keyword".to_string(),
            modifiers: vec![],
        },
    ];
    let data = encode_semantic_tokens(&tokens);

    // unknownType should be skipped, leaving only one 5-tuple
    assert_eq!(data.len(), 5);
}

#[test]
fn test_encode_semantic_tokens_modifier_bitmask() {
    // "readonly" is bit 2 (index 2), value = 4
    let tokens = vec![SemanticToken {
        line: 0,
        character: 0,
        length: 3,
        token_type: "variable".to_string(),
        modifiers: vec!["readonly".to_string()],
    }];
    let data = encode_semantic_tokens(&tokens);

    assert_eq!(data[4], 4); // readonly = bit 2 = value 4
}

// ============================================================================
// LSP Error Codes Tests
// ============================================================================

#[test]
fn test_lsp_error_codes() {
    assert_eq!(SERVER_NOT_INITIALIZED, -32002);
    assert_eq!(UNKNOWN_ERROR_CODE, -32001);
    assert_eq!(REQUEST_FAILED, -32803);
    assert_eq!(SERVER_CANCELLED, -32802);
    assert_eq!(CONTENT_MODIFIED, -32801);
    assert_eq!(REQUEST_CANCELLED, -32800);
}

// ============================================================================
// JSON-RPC Round-Trip Handler Tests
// ============================================================================
//
// These tests feed JSON-RPC messages through the full LspServer pipeline
// using in-memory buffers.

/// Helper: build a Content-Length-framed message from a JSON value.
fn frame(value: &Value) -> Vec<u8> {
    let payload = serde_json::to_vec(value).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    let mut result = header.into_bytes();
    result.extend_from_slice(&payload);
    result
}

/// Helper: parse all framed messages from a byte slice.
fn parse_all_messages(bytes: &[u8]) -> Vec<Message> {
    let mut result = Vec::new();
    let mut rest = bytes;

    loop {
        if rest.is_empty() {
            break;
        }
        // Find \r\n\r\n
        let sep_pos = rest
            .windows(4)
            .position(|w| w == b"\r\n\r\n");
        match sep_pos {
            None => break,
            Some(pos) => {
                let header = std::str::from_utf8(&rest[..pos]).unwrap();
                let cl_line = header
                    .lines()
                    .find(|l| l.starts_with("Content-Length:"))
                    .unwrap();
                let n: usize = cl_line
                    .trim_start_matches("Content-Length:")
                    .trim()
                    .parse()
                    .unwrap();

                let payload = &rest[pos + 4..pos + 4 + n];
                result.push(parse_message(payload).unwrap());
                rest = &rest[pos + 4 + n..];
            }
        }
    }

    result
}

/// Helper: run the server with given input bytes and return output bytes.
fn run_server(bridge: Box<dyn LanguageBridge>, input: Vec<u8>) -> Vec<u8> {
    let reader = BufReader::new(Cursor::new(input));
    let writer = Cursor::new(Vec::new());
    let mut server = LspServer::new(bridge, reader, writer);
    server.serve();
    // Extract the written bytes from the writer.
    server.writer.into_inner().into_inner()
}

/// Extracts the response with a given id from a list of messages.
fn find_response(messages: &[Message], id: i64) -> Option<&coding_adventures_json_rpc::message::Response> {
    messages.iter().find_map(|m| match m {
        Message::Response(r) if r.id == json!(id) => Some(r),
        _ => None,
    })
}

/// Extracts a notification with a given method from a list of messages.
fn find_notification<'a>(messages: &'a [Message], method: &str) -> Option<&'a coding_adventures_json_rpc::Notification> {
    messages.iter().find_map(|m| match m {
        Message::Notification(n) if n.method == method => Some(n),
        _ => None,
    })
}

// -- Initialize --

#[test]
fn test_handler_initialize() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1234, "capabilities": {}}
    })));

    let output = run_server(Box::new(MockBridge { hover_result: None }), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 1).unwrap();
    let result = resp.result.as_ref().unwrap();
    let caps = &result["capabilities"];

    assert_eq!(caps["textDocumentSync"], 2);
    assert_eq!(caps["hoverProvider"], true);
    assert_eq!(caps["documentSymbolProvider"], true);

    let server_info = &result["serverInfo"];
    assert_eq!(server_info["name"], "ls00-generic-lsp-server");
}

// -- Shutdown --

#[test]
fn test_handler_shutdown() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "shutdown"
    })));

    let output = run_server(Box::new(MinimalBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    assert!(resp.result.as_ref().unwrap().is_null());
}

// -- DidOpen publishes diagnostics --

#[test]
fn test_handler_did_open_publishes_diagnostics() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello ERROR world"
        }}
    })));

    let output = run_server(Box::new(MockBridge { hover_result: None }), input);
    let messages = parse_all_messages(&output);

    let notif = find_notification(&messages, "textDocument/publishDiagnostics").unwrap();
    let params = notif.params.as_ref().unwrap();
    assert_eq!(params["uri"], "file:///test.txt");
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(!diags.is_empty());
}

#[test]
fn test_handler_did_open_clean_file() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///clean.txt", "languageId": "test",
            "version": 1, "text": "hello world"
        }}
    })));

    let output = run_server(Box::new(MockBridge { hover_result: None }), input);
    let messages = parse_all_messages(&output);

    let notif = find_notification(&messages, "textDocument/publishDiagnostics").unwrap();
    let params = notif.params.as_ref().unwrap();
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(diags.is_empty());
}

// -- Hover --

#[test]
fn test_handler_hover() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.go", "languageId": "go",
            "version": 1, "text": "func main() {}"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///test.go"},
            "position": {"line": 0, "character": 5}
        }
    })));

    let bridge = MockBridge {
        hover_result: Some(HoverResult {
            contents: "**main** function".to_string(),
            range: Some(Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 4 },
            }),
        }),
    };
    let output = run_server(Box::new(bridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    assert_eq!(result["contents"]["kind"], "markdown");
    assert_eq!(result["contents"]["value"], "**main** function");
}

#[test]
fn test_handler_hover_no_bridge() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": "file:///test.txt"},
            "position": {"line": 0, "character": 0}
        }
    })));

    let output = run_server(Box::new(MinimalBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    assert!(result.is_null());
}

// -- Document Symbol --

#[test]
fn test_handler_document_symbol() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.go", "languageId": "go",
            "version": 1, "text": "func main() { var x = 1 }"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/documentSymbol",
        "params": {"textDocument": {"uri": "file:///test.go"}}
    })));

    let output = run_server(Box::new(MockBridge { hover_result: None }), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["name"], "main");
}

// -- Semantic Tokens Full --

#[test]
fn test_handler_semantic_tokens_full() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello world"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/semanticTokens/full",
        "params": {"textDocument": {"uri": "file:///test.txt"}}
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    assert!(result["data"].is_array());
}

// -- Definition --

#[test]
fn test_handler_definition() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello world"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": {"uri": "file:///test.txt"},
            "position": {"line": 0, "character": 0}
        }
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    assert_eq!(result["uri"], "file:///test.txt");
}

// -- References --

#[test]
fn test_handler_references() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/references",
        "params": {
            "textDocument": {"uri": "file:///test.txt"},
            "position": {"line": 0, "character": 0},
            "context": {"includeDeclaration": true}
        }
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty());
}

// -- Completion --

#[test]
fn test_handler_completion() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "foo"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": "file:///test.txt"},
            "position": {"line": 0, "character": 3}
        }
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    let items = result["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["label"], "foo");
}

// -- Rename --

#[test]
fn test_handler_rename() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/rename",
        "params": {
            "textDocument": {"uri": "file:///test.txt"},
            "position": {"line": 0, "character": 0},
            "newName": "world"
        }
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    assert!(result["changes"].is_object());
}

// -- Folding Range --

#[test]
fn test_handler_folding_range() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/foldingRange",
        "params": {"textDocument": {"uri": "file:///test.txt"}}
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["startLine"], 0);
    assert_eq!(arr[0]["endLine"], 5);
}

// -- Signature Help --

#[test]
fn test_handler_signature_help() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "foo(a, b)"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/signatureHelp",
        "params": {
            "textDocument": {"uri": "file:///test.txt"},
            "position": {"line": 0, "character": 4}
        }
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    let sigs = result["signatures"].as_array().unwrap();
    assert!(!sigs.is_empty());
    assert_eq!(sigs[0]["label"], "foo(a int, b string)");
}

// -- Formatting --

#[test]
fn test_handler_formatting() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"processId": 1, "capabilities": {}}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {
            "uri": "file:///test.txt", "languageId": "test",
            "version": 1, "text": "hello world"
        }}
    })));
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/formatting",
        "params": {"textDocument": {"uri": "file:///test.txt"}, "options": {}}
    })));

    let output = run_server(Box::new(FullMockBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 2).unwrap();
    let result = resp.result.as_ref().unwrap();
    let edits = result.as_array().unwrap();
    assert!(!edits.is_empty());
}

// -- Unknown method --

#[test]
fn test_handler_unknown_method() {
    let mut input = Vec::new();
    input.extend_from_slice(&frame(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "unknownMethod",
        "params": {}
    })));

    let output = run_server(Box::new(MinimalBridge), input);
    let messages = parse_all_messages(&output);

    let resp = find_response(&messages, 1).unwrap();
    assert!(resp.error.is_some());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        coding_adventures_json_rpc::errors::METHOD_NOT_FOUND
    );
}

// -- Server constructor --

#[test]
fn test_new_lsp_server_creates_server() {
    let reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
    let writer = Cursor::new(Vec::<u8>::new());
    // Just verify it constructs without panicking.
    let _server = LspServer::new(Box::new(MinimalBridge), reader, writer);
}

// -- Document Symbol with children --

#[test]
fn test_document_symbol_with_children() {
    let bridge = MockBridge { hover_result: None };
    let mut cache = ParseCache::new();
    let mut dm = DocumentManager::new();

    dm.open("file:///a.go", "func main() {}", 1);
    let doc = dm.get("file:///a.go").unwrap();
    let result = cache.get_or_parse("file:///a.go", doc.version, &doc.text, &bridge);
    assert!(result.ast.is_some());

    let ast = result.ast.as_ref().unwrap();
    let syms = bridge.document_symbols(ast.as_ref()).unwrap().unwrap();

    assert_eq!(syms.len(), 1);
    assert_eq!(syms[0].name, "main");
    assert_eq!(syms[0].kind, SymbolKind::Function);
    assert_eq!(syms[0].children.len(), 1);
    assert_eq!(syms[0].children[0].name, "x");
}
