/**
 * @coding-adventures/ls00
 *
 * Generic Language Server Protocol (LSP) framework.
 *
 * This package implements the generic half of a language server. Language-specific
 * "bridges" plug into this framework by implementing the LanguageBridge interface
 * (and any optional provider interfaces). The framework handles all LSP protocol
 * boilerplate: lifecycle, document sync, capability negotiation, and JSON-RPC transport.
 *
 * Architecture
 * ------------
 *
 *   Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
 *
 * Quick start
 * -----------
 *
 *     import { LspServer } from "@coding-adventures/ls00";
 *     import type { LanguageBridge } from "@coding-adventures/ls00";
 *
 *     const bridge: LanguageBridge = {
 *       tokenize(source) { return []; },
 *       parse(source) { return [source, []]; },
 *     };
 *
 *     const server = new LspServer(bridge, process.stdin, process.stdout);
 *     await server.serve();
 *
 * Public exports
 * --------------
 *
 *   LspServer           -- the main server class
 *   LanguageBridge       -- required interface for language implementations
 *   HoverProvider, etc.  -- optional provider interfaces
 *   isHoverProvider, etc -- type guard functions for capability detection
 *   buildCapabilities    -- build capabilities object from a bridge
 *   encodeSemanticTokens -- encode semantic tokens to LSP compact format
 *   semanticTokenLegend  -- the semantic token legend
 *   DocumentManager      -- tracks open file contents
 *   ParseCache           -- caches parse results by (uri, version)
 *   LspErrorCodes        -- LSP-specific error code constants
 *   All LSP types        -- Position, Range, Diagnostic, Token, etc.
 *
 * @module
 */

// Types
export {
  type Position,
  type Range,
  type Location,
  DiagnosticSeverity,
  type Diagnostic,
  type Token,
  type TextEdit,
  type WorkspaceEdit,
  type HoverResult,
  CompletionItemKind,
  type CompletionItem,
  type SemanticToken,
  SymbolKind,
  type DocumentSymbol,
  type FoldingRange,
  type ParameterInformation,
  type SignatureInformation,
  type SignatureHelpResult,
  type TextChange,
} from "./types.js";

// Language bridge interfaces and type guards
export {
  type LanguageBridge,
  type HoverProvider,
  type DefinitionProvider,
  type ReferencesProvider,
  type CompletionProvider,
  type RenameProvider,
  type SemanticTokensProvider,
  type DocumentSymbolsProvider,
  type FoldingRangesProvider,
  type SignatureHelpProvider,
  type FormatProvider,
  isHoverProvider,
  isDefinitionProvider,
  isReferencesProvider,
  isCompletionProvider,
  isRenameProvider,
  isSemanticTokensProvider,
  isDocumentSymbolsProvider,
  isFoldingRangesProvider,
  isSignatureHelpProvider,
  isFormatProvider,
} from "./language-bridge.js";

// Document manager
export {
  DocumentManager,
  type Document,
  convertPositionToStringIndex,
  convertUTF16OffsetToByteOffset,
} from "./document-manager.js";

// Parse cache
export { ParseCache, type ParseResult } from "./parse-cache.js";

// Capabilities
export {
  buildCapabilities,
  semanticTokenLegend,
  type SemanticTokenLegendData,
  encodeSemanticTokens,
  tokenTypeIndex,
  tokenModifierMask,
} from "./capabilities.js";

// LSP error codes
export { LspErrorCodes } from "./lsp-errors.js";

// Server
export { LspServer } from "./server.js";
