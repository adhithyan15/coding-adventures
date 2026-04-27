/**
 * language-bridge.ts -- LanguageBridge and optional provider interfaces
 *
 * # Design Philosophy: Narrow Interfaces
 *
 * TypeScript's interface system naturally supports the narrow-interface pattern.
 * We define one required interface (LanguageBridge) plus many small optional
 * provider interfaces. At runtime, the server checks whether the bridge object
 * also satisfies a provider interface using type guard functions.
 *
 * Compare two approaches:
 *
 *   APPROACH A -- fat interface (one big interface, every method required):
 *
 *     interface LanguageBridge {
 *       tokenize(source: string): Token[];
 *       parse(source: string): [unknown, Diagnostic[]];
 *       hover(ast: unknown, pos: Position): HoverResult | null;
 *       definition(ast: unknown, pos: Position, uri: string): Location | null;
 *       // ... 8 more methods, all REQUIRED
 *     }
 *
 *     Problem: a language that only has a lexer and parser must implement ALL
 *     methods, even the symbol-table features it doesn't have yet.
 *
 *   APPROACH B -- narrow interfaces (what we use):
 *
 *     interface LanguageBridge {
 *       tokenize(source: string): Token[];
 *       parse(source: string): [unknown, Diagnostic[]];
 *     }
 *     interface HoverProvider { hover(...): HoverResult | null; }
 *     interface DefinitionProvider { definition(...): Location | null; }
 *
 *     At runtime, the server checks: does bridge satisfy HoverProvider?
 *     If yes -> advertise hoverProvider:true and call bridge.hover().
 *     If no  -> omit hover from capabilities. No stubs required.
 *
 * # Runtime Capability Detection
 *
 * Detection uses TypeScript type guard functions:
 *
 *     function isHoverProvider(b: LanguageBridge): b is LanguageBridge & HoverProvider {
 *       return 'hover' in b && typeof (b as any).hover === 'function';
 *     }
 *
 * The `is` return type annotation is a "type predicate" -- it tells TypeScript
 * that after calling `isHoverProvider(bridge)` and getting true, the variable
 * `bridge` can be used as `LanguageBridge & HoverProvider` in the true branch.
 *
 * @module
 */

import type {
  Token,
  Diagnostic,
  Position,
  Location,
  HoverResult,
  CompletionItem,
  WorkspaceEdit,
  SemanticToken,
  DocumentSymbol,
  FoldingRange,
  SignatureHelpResult,
  TextEdit,
} from "./types.js";

// ---------------------------------------------------------------------------
// Required interface -- every language bridge must implement this
// ---------------------------------------------------------------------------

/**
 * LanguageBridge is the required minimum interface every language must implement.
 *
 * `tokenize` and `parse` are the foundation for all other features:
 *   - `tokenize` drives semantic token highlighting (accurate syntax coloring)
 *   - `parse` drives diagnostics, folding, and document symbols
 *
 * All other features (hover, go-to-definition, etc.) are optional and declared
 * as separate interfaces below. The LspServer checks at runtime whether the
 * bridge also implements those interfaces.
 */
export interface LanguageBridge {
  /**
   * Lex the source string and return the token stream.
   *
   * The tokens are used for semantic highlighting. Each Token carries a `type`
   * string (e.g. "KEYWORD", "IDENTIFIER"), its `value`, and its 1-based `line`
   * and `column` position. The bridge is responsible for converting 1-based
   * positions to 0-based before building SemanticToken values.
   */
  tokenize(source: string): Token[];

  /**
   * Parse the source string and return:
   *   - ast:         the parsed abstract syntax tree (may be partial on error)
   *   - diagnostics: parse errors and warnings as LSP Diagnostic objects
   *
   * Even when there are syntax errors, `parse` should return a partial AST.
   * This allows hover, folding, and symbol features to continue working on
   * the valid portions of the file.
   *
   * The AST type is `unknown` because each language's parser returns its
   * own concrete AST type. The bridge is responsible for downcasting.
   */
  parse(source: string): [unknown, Diagnostic[]];
}

// ---------------------------------------------------------------------------
// Optional Provider Interfaces
// ---------------------------------------------------------------------------
//
// Each of the following interfaces represents one optional LSP feature.
// A bridge implements only the features its language supports.
// The server uses type guard functions to detect support at runtime.
//
// None of these interfaces extend LanguageBridge -- they are purely additive.
// A bridge object simply implements whichever methods it needs.

/** HoverProvider enables hover tooltips. */
export interface HoverProvider {
  hover(ast: unknown, pos: Position): HoverResult | null;
}

/** DefinitionProvider enables "Go to Definition" (F12 in VS Code). */
export interface DefinitionProvider {
  definition(ast: unknown, pos: Position, uri: string): Location | null;
}

/** ReferencesProvider enables "Find All References". */
export interface ReferencesProvider {
  references(ast: unknown, pos: Position, uri: string, includeDecl: boolean): Location[];
}

/** CompletionProvider enables autocomplete suggestions. */
export interface CompletionProvider {
  completion(ast: unknown, pos: Position): CompletionItem[];
}

/** RenameProvider enables symbol rename (F2 in VS Code). */
export interface RenameProvider {
  rename(ast: unknown, pos: Position, newName: string): WorkspaceEdit | null;
}

/** SemanticTokensProvider enables accurate syntax highlighting. */
export interface SemanticTokensProvider {
  semanticTokens(source: string, tokens: Token[]): SemanticToken[];
}

/** DocumentSymbolsProvider enables the document outline panel. */
export interface DocumentSymbolsProvider {
  documentSymbols(ast: unknown): DocumentSymbol[];
}

/** FoldingRangesProvider enables code folding (collapsible blocks). */
export interface FoldingRangesProvider {
  foldingRanges(ast: unknown): FoldingRange[];
}

/** SignatureHelpProvider enables function signature hints. */
export interface SignatureHelpProvider {
  signatureHelp(ast: unknown, pos: Position): SignatureHelpResult | null;
}

/** FormatProvider enables document formatting (Format on Save). */
export interface FormatProvider {
  format(source: string): TextEdit[];
}

// ---------------------------------------------------------------------------
// Type guard functions -- runtime capability detection
// ---------------------------------------------------------------------------
//
// Each type guard checks whether the bridge object has the relevant method.
// The `bridge is LanguageBridge & XxxProvider` return type is a TypeScript
// "type predicate" that narrows the type in conditional branches.

/** Check if the bridge implements hover tooltips. */
export function isHoverProvider(bridge: LanguageBridge): bridge is LanguageBridge & HoverProvider {
  return "hover" in bridge && typeof (bridge as Record<string, unknown>).hover === "function";
}

/** Check if the bridge implements go-to-definition. */
export function isDefinitionProvider(bridge: LanguageBridge): bridge is LanguageBridge & DefinitionProvider {
  return "definition" in bridge && typeof (bridge as Record<string, unknown>).definition === "function";
}

/** Check if the bridge implements find-all-references. */
export function isReferencesProvider(bridge: LanguageBridge): bridge is LanguageBridge & ReferencesProvider {
  return "references" in bridge && typeof (bridge as Record<string, unknown>).references === "function";
}

/** Check if the bridge implements autocomplete. */
export function isCompletionProvider(bridge: LanguageBridge): bridge is LanguageBridge & CompletionProvider {
  return "completion" in bridge && typeof (bridge as Record<string, unknown>).completion === "function";
}

/** Check if the bridge implements rename. */
export function isRenameProvider(bridge: LanguageBridge): bridge is LanguageBridge & RenameProvider {
  return "rename" in bridge && typeof (bridge as Record<string, unknown>).rename === "function";
}

/** Check if the bridge implements semantic tokens. */
export function isSemanticTokensProvider(bridge: LanguageBridge): bridge is LanguageBridge & SemanticTokensProvider {
  return "semanticTokens" in bridge && typeof (bridge as Record<string, unknown>).semanticTokens === "function";
}

/** Check if the bridge implements document symbols (outline). */
export function isDocumentSymbolsProvider(bridge: LanguageBridge): bridge is LanguageBridge & DocumentSymbolsProvider {
  return "documentSymbols" in bridge && typeof (bridge as Record<string, unknown>).documentSymbols === "function";
}

/** Check if the bridge implements code folding. */
export function isFoldingRangesProvider(bridge: LanguageBridge): bridge is LanguageBridge & FoldingRangesProvider {
  return "foldingRanges" in bridge && typeof (bridge as Record<string, unknown>).foldingRanges === "function";
}

/** Check if the bridge implements signature help. */
export function isSignatureHelpProvider(bridge: LanguageBridge): bridge is LanguageBridge & SignatureHelpProvider {
  return "signatureHelp" in bridge && typeof (bridge as Record<string, unknown>).signatureHelp === "function";
}

/** Check if the bridge implements formatting. */
export function isFormatProvider(bridge: LanguageBridge): bridge is LanguageBridge & FormatProvider {
  return "format" in bridge && typeof (bridge as Record<string, unknown>).format === "function";
}
