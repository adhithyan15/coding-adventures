/**
 * capabilities.ts -- BuildCapabilities, SemanticTokenLegend, and encodeSemanticTokens
 *
 * # What Are Capabilities?
 *
 * During the LSP initialize handshake, the server sends back a "capabilities"
 * object telling the editor which LSP features it supports. The editor uses this
 * to decide which requests to send. If a capability is absent, the editor won't
 * send the corresponding requests -- so no "Go to Definition" button appears
 * unless definitionProvider is true.
 *
 * Building capabilities dynamically (based on the bridge's interface
 * implementations) means the server is always honest about what it can do.
 *
 * # Semantic Token Legend
 *
 * Semantic tokens use a compact binary encoding. Instead of sending
 * {"type":"keyword"} per token, LSP sends an integer index into a legend.
 * The legend must be declared in the capabilities so the editor knows what
 * each index means.
 *
 * @module
 */

import type { LanguageBridge } from "./language-bridge.js";
import type { SemanticToken } from "./types.js";
import {
  isHoverProvider,
  isDefinitionProvider,
  isReferencesProvider,
  isCompletionProvider,
  isRenameProvider,
  isDocumentSymbolsProvider,
  isFoldingRangesProvider,
  isSignatureHelpProvider,
  isFormatProvider,
  isSemanticTokensProvider,
} from "./language-bridge.js";

// ---------------------------------------------------------------------------
// BuildCapabilities
// ---------------------------------------------------------------------------

/**
 * Inspect the bridge at runtime and return the LSP capabilities object
 * to include in the initialize response.
 *
 * Uses TypeScript type guard functions to check which optional provider
 * interfaces the bridge implements. Only advertises capabilities for
 * features the bridge actually supports.
 */
export function buildCapabilities(bridge: LanguageBridge): Record<string, unknown> {
  // textDocumentSync=2 means "incremental": the editor sends only changed
  // ranges, not the full file, on every keystroke.
  const caps: Record<string, unknown> = {
    textDocumentSync: 2,
  };

  if (isHoverProvider(bridge)) {
    caps.hoverProvider = true;
  }

  if (isDefinitionProvider(bridge)) {
    caps.definitionProvider = true;
  }

  if (isReferencesProvider(bridge)) {
    caps.referencesProvider = true;
  }

  if (isCompletionProvider(bridge)) {
    // completionProvider is an object, not a boolean, because it can include
    // triggerCharacters: which chars auto-trigger completions.
    caps.completionProvider = {
      triggerCharacters: [" ", "."],
    };
  }

  if (isRenameProvider(bridge)) {
    caps.renameProvider = true;
  }

  if (isDocumentSymbolsProvider(bridge)) {
    caps.documentSymbolProvider = true;
  }

  if (isFoldingRangesProvider(bridge)) {
    caps.foldingRangeProvider = true;
  }

  if (isSignatureHelpProvider(bridge)) {
    caps.signatureHelpProvider = {
      triggerCharacters: ["(", ","],
    };
  }

  if (isFormatProvider(bridge)) {
    caps.documentFormattingProvider = true;
  }

  if (isSemanticTokensProvider(bridge)) {
    caps.semanticTokensProvider = {
      legend: semanticTokenLegend(),
      full: true,
    };
  }

  return caps;
}

// ---------------------------------------------------------------------------
// Semantic Token Legend
// ---------------------------------------------------------------------------

/**
 * SemanticTokenLegendData holds the legend arrays for semantic tokens.
 * The editor uses these to decode the compact integer encoding.
 */
export interface SemanticTokenLegendData {
  tokenTypes: string[];
  tokenModifiers: string[];
}

/**
 * Return the full legend for all supported semantic token types and modifiers.
 *
 * # Why a Fixed Legend?
 *
 * The legend is sent once in the capabilities response. Afterwards, each
 * semantic token is encoded as an integer index into this legend rather than
 * a string. This makes the per-token encoding much smaller.
 *
 * The ordering matters: index 0 in tokenTypes corresponds to "namespace",
 * index 1 to "type", etc. These match the standard LSP token types.
 */
export function semanticTokenLegend(): SemanticTokenLegendData {
  return {
    // Standard LSP token types (in the order VS Code expects them).
    // Source: https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide
    tokenTypes: [
      "namespace",     // 0
      "type",          // 1
      "class",         // 2
      "enum",          // 3
      "interface",     // 4
      "struct",        // 5
      "typeParameter", // 6
      "parameter",     // 7
      "variable",      // 8
      "property",      // 9
      "enumMember",    // 10
      "event",         // 11
      "function",      // 12
      "method",        // 13
      "macro",         // 14
      "keyword",       // 15
      "modifier",      // 16
      "comment",       // 17
      "string",        // 18
      "number",        // 19
      "regexp",        // 20
      "operator",      // 21
      "decorator",     // 22
    ],
    // Standard LSP token modifiers (bitmask flags).
    // tokenModifier[0] = "declaration" -> bit 0 (value 1)
    // tokenModifier[1] = "definition"  -> bit 1 (value 2)
    // etc.
    tokenModifiers: [
      "declaration",   // bit 0
      "definition",    // bit 1
      "readonly",      // bit 2
      "static",        // bit 3
      "deprecated",    // bit 4
      "abstract",      // bit 5
      "async",         // bit 6
      "modification",  // bit 7
      "documentation", // bit 8
      "defaultLibrary", // bit 9
    ],
  };
}

// ---------------------------------------------------------------------------
// Encoding helpers
// ---------------------------------------------------------------------------

/**
 * Return the integer index for a semantic token type string.
 * Returns -1 if the type is not in the legend (the caller should skip such tokens).
 */
export function tokenTypeIndex(tokenType: string): number {
  const legend = semanticTokenLegend();
  return legend.tokenTypes.indexOf(tokenType);
}

/**
 * Return the bitmask for a list of modifier strings.
 *
 * The LSP semantic tokens encoding represents modifiers as a bitmask:
 *   - "declaration" -> bit 0 -> value 1
 *   - "definition"  -> bit 1 -> value 2
 *   - both          -> value 3 (bitwise OR)
 *
 * Unknown modifiers are silently ignored.
 */
export function tokenModifierMask(modifiers: string[]): number {
  const legend = semanticTokenLegend();
  let mask = 0;
  for (const mod of modifiers) {
    const idx = legend.tokenModifiers.indexOf(mod);
    if (idx !== -1) {
      mask |= (1 << idx);
    }
  }
  return mask;
}

/**
 * Convert an array of SemanticToken values to the LSP compact integer encoding.
 *
 * # The LSP Semantic Token Encoding
 *
 * LSP encodes semantic tokens as a flat array of integers, grouped in 5-tuples:
 *
 *   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
 *
 * Where "delta" means: the difference from the PREVIOUS token's position.
 * This delta encoding makes most values small (often 0 or 1), which compresses
 * well and is efficient to parse.
 *
 * Example: three tokens on different lines:
 *
 *   Token A: line=0, char=0, len=3, type="keyword",  modifiers=[]
 *   Token B: line=0, char=4, len=5, type="function", modifiers=["declaration"]
 *   Token C: line=1, char=0, len=8, type="variable", modifiers=[]
 *
 * Encoded as:
 *   [0, 0, 3, 15, 0,   // A: deltaLine=0, deltaChar=0 (first token)
 *    0, 4, 5, 12, 1,   // B: deltaLine=0, deltaChar=4 (same line, 4 chars later)
 *    1, 0, 8,  8, 0]   // C: deltaLine=1, deltaChar=0 (next line, reset to abs)
 *
 * Note: when deltaLine > 0, deltaStartChar is relative to column 0 of the new line
 * (i.e., absolute for that line). When deltaLine == 0, deltaStartChar is relative
 * to the previous token's start character.
 */
export function encodeSemanticTokens(tokens: SemanticToken[]): number[] {
  if (tokens.length === 0) {
    return [];
  }

  // Sort by (line, character) ascending. The delta encoding requires tokens
  // to be in document order -- otherwise the deltas would be negative.
  const sorted = [...tokens].sort((a, b) => {
    if (a.line !== b.line) return a.line - b.line;
    return a.character - b.character;
  });

  const data: number[] = [];
  let prevLine = 0;
  let prevChar = 0;

  for (const tok of sorted) {
    const typeIdx = tokenTypeIndex(tok.tokenType);
    if (typeIdx === -1) {
      // Unknown token type -- skip it. The client wouldn't know what to do
      // with an index outside the legend anyway.
      continue;
    }

    const deltaLine = tok.line - prevLine;
    let deltaChar: number;
    if (deltaLine === 0) {
      // Same line: character offset is relative to previous token.
      deltaChar = tok.character - prevChar;
    } else {
      // Different line: character offset is absolute (relative to line start).
      deltaChar = tok.character;
    }

    const modMask = tokenModifierMask(tok.modifiers ?? []);

    data.push(deltaLine, deltaChar, tok.length, typeIdx, modMask);

    prevLine = tok.line;
    prevChar = tok.character;
  }

  return data;
}
