/**
 * capabilities.test.ts -- Capabilities and type guard tests
 *
 * Tests cover:
 *   - MinimalBridge: only textDocumentSync, no optional capabilities
 *   - MockBridge with hover + documentSymbols: those capabilities present
 *   - FullMockBridge with all providers: all capabilities present
 *   - SemanticTokenLegend consistency
 *   - LSP error code constants
 */

import { describe, it, expect } from "vitest";
import {
  buildCapabilities,
  semanticTokenLegend,
} from "../src/capabilities.js";
import { LspErrorCodes } from "../src/lsp-errors.js";
import type { LanguageBridge, HoverProvider, DocumentSymbolsProvider } from "../src/language-bridge.js";
import type {
  Token,
  Diagnostic,
  Position,
  HoverResult,
  DocumentSymbol,
  Location,
  CompletionItem,
  WorkspaceEdit,
  SemanticToken,
  FoldingRange,
  SignatureHelpResult,
  TextEdit,
} from "../src/types.js";
import { SymbolKind, CompletionItemKind } from "../src/types.js";

// ---------------------------------------------------------------------------
// Test bridges
// ---------------------------------------------------------------------------

/** MinimalBridge implements ONLY the required LanguageBridge interface. */
class MinimalBridge implements LanguageBridge {
  tokenize(_source: string): Token[] { return []; }
  parse(source: string): [unknown, Diagnostic[]] { return [source, []]; }
}

/** MockBridge implements LanguageBridge + HoverProvider + DocumentSymbolsProvider. */
class MockBridge implements LanguageBridge, HoverProvider, DocumentSymbolsProvider {
  tokenize(source: string): Token[] {
    const tokens: Token[] = [];
    let col = 1;
    for (const word of source.split(/\s+/).filter(Boolean)) {
      tokens.push({ type: "WORD", value: word, line: 1, column: col });
      col += word.length + 1;
    }
    return tokens;
  }

  parse(source: string): [unknown, Diagnostic[]] {
    const diags: Diagnostic[] = [];
    if (source.includes("ERROR")) {
      diags.push({
        range: { start: { line: 0, character: 0 }, end: { line: 0, character: 5 } },
        severity: 1,
        message: "syntax error",
      });
    }
    return [source, diags];
  }

  hover(_ast: unknown, _pos: Position): HoverResult | null {
    return { contents: "**test**" };
  }

  documentSymbols(_ast: unknown): DocumentSymbol[] {
    return [{
      name: "main",
      kind: SymbolKind.Function,
      range: { start: { line: 0, character: 0 }, end: { line: 10, character: 1 } },
      selectionRange: { start: { line: 0, character: 9 }, end: { line: 0, character: 13 } },
      children: [{
        name: "x",
        kind: SymbolKind.Variable,
        range: { start: { line: 1, character: 4 }, end: { line: 1, character: 12 } },
        selectionRange: { start: { line: 1, character: 8 }, end: { line: 1, character: 9 } },
      }],
    }];
  }
}

/**
 * FullMockBridge implements every optional provider interface.
 * Used to test that all capabilities are advertised.
 */
class FullMockBridge extends MockBridge {
  semanticTokens(_source: string, tokens: Token[]): SemanticToken[] {
    return tokens.map((tok) => ({
      line: tok.line - 1,
      character: tok.column - 1,
      length: tok.value.length,
      tokenType: "variable",
      modifiers: [],
    }));
  }

  definition(_ast: unknown, pos: Position, uri: string): Location | null {
    return { uri, range: { start: pos, end: pos } };
  }

  references(_ast: unknown, pos: Position, uri: string, _includeDecl: boolean): Location[] {
    return [{ uri, range: { start: pos, end: pos } }];
  }

  completion(_ast: unknown, _pos: Position): CompletionItem[] {
    return [{ label: "foo", kind: CompletionItemKind.Function, detail: "() void" }];
  }

  rename(_ast: unknown, pos: Position, newName: string): WorkspaceEdit | null {
    return {
      changes: {
        "file:///test.txt": [
          { range: { start: pos, end: pos }, newText: newName },
        ],
      },
    };
  }

  foldingRanges(_ast: unknown): FoldingRange[] {
    return [{ startLine: 0, endLine: 5, kind: "region" }];
  }

  signatureHelp(_ast: unknown, _pos: Position): SignatureHelpResult | null {
    return {
      signatures: [{
        label: "foo(a int, b string)",
        parameters: [{ label: "a int" }, { label: "b string" }],
      }],
      activeSignature: 0,
      activeParameter: 0,
    };
  }

  format(_source: string): TextEdit[] {
    return [{
      range: { start: { line: 0, character: 0 }, end: { line: 999, character: 0 } },
      newText: _source,
    }];
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("buildCapabilities", () => {
  it("minimal bridge: only textDocumentSync", () => {
    const caps = buildCapabilities(new MinimalBridge());

    expect(caps.textDocumentSync).toBe(2);

    const optionalCaps = [
      "hoverProvider", "definitionProvider", "referencesProvider",
      "completionProvider", "renameProvider", "documentSymbolProvider",
      "foldingRangeProvider", "signatureHelpProvider",
      "documentFormattingProvider", "semanticTokensProvider",
    ];
    for (const cap of optionalCaps) {
      expect(caps[cap]).toBeUndefined();
    }
  });

  it("mock bridge: hover + documentSymbols advertised", () => {
    const caps = buildCapabilities(new MockBridge());

    expect(caps.hoverProvider).toBe(true);
    expect(caps.documentSymbolProvider).toBe(true);

    // Should NOT have capabilities the MockBridge doesn't implement
    expect(caps.definitionProvider).toBeUndefined();
    expect(caps.semanticTokensProvider).toBeUndefined();
  });

  it("full bridge: all capabilities advertised", () => {
    const caps = buildCapabilities(new FullMockBridge());

    const expected = [
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
    for (const cap of expected) {
      expect(caps[cap]).toBeDefined();
    }
  });

  it("semanticTokensProvider includes legend and full flag", () => {
    const caps = buildCapabilities(new FullMockBridge());
    const stp = caps.semanticTokensProvider as Record<string, unknown>;
    expect(stp.full).toBe(true);
    expect(stp.legend).toBeDefined();
  });

  it("completionProvider includes triggerCharacters", () => {
    const caps = buildCapabilities(new FullMockBridge());
    const cp = caps.completionProvider as Record<string, unknown>;
    expect(cp.triggerCharacters).toEqual([" ", "."]);
  });

  it("signatureHelpProvider includes triggerCharacters", () => {
    const caps = buildCapabilities(new FullMockBridge());
    const shp = caps.signatureHelpProvider as Record<string, unknown>;
    expect(shp.triggerCharacters).toEqual(["(", ","]);
  });
});

describe("semanticTokenLegend", () => {
  it("has non-empty tokenTypes", () => {
    const legend = semanticTokenLegend();
    expect(legend.tokenTypes.length).toBeGreaterThan(0);
  });

  it("has non-empty tokenModifiers", () => {
    const legend = semanticTokenLegend();
    expect(legend.tokenModifiers.length).toBeGreaterThan(0);
  });

  it("contains required types", () => {
    const legend = semanticTokenLegend();
    const required = ["keyword", "string", "number", "variable", "function"];
    for (const rt of required) {
      expect(legend.tokenTypes).toContain(rt);
    }
  });

  it("contains declaration modifier", () => {
    const legend = semanticTokenLegend();
    expect(legend.tokenModifiers).toContain("declaration");
  });
});

describe("LspErrorCodes", () => {
  it("ServerNotInitialized is -32002", () => {
    expect(LspErrorCodes.ServerNotInitialized).toBe(-32002);
  });

  it("UnknownErrorCode is -32001", () => {
    expect(LspErrorCodes.UnknownErrorCode).toBe(-32001);
  });

  it("RequestFailed is -32803", () => {
    expect(LspErrorCodes.RequestFailed).toBe(-32803);
  });

  it("ServerCancelled is -32802", () => {
    expect(LspErrorCodes.ServerCancelled).toBe(-32802);
  });

  it("ContentModified is -32801", () => {
    expect(LspErrorCodes.ContentModified).toBe(-32801);
  });

  it("RequestCancelled is -32800", () => {
    expect(LspErrorCodes.RequestCancelled).toBe(-32800);
  });
});
