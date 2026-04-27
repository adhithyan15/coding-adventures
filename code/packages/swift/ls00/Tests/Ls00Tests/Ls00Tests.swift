// ============================================================================
// Ls00Tests.swift — comprehensive tests for the ls00 LSP framework
// ============================================================================
//
// # Test Strategy
//
// We test the framework with a MockBridge that implements all optional
// provider protocols. This lets us exercise every code path without
// needing a real language implementation.
//
// # Test Coverage Areas
//
//  1. UTF-16 offset conversion (critical for correctness)
//  2. DocumentManager open/change/close operations
//  3. ParseCache hit/miss behavior
//  4. Semantic token encoding (the delta format)
//  5. Capabilities advertisement (only what the bridge supports)
//  6. Full LSP lifecycle via JSON-RPC round-trips
//
// ============================================================================

import XCTest
@testable import Ls00
import JsonRpc
import Foundation

// ============================================================================
// MockBridge — full-featured test bridge
// ============================================================================

/// Test bridge implementing LanguageBridge + HoverProvider + DocumentSymbolsProvider.
final class MockBridge: LanguageBridge, HoverProvider, DocumentSymbolsProvider {
    var hoverResult: HoverResult? = HoverResult(contents: "**test** hover")

    func tokenize(source: String) -> ([Token], Error?) {
        var tokens: [Token] = []
        var col = 1
        for word in source.split(separator: " ") {
            tokens.append(Token(type: "WORD", value: String(word), line: 1, column: col))
            col += word.count + 1
        }
        return (tokens, nil)
    }

    func parse(source: String) -> (ASTNode?, [Diagnostic], Error?) {
        var diags: [Diagnostic] = []
        if source.contains("ERROR") {
            diags.append(Diagnostic(
                range: Range(
                    start: Position(line: 0, character: 0),
                    end: Position(line: 0, character: 5)
                ),
                severity: .error,
                message: "syntax error: unexpected ERROR token"
            ))
        }
        return (source as ASTNode, diags, nil)
    }

    func hover(ast: ASTNode, pos: Position) -> (HoverResult?, Error?) {
        return (hoverResult, nil)
    }

    func documentSymbols(ast: ASTNode) -> ([DocumentSymbol], Error?) {
        return ([
            DocumentSymbol(
                name: "main",
                kind: .function,
                range: Range(start: Position(line: 0, character: 0), end: Position(line: 10, character: 1)),
                selectionRange: Range(start: Position(line: 0, character: 9), end: Position(line: 0, character: 13)),
                children: [
                    DocumentSymbol(
                        name: "x",
                        kind: .variable,
                        range: Range(start: Position(line: 1, character: 4), end: Position(line: 1, character: 12)),
                        selectionRange: Range(start: Position(line: 1, character: 8), end: Position(line: 1, character: 9))
                    ),
                ]
            ),
        ], nil)
    }
}

/// Minimal bridge implementing ONLY the required LanguageBridge protocol.
/// Used to test that optional capabilities are NOT advertised.
final class MinimalBridge: LanguageBridge {
    func tokenize(source: String) -> ([Token], Error?) {
        return ([], nil)
    }
    func parse(source: String) -> (ASTNode?, [Diagnostic], Error?) {
        return (source as ASTNode, [], nil)
    }
}

// ============================================================================
// UTF-16 Offset Conversion Tests
// ============================================================================

final class UTF16Tests: XCTestCase {

    /// This is the most important correctness test. If UTF-16 conversion is wrong,
    /// every position-dependent feature breaks: hover, definition, references, etc.
    func testConvertUTF16OffsetToByteOffset() {
        // ASCII simple: "hello world", char 6 -> byte 6
        XCTAssertEqual(convertUTF16OffsetToByteOffset("hello world", line: 0, char: 6), 6)

        // Start of file
        XCTAssertEqual(convertUTF16OffsetToByteOffset("abc", line: 0, char: 0), 0)

        // End of short string
        XCTAssertEqual(convertUTF16OffsetToByteOffset("abc", line: 0, char: 3), 3)

        // Second line: "hello\nworld", line 1 starts at byte 6
        XCTAssertEqual(convertUTF16OffsetToByteOffset("hello\nworld", line: 1, char: 0), 6)
    }

    func testEmojiUTF16Conversion() {
        // "A🎸B"
        // UTF-8 bytes: A(1) + 🎸(4) + B(1) = 6 bytes
        // UTF-16 units: A(1) + 🎸(2) + B(1) = 4 units
        // "B" is at UTF-16 char 3, byte offset 5
        let text = "A\u{1F3B8}B"
        XCTAssertEqual(convertUTF16OffsetToByteOffset(text, line: 0, char: 3), 5)
    }

    func testEmojiAtStart() {
        // "🎸hello" — 🎸 = 2 UTF-16 units = 4 UTF-8 bytes
        // "h" is at UTF-16 char 2, byte offset 4
        let text = "\u{1F3B8}hello"
        XCTAssertEqual(convertUTF16OffsetToByteOffset(text, line: 0, char: 2), 4)
    }

    func testTwoByteUTF8() {
        // "cafe!" — e is U+00E9 (2 UTF-8 bytes, 1 UTF-16 code unit)
        // UTF-16 char 4 = byte offset 5 (c=1, a=1, f=1, e=2)
        let text = "caf\u{00e9}!"
        XCTAssertEqual(convertUTF16OffsetToByteOffset(text, line: 0, char: 4), 5)
    }

    func testMultilineWithEmoji() {
        // line 0: "A🎸B\n" (7 bytes)
        // line 1: "hello"
        let text = "A\u{1F3B8}B\nhello"
        XCTAssertEqual(convertUTF16OffsetToByteOffset(text, line: 1, char: 0), 7)
    }

    func testBeyondLineEndClamps() {
        // Character past line end should clamp to newline position
        let text = "ab\ncd"
        XCTAssertEqual(convertUTF16OffsetToByteOffset(text, line: 0, char: 100), 2)
    }
}

// ============================================================================
// DocumentManager Tests
// ============================================================================

final class DocumentManagerTests: XCTestCase {

    func testOpen() {
        let dm = DocumentManager()
        dm.open(uri: "file:///test.txt", text: "hello world", version: 1)

        let doc = dm.get(uri: "file:///test.txt")
        XCTAssertNotNil(doc)
        XCTAssertEqual(doc?.text, "hello world")
        XCTAssertEqual(doc?.version, 1)
    }

    func testGetMissing() {
        let dm = DocumentManager()
        XCTAssertNil(dm.get(uri: "file:///nonexistent.txt"))
    }

    func testClose() {
        let dm = DocumentManager()
        dm.open(uri: "file:///test.txt", text: "hello", version: 1)
        dm.close(uri: "file:///test.txt")
        XCTAssertNil(dm.get(uri: "file:///test.txt"))
    }

    func testApplyChangesFullReplacement() {
        let dm = DocumentManager()
        dm.open(uri: "file:///test.txt", text: "hello world", version: 1)

        let err = dm.applyChanges(uri: "file:///test.txt", changes: [
            TextChange(newText: "goodbye world"),
        ], version: 2)
        XCTAssertNil(err)

        let doc = dm.get(uri: "file:///test.txt")!
        XCTAssertEqual(doc.text, "goodbye world")
        XCTAssertEqual(doc.version, 2)
    }

    func testApplyChangesIncremental() {
        let dm = DocumentManager()
        dm.open(uri: "file:///test.txt", text: "hello world", version: 1)

        // Replace "world" (chars 6-11) with "Go"
        let err = dm.applyChanges(uri: "file:///test.txt", changes: [
            TextChange(
                range: Range(
                    start: Position(line: 0, character: 6),
                    end: Position(line: 0, character: 11)
                ),
                newText: "Go"
            ),
        ], version: 2)
        XCTAssertNil(err)

        let doc = dm.get(uri: "file:///test.txt")!
        XCTAssertEqual(doc.text, "hello Go")
    }

    func testApplyChangesNotOpen() {
        let dm = DocumentManager()
        let err = dm.applyChanges(uri: "file:///notopen.txt", changes: [
            TextChange(newText: "x"),
        ], version: 1)
        XCTAssertNotNil(err)
    }

    func testIncrementalWithEmoji() {
        // "A🎸B" — replace "B" (UTF-16 char 3-4) with "X"
        let dm = DocumentManager()
        dm.open(uri: "file:///test.txt", text: "A\u{1F3B8}B", version: 1)

        let err = dm.applyChanges(uri: "file:///test.txt", changes: [
            TextChange(
                range: Range(
                    start: Position(line: 0, character: 3),
                    end: Position(line: 0, character: 4)
                ),
                newText: "X"
            ),
        ], version: 2)
        XCTAssertNil(err)

        let doc = dm.get(uri: "file:///test.txt")!
        XCTAssertEqual(doc.text, "A\u{1F3B8}X")
    }
}

// ============================================================================
// ParseCache Tests
// ============================================================================

final class ParseCacheTests: XCTestCase {

    func testHitAndMiss() {
        let bridge = MockBridge()
        let cache = ParseCache()

        // First call — cache miss
        let r1 = cache.getOrParse(uri: "file:///a.txt", version: 1, source: "hello", bridge: bridge)
        XCTAssertNotNil(r1)

        // Second call same version — cache hit (same object)
        let r2 = cache.getOrParse(uri: "file:///a.txt", version: 1, source: "hello", bridge: bridge)
        XCTAssertTrue(r1 === r2, "Expected same object on cache hit")

        // Different version — cache miss
        let r3 = cache.getOrParse(uri: "file:///a.txt", version: 2, source: "hello world", bridge: bridge)
        XCTAssertFalse(r3 === r1, "Expected different object for new version")
    }

    func testEvict() {
        let bridge = MockBridge()
        let cache = ParseCache()

        let r1 = cache.getOrParse(uri: "file:///a.txt", version: 1, source: "hello", bridge: bridge)
        cache.evict(uri: "file:///a.txt")

        let r2 = cache.getOrParse(uri: "file:///a.txt", version: 1, source: "hello", bridge: bridge)
        XCTAssertFalse(r1 === r2, "Expected new result after eviction")
    }

    func testDiagnosticsPopulated() {
        let bridge = MockBridge()
        let cache = ParseCache()

        let result = cache.getOrParse(uri: "file:///a.txt", version: 1, source: "source with ERROR token", bridge: bridge)
        XCTAssertFalse(result.diagnostics.isEmpty, "Expected diagnostics for ERROR source")
    }
}

// ============================================================================
// Capabilities Tests
// ============================================================================

final class CapabilitiesTests: XCTestCase {

    func testMinimalBridge() {
        let bridge = MinimalBridge()
        let caps = buildCapabilities(bridge)

        // Always present
        XCTAssertEqual(caps["textDocumentSync"] as? Int, 2)

        // Optional capabilities should NOT be present
        let optionalCaps = [
            "hoverProvider", "definitionProvider", "referencesProvider",
            "completionProvider", "renameProvider", "documentSymbolProvider",
            "foldingRangeProvider", "signatureHelpProvider",
            "documentFormattingProvider", "semanticTokensProvider",
        ]
        for cap in optionalCaps {
            XCTAssertNil(caps[cap], "minimal bridge should not advertise \(cap)")
        }
    }

    func testFullBridge() {
        let bridge = MockBridge()
        let caps = buildCapabilities(bridge)

        // MockBridge implements HoverProvider and DocumentSymbolsProvider
        XCTAssertNotNil(caps["hoverProvider"])
        XCTAssertNotNil(caps["documentSymbolProvider"])
    }
}

// ============================================================================
// Semantic Token Encoding Tests
// ============================================================================

final class SemanticTokenTests: XCTestCase {

    func testEncodeEmpty() {
        let data = encodeSemanticTokens([])
        XCTAssertEqual(data.count, 0)
    }

    func testEncodeSingleToken() {
        let tokens = [
            SemanticToken(line: 0, character: 0, length: 5, tokenType: "keyword"),
        ]
        let data = encodeSemanticTokens(tokens)

        // Expected: [0, 0, 5, 15, 0]
        XCTAssertEqual(data.count, 5)
        XCTAssertEqual(data[0], 0, "deltaLine")
        XCTAssertEqual(data[1], 0, "deltaChar")
        XCTAssertEqual(data[2], 5, "length")
        XCTAssertEqual(data[3], 15, "keyword = index 15")
        XCTAssertEqual(data[4], 0, "no modifiers")
    }

    func testEncodeMultipleTokensSameLine() {
        let tokens = [
            SemanticToken(line: 0, character: 0, length: 3, tokenType: "keyword"),
            SemanticToken(line: 0, character: 4, length: 4, tokenType: "function", modifiers: ["declaration"]),
        ]
        let data = encodeSemanticTokens(tokens)

        XCTAssertEqual(data.count, 10)
        // Token A: [0, 0, 3, 15, 0]
        XCTAssertEqual(Array(data[0..<5]), [0, 0, 3, 15, 0])
        // Token B: [0, 4, 4, 12, 1] (function=12, declaration=bit0=1)
        XCTAssertEqual(Array(data[5..<10]), [0, 4, 4, 12, 1])
    }

    func testEncodeMultipleLines() {
        let tokens = [
            SemanticToken(line: 0, character: 0, length: 3, tokenType: "keyword"),
            SemanticToken(line: 2, character: 4, length: 5, tokenType: "number"),
        ]
        let data = encodeSemanticTokens(tokens)

        XCTAssertEqual(data.count, 10)
        // Token B: deltaLine=2, deltaChar=4 (absolute on new line), number=19
        XCTAssertEqual(data[5], 2, "deltaLine for token B")
        XCTAssertEqual(data[6], 4, "deltaChar for token B (absolute)")
        XCTAssertEqual(data[8], 19, "number = index 19")
    }

    func testEncodeUnsortedInput() {
        // Tokens in reverse order — encoder should sort them
        let tokens = [
            SemanticToken(line: 2, character: 0, length: 3, tokenType: "keyword"),
            SemanticToken(line: 0, character: 0, length: 5, tokenType: "string"),
        ]
        let data = encodeSemanticTokens(tokens)

        XCTAssertEqual(data.count, 10)
        // First token should be line 0 (string, index 18)
        XCTAssertEqual(data[0], 0, "first deltaLine")
        XCTAssertEqual(data[3], 18, "string = index 18")
        // Second token should be line 2 (keyword, index 15)
        XCTAssertEqual(data[5], 2, "second deltaLine")
        XCTAssertEqual(data[8], 15, "keyword = index 15")
    }

    func testEncodeSkipsUnknownTypes() {
        let tokens = [
            SemanticToken(line: 0, character: 0, length: 5, tokenType: "NONEXISTENT"),
        ]
        let data = encodeSemanticTokens(tokens)
        XCTAssertEqual(data.count, 0, "Unknown type should be skipped")
    }
}

// ============================================================================
// Server Integration Tests
// ============================================================================

final class ServerTests: XCTestCase {

    /// Helper to build a Content-Length-framed JSON-RPC message string.
    func frame(_ json: String) -> String {
        let payload = Data(json.utf8)
        return "Content-Length: \(payload.count)\r\n\r\n" + json
    }

    func testInitializeReturnsCapabilities() throws {
        let json = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let inputData = Data(frame(json).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("capabilities"),
                       "Expected capabilities in response: \(responseStr)")
        XCTAssertTrue(responseStr.contains("hoverProvider"),
                       "Expected hoverProvider for MockBridge: \(responseStr)")
    }

    func testDidOpenPublishesDiagnostics() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let didOpenJson = """
        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.bf","languageId":"bf","version":1,"text":"hello world"}}}
        """
        let inputData = Data((frame(initJson) + frame(didOpenJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        // Should contain both the initialize response and publishDiagnostics notification
        XCTAssertTrue(responseStr.contains("capabilities"))
        XCTAssertTrue(responseStr.contains("publishDiagnostics") || responseStr.contains("diagnostics"),
                       "Expected diagnostics publication: \(responseStr)")
    }

    func testDidOpenWithErrorSource() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let didOpenJson = """
        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.bf","languageId":"bf","version":1,"text":"code with ERROR here"}}}
        """
        let inputData = Data((frame(initJson) + frame(didOpenJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("syntax error"),
                       "Expected error diagnostic in output: \(responseStr)")
    }

    func testHoverRequest() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let didOpenJson = """
        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.bf","languageId":"bf","version":1,"text":"hello world"}}}
        """
        let hoverJson = """
        {"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///test.bf"},"position":{"line":0,"character":0}}}
        """
        let inputData = Data((frame(initJson) + frame(didOpenJson) + frame(hoverJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("test"),
                       "Expected hover content in response: \(responseStr)")
    }

    func testDocumentSymbolRequest() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let didOpenJson = """
        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.bf","languageId":"bf","version":1,"text":"hello world"}}}
        """
        let symbolJson = """
        {"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///test.bf"}}}
        """
        let inputData = Data((frame(initJson) + frame(didOpenJson) + frame(symbolJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("main"),
                       "Expected 'main' symbol in response: \(responseStr)")
    }

    func testShutdownSetsFlag() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let shutdownJson = """
        {"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}
        """
        let inputData = Data((frame(initJson) + frame(shutdownJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        XCTAssertTrue(server.isShutdown)
    }

    func testMinimalBridgeCapabilities() throws {
        let json = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let inputData = Data(frame(json).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MinimalBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertFalse(responseStr.contains("hoverProvider"),
                        "Minimal bridge should NOT advertise hoverProvider: \(responseStr)")
    }

    func testDidChangeThenHover() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let didOpenJson = """
        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.bf","languageId":"bf","version":1,"text":"hello"}}}
        """
        let didChangeJson = """
        {"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///test.bf","version":2},"contentChanges":[{"text":"hello world"}]}}
        """
        let hoverJson = """
        {"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///test.bf"},"position":{"line":0,"character":0}}}
        """
        let inputData = Data((frame(initJson) + frame(didOpenJson) + frame(didChangeJson) + frame(hoverJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        let responseStr = output.string
        XCTAssertTrue(responseStr.contains("test"),
                       "Expected hover content after change: \(responseStr)")
    }

    func testDidCloseClearsDiagnostics() throws {
        let initJson = """
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}
        """
        let didOpenJson = """
        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.bf","languageId":"bf","version":1,"text":"hello ERROR"}}}
        """
        let didCloseJson = """
        {"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file:///test.bf"}}}
        """
        let inputData = Data((frame(initJson) + frame(didOpenJson) + frame(didCloseJson)).utf8)
        let output = DataOutput()

        let server = LspServer(bridge: MockBridge(), inputData: inputData, output: output)
        server.serve()

        // After close, the document should be gone from the manager
        XCTAssertNil(server.docManager.get(uri: "file:///test.bf"))
    }
}

// ============================================================================
// Semantic Token Legend Tests
// ============================================================================

final class SemanticTokenLegendTests: XCTestCase {

    func testLegendContainsStandardTypes() {
        let legend = semanticTokenLegend()
        XCTAssertTrue(legend.tokenTypes.contains("keyword"))
        XCTAssertTrue(legend.tokenTypes.contains("function"))
        XCTAssertTrue(legend.tokenTypes.contains("variable"))
        XCTAssertTrue(legend.tokenTypes.contains("string"))
        XCTAssertTrue(legend.tokenTypes.contains("number"))
    }

    func testLegendContainsStandardModifiers() {
        let legend = semanticTokenLegend()
        XCTAssertTrue(legend.tokenModifiers.contains("declaration"))
        XCTAssertTrue(legend.tokenModifiers.contains("definition"))
        XCTAssertTrue(legend.tokenModifiers.contains("readonly"))
    }

    func testTokenTypeIndex() {
        XCTAssertEqual(tokenTypeIndex("keyword"), 15)
        XCTAssertEqual(tokenTypeIndex("function"), 12)
        XCTAssertEqual(tokenTypeIndex("variable"), 8)
        XCTAssertEqual(tokenTypeIndex("NONEXISTENT"), -1)
    }

    func testTokenModifierMask() {
        XCTAssertEqual(tokenModifierMask(["declaration"]), 1)    // bit 0
        XCTAssertEqual(tokenModifierMask(["definition"]), 2)     // bit 1
        XCTAssertEqual(tokenModifierMask(["declaration", "definition"]), 3) // bits 0+1
        XCTAssertEqual(tokenModifierMask([]), 0)
        XCTAssertEqual(tokenModifierMask(["NONEXISTENT"]), 0)
    }
}

// ============================================================================
// LSP Error Codes Tests
// ============================================================================

final class LspErrorCodesTests: XCTestCase {

    func testErrorCodeValues() {
        XCTAssertEqual(LspErrorCodes.serverNotInitialized, -32002)
        XCTAssertEqual(LspErrorCodes.unknownErrorCode, -32001)
        XCTAssertEqual(LspErrorCodes.requestFailed, -32803)
        XCTAssertEqual(LspErrorCodes.serverCancelled, -32802)
        XCTAssertEqual(LspErrorCodes.contentModified, -32801)
        XCTAssertEqual(LspErrorCodes.requestCancelled, -32800)
    }
}
