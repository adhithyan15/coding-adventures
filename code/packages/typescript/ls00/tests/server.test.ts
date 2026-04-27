/**
 * server.test.ts -- Handler integration tests via JSON-RPC round-trip
 *
 * These tests feed JSON-RPC messages through the full LspServer pipeline.
 * The server runs in a goroutine-equivalent (async background), and the test
 * sends messages via pipes and reads responses.
 *
 * # Test Pattern
 *
 * 1. Create an LspServer with a MockBridge connected to io.Pipe equivalents
 * 2. Start server.serve() in the background
 * 3. Send JSON-RPC messages via the client writer
 * 4. Read responses via the client reader
 * 5. Assert on the response content
 *
 * We use PassThrough streams (Node.js equivalent of io.Pipe) for in-memory
 * bidirectional communication.
 */

import { describe, it, expect } from "vitest";
import { PassThrough } from "node:stream";
import { MessageReader, MessageWriter } from "@coding-adventures/json-rpc";
import type { Message, Request, Notification, Response } from "@coding-adventures/json-rpc";
import { LspServer } from "../src/server.js";
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
// Mock bridges
// ---------------------------------------------------------------------------

/** MockBridge implements hover and documentSymbols. */
class MockBridge implements LanguageBridge, HoverProvider, DocumentSymbolsProvider {
  hoverResult: HoverResult | null = null;

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
        message: "syntax error: unexpected ERROR token",
      });
    }
    return [source, diags];
  }

  hover(_ast: unknown, _pos: Position): HoverResult | null {
    return this.hoverResult;
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

/** FullMockBridge with all optional providers. */
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

  format(source: string): TextEdit[] {
    return [{
      range: { start: { line: 0, character: 0 }, end: { line: 999, character: 0 } },
      newText: source,
    }];
  }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/**
 * Create an LspServer with pipe-based IO for testing.
 * Returns the client's writer and reader for sending/receiving messages.
 */
function createTestServer(bridge: LanguageBridge): {
  clientWriter: MessageWriter;
  clientReader: MessageReader;
  done: Promise<void>;
} {
  // Client writes to inStream, server reads from inStream
  const inStream = new PassThrough();
  // Server writes to outStream, client reads from outStream
  const outStream = new PassThrough();

  const server = new LspServer(bridge, inStream, outStream);

  // Start the server in the background
  const done = server.serve().then(() => {
    outStream.end();
  });

  const clientWriter = new MessageWriter(inStream);
  const clientReader = new MessageReader(outStream);

  return { clientWriter, clientReader, done };
}

/** Send a JSON-RPC request and read the response. */
async function sendRequest(
  writer: MessageWriter,
  reader: MessageReader,
  id: number,
  method: string,
  params: unknown,
): Promise<Record<string, unknown> | null> {
  writer.writeMessage({
    type: "request" as const,
    id,
    method,
    params,
  });

  const msg = await reader.readMessage();
  if (!msg || msg.type !== "response") {
    throw new Error(`expected response, got ${msg?.type}`);
  }
  const resp = msg as Response;

  if (resp.error) {
    return { __error: resp.error };
  }
  if (resp.result === null || resp.result === undefined) {
    return null;
  }
  if (typeof resp.result === "object" && !Array.isArray(resp.result)) {
    return resp.result as Record<string, unknown>;
  }
  return { __result: resp.result };
}

/** Send a JSON-RPC notification (no response expected). */
function sendNotif(
  writer: MessageWriter,
  method: string,
  params: unknown,
): void {
  writer.writeMessage({
    type: "notification" as const,
    method,
    params,
  });
}

/** Read the next notification from the reader. */
async function readNotif(reader: MessageReader): Promise<Notification> {
  const msg = await reader.readMessage();
  if (!msg || msg.type !== "notification") {
    throw new Error(`expected notification, got ${msg?.type}`);
  }
  return msg as Notification;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("LspServer integration", () => {
  it("initialize returns capabilities", async () => {
    const bridge = new MockBridge();
    bridge.hoverResult = { contents: "test" };
    const { clientWriter, clientReader } = createTestServer(bridge);

    const result = await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1234,
      capabilities: {},
    });

    expect(result).toBeDefined();
    const caps = result!.capabilities as Record<string, unknown>;
    expect(caps.textDocumentSync).toBe(2);
    expect(caps.hoverProvider).toBe(true);
    expect(caps.documentSymbolProvider).toBe(true);

    const serverInfo = result!.serverInfo as Record<string, unknown>;
    expect(serverInfo.name).toBe("ls00-generic-lsp-server");
  });

  it("didOpen publishes diagnostics for error source", async () => {
    const bridge = new MockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    // Initialize
    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    // Open a file with ERROR
    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello ERROR world",
      },
    });

    const notif = await readNotif(clientReader);
    expect(notif.method).toBe("textDocument/publishDiagnostics");

    const params = notif.params as Record<string, unknown>;
    expect(params.uri).toBe("file:///test.txt");
    const diags = params.diagnostics as unknown[];
    expect(diags.length).toBeGreaterThan(0);
  });

  it("didOpen publishes empty diagnostics for clean source", async () => {
    const bridge = new MockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///clean.txt",
        languageId: "test",
        version: 1,
        text: "hello world",
      },
    });

    const notif = await readNotif(clientReader);
    expect(notif.method).toBe("textDocument/publishDiagnostics");

    const params = notif.params as Record<string, unknown>;
    const diags = params.diagnostics as unknown[];
    expect(diags).toHaveLength(0);
  });

  it("hover returns content with range", async () => {
    const bridge = new MockBridge();
    bridge.hoverResult = {
      contents: "**main** function",
      range: {
        start: { line: 0, character: 0 },
        end: { line: 0, character: 4 },
      },
    };
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.go",
        languageId: "go",
        version: 1,
        text: "func main() {}",
      },
    });
    await readNotif(clientReader); // consume publishDiagnostics

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/hover", {
      textDocument: { uri: "file:///test.go" },
      position: { line: 0, character: 5 },
    });

    expect(result).toBeDefined();
    const contents = result!.contents as Record<string, unknown>;
    expect(contents.kind).toBe("markdown");
    expect(contents.value).toBe("**main** function");
    expect(result!.range).toBeDefined();
  });

  it("hover returns null when bridge returns null", async () => {
    const bridge = new MockBridge();
    bridge.hoverResult = null;
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.go",
        languageId: "go",
        version: 1,
        text: "hello",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/hover", {
      textDocument: { uri: "file:///test.go" },
      position: { line: 0, character: 0 },
    });

    expect(result).toBeNull();
  });

  it("didChange triggers re-parse and new diagnostics", async () => {
    const bridge = new MockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    // Open clean file
    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello",
      },
    });
    const notif1 = await readNotif(clientReader);
    const diags1 = (notif1.params as Record<string, unknown>).diagnostics as unknown[];
    expect(diags1).toHaveLength(0);

    // Change to include ERROR
    sendNotif(clientWriter, "textDocument/didChange", {
      textDocument: { uri: "file:///test.txt", version: 2 },
      contentChanges: [{ text: "hello ERROR" }],
    });
    const notif2 = await readNotif(clientReader);
    const diags2 = (notif2.params as Record<string, unknown>).diagnostics as unknown[];
    expect(diags2.length).toBeGreaterThan(0);
  });

  it("didClose clears diagnostics", async () => {
    const bridge = new MockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello ERROR",
      },
    });
    await readNotif(clientReader); // consume open diagnostics

    sendNotif(clientWriter, "textDocument/didClose", {
      textDocument: { uri: "file:///test.txt" },
    });

    const notif = await readNotif(clientReader);
    expect(notif.method).toBe("textDocument/publishDiagnostics");
    const params = notif.params as Record<string, unknown>;
    const diags = params.diagnostics as unknown[];
    expect(diags).toHaveLength(0);
  });

  it("documentSymbol returns nested symbols", async () => {
    const bridge = new MockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///a.go",
        languageId: "go",
        version: 1,
        text: "func main() {}",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/documentSymbol", {
      textDocument: { uri: "file:///a.go" },
    });

    expect(result).toBeDefined();
    const symbols = result!.__result as unknown[];
    expect(symbols).toHaveLength(1);

    const mainSym = symbols[0] as Record<string, unknown>;
    expect(mainSym.name).toBe("main");
    expect(mainSym.kind).toBe(SymbolKind.Function);

    const children = mainSym.children as unknown[];
    expect(children).toHaveLength(1);
    expect((children[0] as Record<string, unknown>).name).toBe("x");
  });

  it("definition handler returns location", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/definition", {
      textDocument: { uri: "file:///test.txt" },
      position: { line: 0, character: 0 },
    });

    expect(result).toBeDefined();
    expect(result!.uri).toBe("file:///test.txt");
  });

  it("completion handler returns items", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/completion", {
      textDocument: { uri: "file:///test.txt" },
      position: { line: 0, character: 0 },
    });

    expect(result).toBeDefined();
    expect(result!.isIncomplete).toBe(false);
    const items = result!.items as unknown[];
    expect(items).toHaveLength(1);
    expect((items[0] as Record<string, unknown>).label).toBe("foo");
  });

  it("references handler returns locations", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/references", {
      textDocument: { uri: "file:///test.txt" },
      position: { line: 0, character: 0 },
      context: { includeDeclaration: true },
    });

    expect(result).toBeDefined();
    const locs = result!.__result as unknown[];
    expect(locs).toHaveLength(1);
  });

  it("foldingRange handler returns ranges", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/foldingRange", {
      textDocument: { uri: "file:///test.txt" },
    });

    expect(result).toBeDefined();
    const ranges = result!.__result as unknown[];
    expect(ranges).toHaveLength(1);
    expect((ranges[0] as Record<string, unknown>).startLine).toBe(0);
    expect((ranges[0] as Record<string, unknown>).endLine).toBe(5);
  });

  it("signatureHelp handler returns signatures", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "foo(a, b)",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/signatureHelp", {
      textDocument: { uri: "file:///test.txt" },
      position: { line: 0, character: 4 },
    });

    expect(result).toBeDefined();
    const sigs = result!.signatures as unknown[];
    expect(sigs).toHaveLength(1);
    expect(result!.activeSignature).toBe(0);
    expect(result!.activeParameter).toBe(0);
  });

  it("formatting handler returns edits", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello world",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/formatting", {
      textDocument: { uri: "file:///test.txt" },
      options: { tabSize: 4, insertSpaces: true },
    });

    expect(result).toBeDefined();
    const edits = result!.__result as unknown[];
    expect(edits).toHaveLength(1);
  });

  it("semanticTokens handler returns encoded data", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello world",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/semanticTokens/full", {
      textDocument: { uri: "file:///test.txt" },
    });

    expect(result).toBeDefined();
    const data = result!.data as number[];
    // "hello world" -> 2 tokens -> 10 integers
    expect(data).toHaveLength(10);
  });

  it("rename handler returns workspace edit", async () => {
    const bridge = new FullMockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });
    sendNotif(clientWriter, "initialized", {});

    sendNotif(clientWriter, "textDocument/didOpen", {
      textDocument: {
        uri: "file:///test.txt",
        languageId: "test",
        version: 1,
        text: "hello world",
      },
    });
    await readNotif(clientReader);

    const result = await sendRequest(clientWriter, clientReader, 2, "textDocument/rename", {
      textDocument: { uri: "file:///test.txt" },
      position: { line: 0, character: 0 },
      newName: "hi",
    });

    expect(result).toBeDefined();
    const changes = result!.changes as Record<string, unknown[]>;
    expect(changes["file:///test.txt"]).toHaveLength(1);
  });

  it("shutdown returns null", async () => {
    const bridge = new MockBridge();
    const { clientWriter, clientReader } = createTestServer(bridge);

    await sendRequest(clientWriter, clientReader, 1, "initialize", {
      processId: 1, capabilities: {},
    });

    const result = await sendRequest(clientWriter, clientReader, 2, "shutdown", {});
    expect(result).toBeNull();
  });
});
