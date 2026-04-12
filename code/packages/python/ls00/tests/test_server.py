"""Tests for the LspServer — lifecycle, handlers, and JSON-RPC round-trips.

These tests feed JSON-RPC messages through the full pipeline using io pipes.
The server runs in a thread; the test feeds messages and reads responses.

We use threading + pipes because the server blocks in serve() waiting for
input. A pipe provides the blocking reads the server expects.
"""

from __future__ import annotations

import io
import json
import threading
from typing import Any

import pytest

from json_rpc import (
    MessageReader,
    MessageWriter,
    Notification,
    Request,
    Response,
)
from ls00 import (
    CompletionItem,
    CompletionItemKind,
    Diagnostic,
    DiagnosticSeverity,
    DocumentSymbol,
    FoldingRange,
    HoverResult,
    Location,
    LspServer,
    ParseCache,
    Position,
    Range,
    SemanticToken,
    SignatureHelpResult,
    SignatureInformation,
    ParameterInformation,
    SymbolKind,
    TextEdit,
    Token,
    WorkspaceEdit,
    build_capabilities,
    SERVER_NOT_INITIALIZED,
    UNKNOWN_ERROR_CODE,
    REQUEST_FAILED,
    SERVER_CANCELLED,
    CONTENT_MODIFIED,
    REQUEST_CANCELLED,
)


# ---------------------------------------------------------------------------
# Mock Bridges
# ---------------------------------------------------------------------------


class MockBridge:
    """Test bridge with HoverProvider + DocumentSymbolsProvider."""

    def __init__(self, hover_result: HoverResult | None = None) -> None:
        self.hover_result = hover_result

    def tokenize(self, source: str) -> list[Token]:
        tokens: list[Token] = []
        col = 1
        for word in source.split():
            tokens.append(Token(type="WORD", value=word, line=1, column=col))
            col += len(word) + 1
        return tokens

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        diags: list[Diagnostic] = []
        if "ERROR" in source:
            diags.append(Diagnostic(
                range=Range(
                    start=Position(line=0, character=0),
                    end=Position(line=0, character=5),
                ),
                severity=DiagnosticSeverity.ERROR,
                message="syntax error: unexpected ERROR token",
            ))
        return source, diags

    def hover(self, ast: Any, pos: Position) -> HoverResult | None:
        return self.hover_result

    def document_symbols(self, ast: Any) -> list[DocumentSymbol]:
        return [DocumentSymbol(
            name="main",
            kind=SymbolKind.FUNCTION,
            range=Range(Position(0, 0), Position(10, 1)),
            selection_range=Range(Position(0, 9), Position(0, 13)),
            children=[DocumentSymbol(
                name="x",
                kind=SymbolKind.VARIABLE,
                range=Range(Position(1, 4), Position(1, 12)),
                selection_range=Range(Position(1, 8), Position(1, 9)),
            )],
        )]


class MinimalBridge:
    """Implements ONLY the required LanguageBridge protocol."""

    def tokenize(self, source: str) -> list[Token]:
        return []

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        return source, []


class FullMockBridge(MockBridge):
    """Extends MockBridge with ALL optional provider interfaces."""

    def semantic_tokens(
        self, source: str, tokens: list[Token]
    ) -> list[SemanticToken]:
        result: list[SemanticToken] = []
        for tok in tokens:
            result.append(SemanticToken(
                line=tok.line - 1,
                character=tok.column - 1,
                length=len(tok.value),
                token_type="variable",
            ))
        return result

    def definition(self, ast: Any, pos: Position, uri: str) -> Location | None:
        return Location(uri=uri, range=Range(start=pos, end=pos))

    def references(
        self, ast: Any, pos: Position, uri: str, include_decl: bool
    ) -> list[Location]:
        return [Location(uri=uri, range=Range(start=pos, end=pos))]

    def completion(self, ast: Any, pos: Position) -> list[CompletionItem]:
        return [CompletionItem(
            label="foo",
            kind=CompletionItemKind.FUNCTION,
            detail="() void",
        )]

    def rename(
        self, ast: Any, pos: Position, new_name: str
    ) -> WorkspaceEdit | None:
        return WorkspaceEdit(changes={
            "file:///test.txt": [TextEdit(
                range=Range(start=pos, end=pos),
                new_text=new_name,
            )],
        })

    def folding_ranges(self, ast: Any) -> list[FoldingRange]:
        return [FoldingRange(start_line=0, end_line=5, kind="region")]

    def signature_help(
        self, ast: Any, pos: Position
    ) -> SignatureHelpResult | None:
        return SignatureHelpResult(
            signatures=[SignatureInformation(
                label="foo(a int, b string)",
                parameters=[
                    ParameterInformation(label="a int"),
                    ParameterInformation(label="b string"),
                ],
            )],
        )

    def format(self, source: str) -> list[TextEdit]:
        return [TextEdit(
            range=Range(Position(0, 0), Position(999, 0)),
            new_text=source,
        )]


# ---------------------------------------------------------------------------
# Pipe-based test server utilities
# ---------------------------------------------------------------------------


class _PipeServer:
    """Wraps an LspServer with pipe-based IO for testing.

    The server runs in a background thread. The test writes messages via
    ``client_writer`` and reads responses via ``client_reader``.
    """

    def __init__(self, bridge: Any) -> None:
        # Use os-level pipes for proper blocking behavior.
        # We use BytesIO-backed wrappers via threading for simplicity.
        import os

        # Client -> Server pipe
        self._in_r_fd, self._in_w_fd = os.pipe()
        # Server -> Client pipe
        self._out_r_fd, self._out_w_fd = os.pipe()

        in_r = os.fdopen(self._in_r_fd, "rb")
        in_w = os.fdopen(self._in_w_fd, "wb")
        out_r = os.fdopen(self._out_r_fd, "rb")
        out_w = os.fdopen(self._out_w_fd, "wb")

        self._in_w = in_w
        self._out_r = out_r

        self.server = LspServer(bridge, in_r, out_w)
        self.client_writer = MessageWriter(in_w)
        self.client_reader = MessageReader(out_r)

        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def _run(self) -> None:
        try:
            self.server.serve()
        except Exception:
            pass

    def close(self) -> None:
        """Close the write pipe to trigger server EOF."""
        try:
            self._in_w.close()
        except Exception:
            pass


def _send_request(
    ps: _PipeServer, req_id: int, method: str, params: Any
) -> dict[str, Any] | None:
    """Send a request and read the response."""
    ps.client_writer.write_message(Request(id=req_id, method=method, params=params))

    msg = ps.client_reader.read_message()
    if msg is None:
        return None
    assert isinstance(msg, Response)
    assert msg.id == req_id

    if msg.error is not None:
        return {"__error": {"code": msg.error.code, "message": msg.error.message}}

    if msg.result is None:
        return None

    if isinstance(msg.result, dict):
        return msg.result

    return {"__result": msg.result}


def _send_notif(ps: _PipeServer, method: str, params: Any) -> None:
    """Send a notification (no response expected)."""
    ps.client_writer.write_message(Notification(method=method, params=params))


def _read_notif(ps: _PipeServer) -> Notification:
    """Read the next message, expecting a notification."""
    msg = ps.client_reader.read_message()
    assert msg is not None
    assert isinstance(msg, Notification)
    return msg


def _init_server(ps: _PipeServer) -> dict[str, Any] | None:
    """Perform the initialize/initialized handshake."""
    result = _send_request(ps, 1, "initialize", {
        "processId": 1234,
        "capabilities": {},
    })
    _send_notif(ps, "initialized", {})
    return result


def _open_doc(
    ps: _PipeServer, uri: str, text: str, version: int = 1
) -> Notification:
    """Open a document and consume the publishDiagnostics notification."""
    _send_notif(ps, "textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "test",
            "version": version,
            "text": text,
        },
    })
    return _read_notif(ps)  # consume publishDiagnostics


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestLSPErrorCodes:
    """Verify the error code constants match the LSP specification."""

    def test_server_not_initialized(self) -> None:
        assert SERVER_NOT_INITIALIZED == -32002

    def test_unknown_error_code(self) -> None:
        assert UNKNOWN_ERROR_CODE == -32001

    def test_request_failed(self) -> None:
        assert REQUEST_FAILED == -32803

    def test_server_cancelled(self) -> None:
        assert SERVER_CANCELLED == -32802

    def test_content_modified(self) -> None:
        assert CONTENT_MODIFIED == -32801

    def test_request_cancelled(self) -> None:
        assert REQUEST_CANCELLED == -32800


class TestNewLspServer:
    """Verify the constructor returns a usable server."""

    def test_creates_server(self) -> None:
        bridge = MockBridge()
        server = LspServer(bridge, io.BytesIO(), io.BytesIO())
        assert server is not None


class TestHandlerInitialize:
    """Test the initialize handler via full JSON-RPC pipeline."""

    def test_returns_capabilities(self) -> None:
        bridge = MockBridge(hover_result=HoverResult(contents="test"))
        ps = _PipeServer(bridge)
        try:
            result = _init_server(ps)
            assert result is not None
            caps = result["capabilities"]
            assert caps["textDocumentSync"] == 2
            assert caps["hoverProvider"] is True
            assert caps["documentSymbolProvider"] is True

            server_info = result["serverInfo"]
            assert server_info["name"] == "ls00-generic-lsp-server"
        finally:
            ps.close()


class TestHandlerDidOpenPublishesDiagnostics:
    """Test that opening a file pushes diagnostics."""

    def test_error_source_has_diagnostics(self) -> None:
        bridge = MockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)

            _send_notif(ps, "textDocument/didOpen", {
                "textDocument": {
                    "uri": "file:///test.txt",
                    "languageId": "test",
                    "version": 1,
                    "text": "hello ERROR world",
                },
            })

            notif = _read_notif(ps)
            assert notif.method == "textDocument/publishDiagnostics"
            params = notif.params
            assert params["uri"] == "file:///test.txt"
            diags = params["diagnostics"]
            assert len(diags) > 0
        finally:
            ps.close()

    def test_clean_source_has_empty_diagnostics(self) -> None:
        bridge = MockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)

            _send_notif(ps, "textDocument/didOpen", {
                "textDocument": {
                    "uri": "file:///clean.txt",
                    "languageId": "test",
                    "version": 1,
                    "text": "hello world",
                },
            })

            notif = _read_notif(ps)
            assert notif.method == "textDocument/publishDiagnostics"
            params = notif.params
            diags = params["diagnostics"]
            assert len(diags) == 0
        finally:
            ps.close()


class TestHandlerHover:
    """Test the hover handler end-to-end."""

    def test_hover_with_result(self) -> None:
        bridge = MockBridge(hover_result=HoverResult(
            contents="**main** function",
            range=Range(Position(0, 0), Position(0, 4)),
        ))
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.go", "func main() {}")

            result = _send_request(ps, 2, "textDocument/hover", {
                "textDocument": {"uri": "file:///test.go"},
                "position": {"line": 0, "character": 5},
            })

            assert result is not None
            contents = result["contents"]
            assert contents["kind"] == "markdown"
            assert contents["value"] == "**main** function"
        finally:
            ps.close()

    def test_hover_no_bridge(self) -> None:
        """Minimal bridge returns null hover."""
        bridge = MinimalBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello")

            result = _send_request(ps, 2, "textDocument/hover", {
                "textDocument": {"uri": "file:///test.txt"},
                "position": {"line": 0, "character": 0},
            })

            assert result is None
        finally:
            ps.close()


class TestHandlerDocumentSymbol:
    """Test the documentSymbol handler."""

    def test_returns_symbols(self) -> None:
        bridge = MockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.go", "func main() { var x = 1 }")

            result = _send_request(ps, 2, "textDocument/documentSymbol", {
                "textDocument": {"uri": "file:///test.go"},
            })

            assert result is not None
            arr = result["__result"]
            assert len(arr) > 0
            assert arr[0]["name"] == "main"
        finally:
            ps.close()


class TestHandlerSemanticTokensFull:
    """Test the semanticTokens/full handler."""

    def test_returns_data(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello world")

            result = _send_request(ps, 2, "textDocument/semanticTokens/full", {
                "textDocument": {"uri": "file:///test.txt"},
            })

            assert result is not None
            assert "data" in result
        finally:
            ps.close()


class TestHandlerDefinition:
    """Test the definition handler."""

    def test_returns_location(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello world")

            result = _send_request(ps, 2, "textDocument/definition", {
                "textDocument": {"uri": "file:///test.txt"},
                "position": {"line": 0, "character": 0},
            })

            assert result is not None
            assert result["uri"] == "file:///test.txt"
        finally:
            ps.close()


class TestHandlerReferences:
    """Test the references handler."""

    def test_returns_locations(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello")

            result = _send_request(ps, 2, "textDocument/references", {
                "textDocument": {"uri": "file:///test.txt"},
                "position": {"line": 0, "character": 0},
                "context": {"includeDeclaration": True},
            })

            assert result is not None
            arr = result["__result"]
            assert len(arr) > 0
        finally:
            ps.close()


class TestHandlerCompletion:
    """Test the completion handler."""

    def test_returns_items(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "foo")

            result = _send_request(ps, 2, "textDocument/completion", {
                "textDocument": {"uri": "file:///test.txt"},
                "position": {"line": 0, "character": 3},
            })

            assert result is not None
            items = result["items"]
            assert len(items) > 0
        finally:
            ps.close()


class TestHandlerRename:
    """Test the rename handler."""

    def test_returns_changes(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "let x = 1")

            result = _send_request(ps, 2, "textDocument/rename", {
                "textDocument": {"uri": "file:///test.txt"},
                "position": {"line": 0, "character": 4},
                "newName": "y",
            })

            assert result is not None
            assert result["changes"] is not None
        finally:
            ps.close()


class TestHandlerFoldingRange:
    """Test the foldingRange handler."""

    def test_returns_ranges(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "func main() {\n  hello\n}")

            result = _send_request(ps, 2, "textDocument/foldingRange", {
                "textDocument": {"uri": "file:///test.txt"},
            })

            assert result is not None
            arr = result["__result"]
            assert len(arr) > 0
        finally:
            ps.close()


class TestHandlerSignatureHelp:
    """Test the signatureHelp handler."""

    def test_returns_signatures(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "foo(")

            result = _send_request(ps, 2, "textDocument/signatureHelp", {
                "textDocument": {"uri": "file:///test.txt"},
                "position": {"line": 0, "character": 4},
            })

            assert result is not None
            sigs = result["signatures"]
            assert len(sigs) > 0
        finally:
            ps.close()


class TestHandlerFormatting:
    """Test the formatting handler."""

    def test_returns_edits(self) -> None:
        bridge = FullMockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello  world")

            result = _send_request(ps, 2, "textDocument/formatting", {
                "textDocument": {"uri": "file:///test.txt"},
                "options": {"tabSize": 2, "insertSpaces": True},
            })

            assert result is not None
            arr = result["__result"]
            assert len(arr) > 0
        finally:
            ps.close()


class TestHandlerDidChange:
    """Test that didChange updates the document and pushes diagnostics."""

    def test_change_triggers_diagnostics(self) -> None:
        bridge = MockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello world")

            # Change document to add ERROR
            _send_notif(ps, "textDocument/didChange", {
                "textDocument": {"uri": "file:///test.txt", "version": 2},
                "contentChanges": [{"text": "hello ERROR world"}],
            })

            notif = _read_notif(ps)
            assert notif.method == "textDocument/publishDiagnostics"
            params = notif.params
            diags = params["diagnostics"]
            assert len(diags) > 0
        finally:
            ps.close()


class TestHandlerDidClose:
    """Test that didClose clears diagnostics."""

    def test_close_clears_diagnostics(self) -> None:
        bridge = MockBridge()
        ps = _PipeServer(bridge)
        try:
            _init_server(ps)
            _open_doc(ps, "file:///test.txt", "hello")

            _send_notif(ps, "textDocument/didClose", {
                "textDocument": {"uri": "file:///test.txt"},
            })

            notif = _read_notif(ps)
            assert notif.method == "textDocument/publishDiagnostics"
            params = notif.params
            diags = params["diagnostics"]
            assert len(diags) == 0
        finally:
            ps.close()


class TestHandlerShutdown:
    """Test the shutdown handler."""

    def test_shutdown_returns_null(self) -> None:
        bridge = MockBridge()
        ps = _PipeServer(bridge)
        try:
            _send_request(ps, 1, "initialize", {
                "processId": 1, "capabilities": {},
            })

            result = _send_request(ps, 2, "shutdown", None)
            assert result is None
        finally:
            ps.close()


class TestDocumentSymbolConversion:
    """Test nested symbol conversion."""

    def test_nested_symbols(self) -> None:
        bridge = MockBridge()
        cache = ParseCache()
        from ls00 import DocumentManager

        dm = DocumentManager()
        dm.open("file:///a.go", "func main() {}", 1)
        doc = dm.get("file:///a.go")
        assert doc is not None

        result = cache.get_or_parse("file:///a.go", doc.version, doc.text, bridge)
        assert result is not None

        syms = bridge.document_symbols(result.ast)
        assert len(syms) == 1
        assert syms[0].name == "main"
        assert syms[0].kind == SymbolKind.FUNCTION
        assert len(syms[0].children) == 1
        assert syms[0].children[0].name == "x"
