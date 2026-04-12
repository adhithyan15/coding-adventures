//! Capability advertisement and semantic token encoding.
//!
//! # What Are Capabilities?
//!
//! During the LSP initialize handshake, the server sends back a "capabilities"
//! object telling the editor which LSP features it supports. The editor uses
//! this to decide which requests to send. If a capability is absent, the editor
//! won't send the corresponding requests -- so no "Go to Definition" button
//! appears unless `definitionProvider` is `true`.
//!
//! # Semantic Token Legend
//!
//! Semantic tokens use a compact binary encoding. Instead of sending
//! `{"type":"keyword"}` per token, LSP sends an integer index into a legend.
//! The legend must be declared in the capabilities so the editor knows what
//! each index means.

use crate::language_bridge::LanguageBridge;
use crate::types::SemanticToken;
use serde::Serialize;
use serde_json::{json, Value};

/// Inspect the bridge's capability flags and return the LSP capabilities
/// object to include in the `initialize` response.
///
/// Uses the `supports_*` methods on the bridge trait to determine which
/// optional capabilities to advertise.
pub fn build_capabilities(bridge: &dyn LanguageBridge) -> Value {
    // textDocumentSync=2 means "incremental": the editor sends only changed
    // ranges, not the full file, on every keystroke.
    let mut caps = serde_json::Map::new();
    caps.insert("textDocumentSync".to_string(), json!(2));

    if bridge.supports_hover() {
        caps.insert("hoverProvider".to_string(), json!(true));
    }

    if bridge.supports_definition() {
        caps.insert("definitionProvider".to_string(), json!(true));
    }

    if bridge.supports_references() {
        caps.insert("referencesProvider".to_string(), json!(true));
    }

    if bridge.supports_completion() {
        caps.insert(
            "completionProvider".to_string(),
            json!({"triggerCharacters": [" ", "."]}),
        );
    }

    if bridge.supports_rename() {
        caps.insert("renameProvider".to_string(), json!(true));
    }

    if bridge.supports_document_symbols() {
        caps.insert("documentSymbolProvider".to_string(), json!(true));
    }

    if bridge.supports_folding_ranges() {
        caps.insert("foldingRangeProvider".to_string(), json!(true));
    }

    if bridge.supports_signature_help() {
        caps.insert(
            "signatureHelpProvider".to_string(),
            json!({"triggerCharacters": ["(", ","]}),
        );
    }

    if bridge.supports_format() {
        caps.insert("documentFormattingProvider".to_string(), json!(true));
    }

    if bridge.supports_semantic_tokens() {
        caps.insert(
            "semanticTokensProvider".to_string(),
            json!({
                "legend": semantic_token_legend(),
                "full": true
            }),
        );
    }

    Value::Object(caps)
}

// ---------------------------------------------------------------------------
// SemanticTokenLegendData
// ---------------------------------------------------------------------------

/// Holds the legend arrays for semantic tokens.
/// The editor uses these to decode the compact integer encoding.
#[derive(Debug, Clone, Serialize)]
pub struct SemanticTokenLegendData {
    #[serde(rename = "tokenTypes")]
    pub token_types: Vec<String>,
    #[serde(rename = "tokenModifiers")]
    pub token_modifiers: Vec<String>,
}

/// Return the full legend for all supported semantic token types and modifiers.
///
/// # Why a Fixed Legend?
///
/// The legend is sent once in the capabilities response. Afterwards, each
/// semantic token is encoded as an integer index into this legend rather than
/// a string. This makes the per-token encoding much smaller.
///
/// The ordering matters: index 0 corresponds to `"namespace"`, index 1 to
/// `"type"`, etc. These match the standard LSP token types.
pub fn semantic_token_legend() -> SemanticTokenLegendData {
    SemanticTokenLegendData {
        // Standard LSP token types (in the order VS Code expects them).
        token_types: vec![
            "namespace".into(),     // 0
            "type".into(),          // 1
            "class".into(),         // 2
            "enum".into(),          // 3
            "interface".into(),     // 4
            "struct".into(),        // 5
            "typeParameter".into(), // 6
            "parameter".into(),     // 7
            "variable".into(),      // 8
            "property".into(),      // 9
            "enumMember".into(),    // 10
            "event".into(),         // 11
            "function".into(),      // 12
            "method".into(),        // 13
            "macro".into(),         // 14
            "keyword".into(),       // 15
            "modifier".into(),      // 16
            "comment".into(),       // 17
            "string".into(),        // 18
            "number".into(),        // 19
            "regexp".into(),        // 20
            "operator".into(),      // 21
            "decorator".into(),     // 22
        ],
        // Standard LSP token modifiers (bitmask flags).
        token_modifiers: vec![
            "declaration".into(),    // bit 0
            "definition".into(),     // bit 1
            "readonly".into(),       // bit 2
            "static".into(),         // bit 3
            "deprecated".into(),     // bit 4
            "abstract".into(),       // bit 5
            "async".into(),          // bit 6
            "modification".into(),   // bit 7
            "documentation".into(),  // bit 8
            "defaultLibrary".into(), // bit 9
        ],
    }
}

/// Return the integer index for a semantic token type string.
/// Returns `None` if the type is not in the legend (the caller should skip
/// such tokens).
pub fn token_type_index(token_type: &str) -> Option<usize> {
    let legend = semantic_token_legend();
    legend.token_types.iter().position(|t| t == token_type)
}

/// Return the bitmask for a list of modifier strings.
///
/// The LSP semantic tokens encoding represents modifiers as a bitmask:
/// - `"declaration"` -> bit 0 -> value 1
/// - `"definition"` -> bit 1 -> value 2
/// - both -> value 3 (bitwise OR)
///
/// Unknown modifiers are silently ignored.
pub fn token_modifier_mask(modifiers: &[String]) -> i32 {
    let legend = semantic_token_legend();
    let mut mask: i32 = 0;
    for modifier in modifiers {
        if let Some(idx) = legend.token_modifiers.iter().position(|m| m == modifier) {
            mask |= 1 << idx;
        }
    }
    mask
}

/// Convert a slice of `SemanticToken` values to the LSP compact integer
/// encoding.
///
/// # The LSP Semantic Token Encoding
///
/// LSP encodes semantic tokens as a flat array of integers, grouped in
/// 5-tuples:
///
/// ```text
/// [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
/// ```
///
/// Where "delta" means: the difference from the PREVIOUS token's position.
/// This delta encoding makes most values small (often 0 or 1).
///
/// When `deltaLine > 0`, `deltaStartChar` is relative to column 0 of the new
/// line (i.e., absolute for that line). When `deltaLine == 0`, `deltaStartChar`
/// is relative to the previous token's start character.
pub fn encode_semantic_tokens(tokens: &[SemanticToken]) -> Vec<i32> {
    if tokens.is_empty() {
        return Vec::new();
    }

    // Sort by (line, character) ascending. The delta encoding requires tokens
    // to be in document order.
    let mut sorted = tokens.to_vec();
    sorted.sort_by(|a, b| {
        a.line.cmp(&b.line).then(a.character.cmp(&b.character))
    });

    let mut data = Vec::with_capacity(sorted.len() * 5);
    let mut prev_line: i32 = 0;
    let mut prev_char: i32 = 0;

    for tok in &sorted {
        let type_idx = match token_type_index(&tok.token_type) {
            Some(idx) => idx as i32,
            None => continue, // unknown token type -- skip
        };

        let delta_line = tok.line - prev_line;
        let delta_char = if delta_line == 0 {
            tok.character - prev_char
        } else {
            tok.character // absolute on new line
        };

        let mod_mask = token_modifier_mask(&tok.modifiers);

        data.push(delta_line);
        data.push(delta_char);
        data.push(tok.length);
        data.push(type_idx);
        data.push(mod_mask);

        prev_line = tok.line;
        prev_char = tok.character;
    }

    data
}
