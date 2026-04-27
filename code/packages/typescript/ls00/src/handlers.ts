/**
 * handlers.ts -- All LSP request and notification handlers
 *
 * This module contains every handler the LspServer registers with the JSON-RPC
 * server. They are organized by lifecycle stage:
 *
 *   1. Lifecycle: initialize, initialized, shutdown, exit
 *   2. Text document sync: didOpen, didChange, didClose, didSave
 *   3. Feature requests: hover, definition, references, completion, rename,
 *      documentSymbol, semanticTokens/full, foldingRange, signatureHelp, formatting
 *
 * Each handler extracts parameters from the raw JSON-RPC params object, delegates
 * to the bridge (via the ParseCache), and returns a result in LSP format.
 *
 * # Handler Patterns
 *
 * Request handlers return `unknown` (the result) or throw. The Server class
 * in json-rpc wraps thrown errors into JSON-RPC error responses.
 *
 * Notification handlers return void. Even if they fail, no response is sent
 * (the LSP spec forbids it).
 *
 * @module
 */

import type { LanguageBridge } from "./language-bridge.js";
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
import type { DocumentManager, Document } from "./document-manager.js";
import type { ParseCache, ParseResult } from "./parse-cache.js";
import type {
  Position,
  Range,
  Diagnostic,
  DocumentSymbol,
  TextChange,
} from "./types.js";
import { buildCapabilities } from "./capabilities.js";
import { encodeSemanticTokens } from "./capabilities.js";
import { LspErrorCodes } from "./lsp-errors.js";

// ---------------------------------------------------------------------------
// Handler context -- shared state passed to all handlers
// ---------------------------------------------------------------------------

/**
 * HandlerContext holds the shared state that all handlers need access to.
 * This is created by the LspServer and passed to each handler function.
 */
export interface HandlerContext {
  bridge: LanguageBridge;
  docManager: DocumentManager;
  parseCache: ParseCache;
  sendNotification: (method: string, params: unknown) => void;
  isInitialized: () => boolean;
  setInitialized: (v: boolean) => void;
  isShutdown: () => boolean;
  setShutdown: (v: boolean) => void;
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/** Extract a Position from params that have a "position" field. */
function parsePosition(params: Record<string, unknown>): Position {
  const pos = (params.position ?? {}) as Record<string, unknown>;
  return {
    line: Number(pos.line ?? 0),
    character: Number(pos.character ?? 0),
  };
}

/** Extract the document URI from params that have a textDocument field. */
function parseURI(params: Record<string, unknown>): string {
  const td = (params.textDocument ?? {}) as Record<string, unknown>;
  return String(td.uri ?? "");
}

/** Convert our Position to a JSON-serializable object. */
function positionToLSP(p: Position): Record<string, unknown> {
  return { line: p.line, character: p.character };
}

/** Convert our Range to a JSON-serializable object. */
function rangeToLSP(r: Range): Record<string, unknown> {
  return { start: positionToLSP(r.start), end: positionToLSP(r.end) };
}

/** Convert a Location to a JSON-serializable object. */
function locationToLSP(loc: { uri: string; range: Range }): Record<string, unknown> {
  return { uri: loc.uri, range: rangeToLSP(loc.range) };
}

/** Parse a raw JSON range object from the LSP protocol. */
function parseLSPRange(raw: unknown): Range {
  const m = (raw ?? {}) as Record<string, unknown>;
  const startMap = (m.start ?? {}) as Record<string, unknown>;
  const endMap = (m.end ?? {}) as Record<string, unknown>;

  return {
    start: { line: Number(startMap.line ?? 0), character: Number(startMap.character ?? 0) },
    end: { line: Number(endMap.line ?? 0), character: Number(endMap.character ?? 0) },
  };
}

/**
 * Get the current parse result for a document.
 * This is the hot path for all feature handlers.
 */
function getParseResult(
  ctx: HandlerContext,
  uri: string,
): { doc: Document; result: ParseResult } {
  const doc = ctx.docManager.get(uri);
  if (!doc) {
    throw { code: LspErrorCodes.RequestFailed, message: `document not open: ${uri}` };
  }
  const result = ctx.parseCache.getOrParse(uri, doc.version, doc.text, ctx.bridge);
  return { doc, result };
}

/**
 * Publish diagnostics for a document to the editor.
 *
 * LSP servers push diagnostics proactively after every parse to update
 * the editor's squiggle underlines.
 */
function publishDiagnostics(
  ctx: HandlerContext,
  uri: string,
  version: number,
  diagnostics: Diagnostic[],
): void {
  const lspDiags = diagnostics.map((d) => {
    const diag: Record<string, unknown> = {
      range: rangeToLSP(d.range),
      severity: d.severity,
      message: d.message,
    };
    if (d.code) {
      diag.code = d.code;
    }
    return diag;
  });

  const params: Record<string, unknown> = {
    uri,
    diagnostics: lspDiags,
  };
  if (version > 0) {
    params.version = version;
  }

  ctx.sendNotification("textDocument/publishDiagnostics", params);
}

// ---------------------------------------------------------------------------
// Lifecycle handlers
// ---------------------------------------------------------------------------

/**
 * Handle the LSP initialize request.
 *
 * This is the server's first message. We store the client info (for logging)
 * and return our capabilities built from the bridge.
 */
export function handleInitialize(
  ctx: HandlerContext,
  _id: string | number,
  _params: unknown,
): unknown {
  ctx.setInitialized(true);

  const caps = buildCapabilities(ctx.bridge);

  return {
    capabilities: caps,
    serverInfo: {
      name: "ls00-generic-lsp-server",
      version: "0.1.0",
    },
  };
}

/**
 * Handle the "initialized" notification.
 *
 * This is the editor's acknowledgment that it received our capabilities.
 * No-op: the handshake is complete.
 */
export function handleInitialized(
  _ctx: HandlerContext,
  _params: unknown,
): void {
  // No-op
}

/**
 * Handle the LSP shutdown request.
 *
 * After receiving shutdown, the server should stop processing new requests
 * and return null.
 */
export function handleShutdown(
  ctx: HandlerContext,
  _id: string | number,
  _params: unknown,
): unknown {
  ctx.setShutdown(true);
  return null;
}

/**
 * Handle the "exit" notification.
 *
 * Exit code semantics (from the LSP spec):
 *   - 0: shutdown was received before exit -> clean shutdown
 *   - 1: shutdown was NOT received -> abnormal termination
 */
export function handleExit(
  ctx: HandlerContext,
  _params: unknown,
): void {
  const exitCode = ctx.isShutdown() ? 0 : 1;
  process.exit(exitCode);
}

// ---------------------------------------------------------------------------
// Text document synchronization handlers
// ---------------------------------------------------------------------------

/**
 * Handle textDocument/didOpen -- the editor opened a file.
 *
 * Params: {"textDocument": {"uri": "...", "languageId": "...", "version": 1, "text": "..."}}
 */
export function handleDidOpen(
  ctx: HandlerContext,
  params: unknown,
): void {
  const p = params as Record<string, unknown>;
  const td = (p.textDocument ?? {}) as Record<string, unknown>;

  const uri = String(td.uri ?? "");
  const text = String(td.text ?? "");
  let version = 1;
  if (typeof td.version === "number") {
    version = td.version;
  }

  if (!uri) return;

  ctx.docManager.open(uri, text, version);

  // Parse immediately and push diagnostics so the editor shows squiggles
  // as soon as the file is opened.
  const result = ctx.parseCache.getOrParse(uri, version, text, ctx.bridge);
  publishDiagnostics(ctx, uri, version, result.diagnostics);
}

/**
 * Handle textDocument/didChange -- the user edited a file.
 *
 * Params: {"textDocument": {"uri": "...", "version": 2}, "contentChanges": [...]}
 */
export function handleDidChange(
  ctx: HandlerContext,
  params: unknown,
): void {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  if (!uri) return;

  let version = 0;
  const td = (p.textDocument ?? {}) as Record<string, unknown>;
  if (typeof td.version === "number") {
    version = td.version;
  }

  // Parse the content changes array.
  const changesRaw = (p.contentChanges ?? []) as unknown[];
  const changes: TextChange[] = [];

  for (const changeRaw of changesRaw) {
    const changeMap = changeRaw as Record<string, unknown>;
    const newText = String(changeMap.text ?? "");
    const change: TextChange = { newText };

    if (changeMap.range !== undefined && changeMap.range !== null) {
      change.range = parseLSPRange(changeMap.range);
    }

    changes.push(change);
  }

  // Apply the changes to the document manager.
  try {
    ctx.docManager.applyChanges(uri, changes, version);
  } catch {
    // Document wasn't open (e.g., a race condition). Ignore.
    return;
  }

  // Get the updated text and re-parse.
  const doc = ctx.docManager.get(uri);
  if (!doc) return;

  const result = ctx.parseCache.getOrParse(uri, doc.version, doc.text, ctx.bridge);
  publishDiagnostics(ctx, uri, version, result.diagnostics);
}

/**
 * Handle textDocument/didClose -- the editor closed a file.
 *
 * We remove the document from our manager and evict its parse cache entry.
 * We also clear diagnostics by publishing an empty list.
 */
export function handleDidClose(
  ctx: HandlerContext,
  params: unknown,
): void {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  if (!uri) return;

  ctx.docManager.close(uri);
  ctx.parseCache.evict(uri);

  // Clear diagnostics for the closed file.
  publishDiagnostics(ctx, uri, 0, []);
}

/**
 * Handle textDocument/didSave -- the editor saved a file.
 *
 * If the client sends full text in didSave, apply it. Otherwise, just
 * republish diagnostics from the current parse state.
 */
export function handleDidSave(
  ctx: HandlerContext,
  params: unknown,
): void {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  if (!uri) return;

  const text = p.text;
  if (typeof text === "string" && text !== "") {
    const doc = ctx.docManager.get(uri);
    if (doc) {
      ctx.docManager.close(uri);
      ctx.docManager.open(uri, text, doc.version);
      const result = ctx.parseCache.getOrParse(uri, doc.version, text, ctx.bridge);
      publishDiagnostics(ctx, uri, doc.version, result.diagnostics);
    }
  }
}

// ---------------------------------------------------------------------------
// Feature request handlers
// ---------------------------------------------------------------------------

/**
 * Handle textDocument/hover -- show tooltip on mouse-over.
 */
export function handleHover(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  const pos = parsePosition(p);

  if (!isHoverProvider(ctx.bridge)) {
    return null;
  }

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) return null;

  const hoverResult = ctx.bridge.hover(parseResult.ast, pos);
  if (!hoverResult) return null;

  const result: Record<string, unknown> = {
    contents: { kind: "markdown", value: hoverResult.contents },
  };

  if (hoverResult.range) {
    result.range = rangeToLSP(hoverResult.range);
  }

  return result;
}

/**
 * Handle textDocument/definition -- Go to Definition (F12).
 */
export function handleDefinition(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  const pos = parsePosition(p);

  if (!isDefinitionProvider(ctx.bridge)) return null;

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) return null;

  const location = ctx.bridge.definition(parseResult.ast, pos, uri);
  if (!location) return null;

  return locationToLSP(location);
}

/**
 * Handle textDocument/references -- Find All References.
 */
export function handleReferences(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  const pos = parsePosition(p);

  // Extract includeDeclaration from the context object.
  let includeDecl = false;
  const ctxObj = p.context as Record<string, unknown> | undefined;
  if (ctxObj && typeof ctxObj.includeDeclaration === "boolean") {
    includeDecl = ctxObj.includeDeclaration;
  }

  if (!isReferencesProvider(ctx.bridge)) return [];

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) return [];

  const locations = ctx.bridge.references(parseResult.ast, pos, uri, includeDecl);
  return locations.map(locationToLSP);
}

/**
 * Handle textDocument/completion -- autocomplete.
 */
export function handleCompletion(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  const pos = parsePosition(p);

  if (!isCompletionProvider(ctx.bridge)) {
    return { isIncomplete: false, items: [] };
  }

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) {
    return { isIncomplete: false, items: [] };
  }

  const items = ctx.bridge.completion(parseResult.ast, pos);

  const lspItems = items.map((item) => {
    const ci: Record<string, unknown> = { label: item.label };
    if (item.kind) ci.kind = item.kind;
    if (item.detail) ci.detail = item.detail;
    if (item.documentation) ci.documentation = item.documentation;
    if (item.insertText) ci.insertText = item.insertText;
    if (item.insertTextFormat) ci.insertTextFormat = item.insertTextFormat;
    return ci;
  });

  return { isIncomplete: false, items: lspItems };
}

/**
 * Handle textDocument/rename -- symbol rename (F2).
 */
export function handleRename(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  const pos = parsePosition(p);
  const newName = String(p.newName ?? "");

  if (!newName) {
    throw { code: -32602, message: "newName is required" };
  }

  if (!isRenameProvider(ctx.bridge)) {
    throw { code: LspErrorCodes.RequestFailed, message: "rename not supported" };
  }

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) {
    throw { code: LspErrorCodes.RequestFailed, message: "no AST available" };
  }

  const edit = ctx.bridge.rename(parseResult.ast, pos, newName);
  if (!edit) {
    throw { code: LspErrorCodes.RequestFailed, message: "symbol not found at position" };
  }

  // Convert WorkspaceEdit to LSP format.
  const lspChanges: Record<string, unknown[]> = {};
  for (const [editURI, edits] of Object.entries(edit.changes)) {
    lspChanges[editURI] = edits.map((te) => ({
      range: rangeToLSP(te.range),
      newText: te.newText,
    }));
  }

  return { changes: lspChanges };
}

/**
 * Handle textDocument/documentSymbol -- outline panel.
 */
export function handleDocumentSymbol(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);

  if (!isDocumentSymbolsProvider(ctx.bridge)) return [];

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) return [];

  const symbols = ctx.bridge.documentSymbols(parseResult.ast);
  return convertDocumentSymbols(symbols);
}

/**
 * Recursively convert DocumentSymbol arrays to JSON-serializable objects.
 */
function convertDocumentSymbols(symbols: DocumentSymbol[]): unknown[] {
  return symbols.map((sym) => {
    const m: Record<string, unknown> = {
      name: sym.name,
      kind: sym.kind,
      range: rangeToLSP(sym.range),
      selectionRange: rangeToLSP(sym.selectionRange),
    };
    if (sym.children && sym.children.length > 0) {
      m.children = convertDocumentSymbols(sym.children);
    }
    return m;
  });
}

/**
 * Handle textDocument/semanticTokens/full -- semantic highlighting.
 */
export function handleSemanticTokensFull(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);

  if (!isSemanticTokensProvider(ctx.bridge)) {
    return { data: [] };
  }

  const doc = ctx.docManager.get(uri);
  if (!doc) return { data: [] };

  // Tokenize the source to get raw tokens for the bridge.
  const tokens = ctx.bridge.tokenize(doc.text);

  // Ask the bridge to map tokens to semantic types.
  const semTokens = ctx.bridge.semanticTokens(doc.text, tokens);

  // Encode using the compact LSP delta format.
  const data = encodeSemanticTokens(semTokens);

  return { data };
}

/**
 * Handle textDocument/foldingRange -- code folding.
 */
export function handleFoldingRange(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);

  if (!isFoldingRangesProvider(ctx.bridge)) return [];

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) return [];

  const ranges = ctx.bridge.foldingRanges(parseResult.ast);

  return ranges.map((fr) => {
    const m: Record<string, unknown> = {
      startLine: fr.startLine,
      endLine: fr.endLine,
    };
    if (fr.kind) m.kind = fr.kind;
    return m;
  });
}

/**
 * Handle textDocument/signatureHelp -- function signature tooltip.
 */
export function handleSignatureHelp(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);
  const pos = parsePosition(p);

  if (!isSignatureHelpProvider(ctx.bridge)) return null;

  const { result: parseResult } = getParseResult(ctx, uri);
  if (parseResult.ast == null) return null;

  const sigHelp = ctx.bridge.signatureHelp(parseResult.ast, pos);
  if (!sigHelp) return null;

  // Convert to LSP format.
  const lspSigs = sigHelp.signatures.map((sig) => {
    const lspParams = (sig.parameters ?? []).map((param) => {
      const pp: Record<string, unknown> = { label: param.label };
      if (param.documentation) pp.documentation = param.documentation;
      return pp;
    });
    const s: Record<string, unknown> = {
      label: sig.label,
      parameters: lspParams,
    };
    if (sig.documentation) s.documentation = sig.documentation;
    return s;
  });

  return {
    signatures: lspSigs,
    activeSignature: sigHelp.activeSignature,
    activeParameter: sigHelp.activeParameter,
  };
}

/**
 * Handle textDocument/formatting -- document formatting.
 */
export function handleFormatting(
  ctx: HandlerContext,
  _id: string | number,
  params: unknown,
): unknown {
  const p = params as Record<string, unknown>;
  const uri = parseURI(p);

  if (!isFormatProvider(ctx.bridge)) return [];

  const doc = ctx.docManager.get(uri);
  if (!doc) return [];

  const edits = ctx.bridge.format(doc.text);

  return edits.map((edit) => ({
    range: rangeToLSP(edit.range),
    newText: edit.newText,
  }));
}
