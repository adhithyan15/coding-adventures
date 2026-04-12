"""LSP handler methods — initialize, shutdown, text sync, and all feature handlers.

These handlers implement the LSP server lifecycle and feature requests.
Every LSP session begins with initialize and ends with shutdown+exit.

Server Lifecycle
-----------------

::

    Client (editor)              Server (us)
      |                               |
      |--initialize------------------->  store clientInfo, return capabilities
      | <--------------result----------  |
      |                               |
      |--initialized (notif)---------->  no-op (handshake complete)
      |                               |
      |--textDocument/didOpen-------->  open doc, parse, push diagnostics
      |--textDocument/didChange------>  apply change, re-parse, push diagnostics
      |--textDocument/hover---------->  get parse result, call bridge.hover
      | <--------------result----------  |
      |                               |
      |--shutdown--------------------->  set shutdown flag, return null
      |--exit (notif)----------------->  (server stops)

Handler Contract
-----------------

Request handlers return ``(result, error)`` where:
- ``result`` is any JSON-serializable value (success)
- ``error`` is a ``ResponseError`` (failure)
- Only one should be non-None.

Notification handlers have no return value.
"""

from __future__ import annotations

import sys
from typing import Any

from json_rpc import ResponseError
from json_rpc import INVALID_PARAMS as JSON_RPC_INVALID_PARAMS

from ls00.capabilities import build_capabilities, encode_semantic_tokens
from ls00.language_bridge import (
    CompletionProvider,
    DefinitionProvider,
    DocumentSymbolsProvider,
    FoldingRangesProvider,
    FormatProvider,
    HoverProvider,
    ReferencesProvider,
    RenameProvider,
    SemanticTokensProvider,
    SignatureHelpProvider,
)
from ls00.lsp_errors import REQUEST_FAILED
from ls00.types import (
    DocumentSymbol,
    Position,
    Range,
    TextEdit,
)

if sys.version_info >= (3, 11):
    from typing import TYPE_CHECKING
else:
    from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ls00.server import LspServer


# ---------------------------------------------------------------------------
# LSP type conversion helpers
# ---------------------------------------------------------------------------


def _position_to_lsp(p: Position) -> dict[str, Any]:
    """Convert our Position to a JSON-serializable dict."""
    return {"line": p.line, "character": p.character}


def _range_to_lsp(r: Range) -> dict[str, Any]:
    """Convert our Range to a JSON-serializable dict."""
    return {"start": _position_to_lsp(r.start), "end": _position_to_lsp(r.end)}


def _location_to_lsp(uri: str, r: Range) -> dict[str, Any]:
    """Convert a Location to a JSON-serializable dict."""
    return {"uri": uri, "range": _range_to_lsp(r)}


def _parse_position(params: dict[str, Any]) -> Position:
    """Extract a Position from a JSON params object.

    The LSP sends positions as ``{"line": N, "character": N}``.
    """
    pos = params.get("position", {})
    line = int(pos.get("line", 0))
    char = int(pos.get("character", 0))
    return Position(line=line, character=char)


def _parse_uri(params: dict[str, Any]) -> str:
    """Extract the document URI from params that have a textDocument field."""
    td = params.get("textDocument", {})
    return str(td.get("uri", ""))


def _parse_lsp_range(raw: Any) -> Range:
    """Parse a raw JSON range object from the LSP protocol.

    The LSP sends ranges as::

        {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 5}}
    """
    if not isinstance(raw, dict):
        return Range(start=Position(0, 0), end=Position(0, 0))

    start_map = raw.get("start", {})
    end_map = raw.get("end", {})

    return Range(
        start=Position(
            line=int(start_map.get("line", 0)),
            character=int(start_map.get("character", 0)),
        ),
        end=Position(
            line=int(end_map.get("line", 0)),
            character=int(end_map.get("character", 0)),
        ),
    )


def _convert_document_symbols(
    symbols: list[DocumentSymbol],
) -> list[dict[str, Any]]:
    """Recursively convert DocumentSymbol objects to JSON-serializable dicts."""
    result: list[dict[str, Any]] = []
    for sym in symbols:
        m: dict[str, Any] = {
            "name": sym.name,
            "kind": int(sym.kind),
            "range": _range_to_lsp(sym.range),
            "selectionRange": _range_to_lsp(sym.selection_range),
        }
        if sym.children:
            m["children"] = _convert_document_symbols(sym.children)
        result.append(m)
    return result


# ---------------------------------------------------------------------------
# Lifecycle handlers
# ---------------------------------------------------------------------------


def handle_initialize(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process the LSP initialize request.

    This is the server's first message. We store the client info (for logging)
    and return our capabilities built from the bridge.
    """
    server.initialized = True

    caps = build_capabilities(server.bridge)

    return {
        "capabilities": caps,
        "serverInfo": {
            "name": "ls00-generic-lsp-server",
            "version": "0.1.0",
        },
    }


def handle_initialized(server: LspServer, params: Any) -> None:
    """Process the "initialized" notification.

    No-op: the handshake is complete. Normal operation begins now.
    """


def handle_shutdown(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process the LSP shutdown request.

    After receiving shutdown, the server should stop processing new
    requests and return null as the result.
    """
    server.shutdown = True
    return None


def handle_exit(server: LspServer, params: Any) -> None:
    """Process the "exit" notification.

    Exit code semantics (from the LSP spec):
    - 0: shutdown was received before exit -> clean shutdown
    - 1: shutdown was NOT received -> abnormal termination
    """
    if server.shutdown:
        sys.exit(0)
    else:
        sys.exit(1)


# ---------------------------------------------------------------------------
# Text document synchronization handlers
# ---------------------------------------------------------------------------


def handle_did_open(server: LspServer, params: Any) -> None:
    """Handle textDocument/didOpen — editor opened a file.

    Params: ``{"textDocument": {"uri": "...", "languageId": "...", "version": 1, "text": "..."}}``
    """
    if not isinstance(params, dict):
        return

    td = params.get("textDocument", {})
    uri = td.get("uri", "")
    text = td.get("text", "")
    version = int(td.get("version", 1))

    if not uri:
        return

    server.doc_manager.open(uri, text, version)

    # Parse immediately and push diagnostics so the editor shows squiggles
    # as soon as the file is opened.
    result = server.parse_cache.get_or_parse(uri, version, text, server.bridge)
    server.publish_diagnostics(uri, version, result.diagnostics)


def handle_did_change(server: LspServer, params: Any) -> None:
    """Handle textDocument/didChange — user edited a file.

    Params: ``{"textDocument": {"uri": "...", "version": 2}, "contentChanges": [...]}``
    """
    if not isinstance(params, dict):
        return

    uri = _parse_uri(params)
    if not uri:
        return

    version = 0
    td = params.get("textDocument", {})
    if isinstance(td, dict):
        version = int(td.get("version", 0))

    from ls00.document_manager import TextChange

    changes_raw = params.get("contentChanges", [])
    changes: list[TextChange] = []

    for change_raw in changes_raw:
        if not isinstance(change_raw, dict):
            continue

        new_text = change_raw.get("text", "")
        range_raw = change_raw.get("range")

        if range_raw is not None:
            r = _parse_lsp_range(range_raw)
            changes.append(TextChange(range=r, new_text=new_text))
        else:
            changes.append(TextChange(range=None, new_text=new_text))

    try:
        server.doc_manager.apply_changes(uri, changes, version)
    except KeyError:
        return

    doc = server.doc_manager.get(uri)
    if doc is None:
        return

    result = server.parse_cache.get_or_parse(
        uri, doc.version, doc.text, server.bridge
    )
    server.publish_diagnostics(uri, version, result.diagnostics)


def handle_did_close(server: LspServer, params: Any) -> None:
    """Handle textDocument/didClose — editor closed a file.

    Removes the document and clears diagnostics.
    """
    if not isinstance(params, dict):
        return

    uri = _parse_uri(params)
    if not uri:
        return

    server.doc_manager.close(uri)
    server.parse_cache.evict(uri)

    # Clear diagnostics for the closed file by publishing an empty list.
    server.publish_diagnostics(uri, 0, [])


def handle_did_save(server: LspServer, params: Any) -> None:
    """Handle textDocument/didSave — editor saved a file.

    If the client sends full text in didSave, apply it.
    """
    if not isinstance(params, dict):
        return

    uri = _parse_uri(params)
    if not uri:
        return

    text = params.get("text")
    if isinstance(text, str) and text:
        doc = server.doc_manager.get(uri)
        if doc is not None:
            server.doc_manager.close(uri)
            server.doc_manager.open(uri, text, doc.version)
            result = server.parse_cache.get_or_parse(
                uri, doc.version, text, server.bridge
            )
            server.publish_diagnostics(uri, doc.version, result.diagnostics)


# ---------------------------------------------------------------------------
# Feature request handlers
# ---------------------------------------------------------------------------


def handle_hover(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/hover."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    pos = _parse_position(params)

    if not isinstance(server.bridge, HoverProvider):
        return None

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return None

    hover_result = server.bridge.hover(parse_result.ast, pos)
    if hover_result is None:
        return None

    result: dict[str, Any] = {
        "contents": {
            "kind": "markdown",
            "value": hover_result.contents,
        },
    }

    if hover_result.range is not None:
        result["range"] = _range_to_lsp(hover_result.range)

    return result


def handle_definition(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/definition."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    pos = _parse_position(params)

    if not isinstance(server.bridge, DefinitionProvider):
        return None

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return None

    location = server.bridge.definition(parse_result.ast, pos, uri)
    if location is None:
        return None

    return _location_to_lsp(location.uri, location.range)


def handle_references(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/references."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    pos = _parse_position(params)

    include_decl = False
    ctx = params.get("context", {})
    if isinstance(ctx, dict):
        include_decl = bool(ctx.get("includeDeclaration", False))

    if not isinstance(server.bridge, ReferencesProvider):
        return []

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return []

    locations = server.bridge.references(parse_result.ast, pos, uri, include_decl)
    return [_location_to_lsp(loc.uri, loc.range) for loc in locations]


def handle_completion(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/completion."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    pos = _parse_position(params)
    empty_list: dict[str, Any] = {"isIncomplete": False, "items": []}

    if not isinstance(server.bridge, CompletionProvider):
        return empty_list

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return empty_list

    items = server.bridge.completion(parse_result.ast, pos)

    lsp_items: list[dict[str, Any]] = []
    for item in items:
        ci: dict[str, Any] = {"label": item.label}
        if item.kind is not None and item.kind != 0:
            ci["kind"] = int(item.kind)
        if item.detail:
            ci["detail"] = item.detail
        if item.documentation:
            ci["documentation"] = item.documentation
        if item.insert_text:
            ci["insertText"] = item.insert_text
        if item.insert_text_format:
            ci["insertTextFormat"] = item.insert_text_format
        lsp_items.append(ci)

    return {"isIncomplete": False, "items": lsp_items}


def handle_rename(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/rename."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    pos = _parse_position(params)
    new_name = params.get("newName", "")

    if not new_name:
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="newName is required")

    if not isinstance(server.bridge, RenameProvider):
        return ResponseError(code=REQUEST_FAILED, message="rename not supported")

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return ResponseError(code=REQUEST_FAILED, message="no AST available")

    edit = server.bridge.rename(parse_result.ast, pos, new_name)
    if edit is None:
        return ResponseError(
            code=REQUEST_FAILED, message="symbol not found at position"
        )

    # Convert WorkspaceEdit to LSP format.
    lsp_changes: dict[str, Any] = {}
    for edit_uri, edits in edit.changes.items():
        lsp_edits: list[dict[str, Any]] = []
        for te in edits:
            lsp_edits.append({
                "range": _range_to_lsp(te.range),
                "newText": te.new_text,
            })
        lsp_changes[edit_uri] = lsp_edits

    return {"changes": lsp_changes}


def handle_document_symbol(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/documentSymbol."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)

    if not isinstance(server.bridge, DocumentSymbolsProvider):
        return []

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return []

    symbols = server.bridge.document_symbols(parse_result.ast)
    return _convert_document_symbols(symbols)


def handle_semantic_tokens_full(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/semanticTokens/full."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    empty_data: dict[str, Any] = {"data": []}

    if not isinstance(server.bridge, SemanticTokensProvider):
        return empty_data

    doc = server.doc_manager.get(uri)
    if doc is None:
        return empty_data

    tokens = server.bridge.tokenize(doc.text)
    sem_tokens = server.bridge.semantic_tokens(doc.text, tokens)
    data = encode_semantic_tokens(sem_tokens)

    return {"data": data}


def handle_folding_range(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/foldingRange."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)

    if not isinstance(server.bridge, FoldingRangesProvider):
        return []

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return []

    ranges = server.bridge.folding_ranges(parse_result.ast)

    result: list[dict[str, Any]] = []
    for fr in ranges:
        m: dict[str, Any] = {
            "startLine": fr.start_line,
            "endLine": fr.end_line,
        }
        if fr.kind:
            m["kind"] = fr.kind
        result.append(m)

    return result


def handle_signature_help(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/signatureHelp."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)
    pos = _parse_position(params)

    if not isinstance(server.bridge, SignatureHelpProvider):
        return None

    doc, parse_result = server.get_parse_result(uri)
    if parse_result is None:
        return ResponseError(code=REQUEST_FAILED, message=f"document not open: {uri}")

    if parse_result.ast is None:
        return None

    sig_help = server.bridge.signature_help(parse_result.ast, pos)
    if sig_help is None:
        return None

    lsp_sigs: list[dict[str, Any]] = []
    for sig in sig_help.signatures:
        lsp_params: list[dict[str, Any]] = []
        for param in sig.parameters:
            pp: dict[str, Any] = {"label": param.label}
            if param.documentation:
                pp["documentation"] = param.documentation
            lsp_params.append(pp)
        s: dict[str, Any] = {
            "label": sig.label,
            "parameters": lsp_params,
        }
        if sig.documentation:
            s["documentation"] = sig.documentation
        lsp_sigs.append(s)

    return {
        "signatures": lsp_sigs,
        "activeSignature": sig_help.active_signature,
        "activeParameter": sig_help.active_parameter,
    }


def handle_formatting(
    server: LspServer, id: Any, params: Any  # noqa: A002
) -> Any:
    """Process textDocument/formatting."""
    if not isinstance(params, dict):
        return ResponseError(code=JSON_RPC_INVALID_PARAMS, message="invalid params")

    uri = _parse_uri(params)

    if not isinstance(server.bridge, FormatProvider):
        return []

    doc = server.doc_manager.get(uri)
    if doc is None:
        return []

    edits = server.bridge.format(doc.text)

    return [
        {"range": _range_to_lsp(edit.range), "newText": edit.new_text}
        for edit in edits
    ]
