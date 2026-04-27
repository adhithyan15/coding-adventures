"""LspServer — the main coordinator.

LspServer wires together:
- The LanguageBridge (language-specific logic)
- The DocumentManager (tracks open file contents)
- The ParseCache (avoids redundant parses)
- The JSON-RPC Server (protocol layer)

It registers all LSP request and notification handlers with the JSON-RPC
server, then calls ``serve()`` to start the blocking read-dispatch-write loop.

Sending Notifications to the Editor
--------------------------------------

The JSON-RPC Server (from the json-rpc package) handles request/response
pairs. But the LSP server also needs to PUSH notifications to the editor
(e.g., ``textDocument/publishDiagnostics``). We do this by holding a
reference to the JSON-RPC MessageWriter and calling ``write_message``
directly.
"""

from __future__ import annotations

from typing import IO, Any

from json_rpc import MessageWriter, Notification, ResponseError, Server

from ls00.capabilities import build_capabilities
from ls00.document_manager import DocumentManager
from ls00.handlers import (
    _range_to_lsp,
    handle_completion,
    handle_definition,
    handle_did_change,
    handle_did_close,
    handle_did_open,
    handle_did_save,
    handle_document_symbol,
    handle_exit,
    handle_folding_range,
    handle_formatting,
    handle_hover,
    handle_initialize,
    handle_initialized,
    handle_references,
    handle_rename,
    handle_semantic_tokens_full,
    handle_shutdown,
    handle_signature_help,
)
from ls00.language_bridge import LanguageBridge
from ls00.parse_cache import ParseCache, ParseResult
from ls00.types import Diagnostic


class LspServer:
    """The main LSP server.

    Create it with ``LspServer(bridge, in_stream, out_stream)``, then call
    ``serve()`` to start serving. It is designed to be used once per process --
    start it, it blocks, it exits.

    Example::

        server = LspServer(my_bridge, sys.stdin.buffer, sys.stdout.buffer)
        server.serve()
    """

    def __init__(
        self,
        bridge: LanguageBridge,
        in_stream: IO[bytes],
        out_stream: IO[bytes],
    ) -> None:
        self.bridge = bridge
        self.doc_manager = DocumentManager()
        self.parse_cache = ParseCache()
        self._rpc_server = Server(in_stream, out_stream)
        self._writer = MessageWriter(out_stream)

        # shutdown tracks whether the editor has sent "shutdown".
        self.shutdown = False
        # initialized tracks whether the initialize handshake is complete.
        self.initialized = False

        self._register_handlers()

    def serve(self) -> None:
        """Start the blocking JSON-RPC read-dispatch-write loop.

        This call blocks until the editor closes the connection (EOF on stdin).
        """
        self._rpc_server.serve()

    def get_parse_result(
        self, uri: str
    ) -> tuple[Any, ParseResult | None]:
        """Retrieve the current parse result for a document.

        This is the hot path for all feature handlers. It:
        1. Gets the current document text from the DocumentManager
        2. Returns the cached ParseResult (or re-parses if needed)

        Returns ``(doc, None)`` if the document is not open.
        """
        doc = self.doc_manager.get(uri)
        if doc is None:
            return None, None

        result = self.parse_cache.get_or_parse(
            uri, doc.version, doc.text, self.bridge
        )
        return doc, result

    def publish_diagnostics(
        self, uri: str, version: int, diagnostics: list[Diagnostic]
    ) -> None:
        """Send textDocument/publishDiagnostics notification to the editor.

        Called after every didOpen and didChange event to update the
        squiggle underlines in the editor.
        """
        lsp_diags: list[dict[str, Any]] = []
        for d in diagnostics:
            diag: dict[str, Any] = {
                "range": _range_to_lsp(d.range),
                "severity": int(d.severity),
                "message": d.message,
            }
            if d.code:
                diag["code"] = d.code
            lsp_diags.append(diag)

        notif_params: dict[str, Any] = {
            "uri": uri,
            "diagnostics": lsp_diags,
        }
        if version > 0:
            notif_params["version"] = version

        # Best-effort: if the write fails, the editor shows stale diagnostics.
        try:
            self._writer.write_message(
                Notification(method="textDocument/publishDiagnostics", params=notif_params)
            )
        except Exception:  # noqa: BLE001
            pass

    def _register_handlers(self) -> None:
        """Wire all LSP method names to their handler functions.

        Requests (have an id, get a response):
            initialize, shutdown, textDocument/hover, textDocument/definition,
            textDocument/references, textDocument/completion, textDocument/rename,
            textDocument/documentSymbol, textDocument/semanticTokens/full,
            textDocument/foldingRange, textDocument/signatureHelp,
            textDocument/formatting

        Notifications (no id, no response):
            initialized, textDocument/didOpen, textDocument/didChange,
            textDocument/didClose, textDocument/didSave
        """
        s = self

        # -- Lifecycle ---------------------------------------------------------
        self._rpc_server.on_request(
            "initialize",
            lambda req_id, params: handle_initialize(s, req_id, params),
        )
        self._rpc_server.on_notification(
            "initialized",
            lambda params: handle_initialized(s, params),
        )
        self._rpc_server.on_request(
            "shutdown",
            lambda req_id, params: handle_shutdown(s, req_id, params),
        )
        self._rpc_server.on_notification(
            "exit",
            lambda params: handle_exit(s, params),
        )

        # -- Text document synchronization ------------------------------------
        self._rpc_server.on_notification(
            "textDocument/didOpen",
            lambda params: handle_did_open(s, params),
        )
        self._rpc_server.on_notification(
            "textDocument/didChange",
            lambda params: handle_did_change(s, params),
        )
        self._rpc_server.on_notification(
            "textDocument/didClose",
            lambda params: handle_did_close(s, params),
        )
        self._rpc_server.on_notification(
            "textDocument/didSave",
            lambda params: handle_did_save(s, params),
        )

        # -- Feature requests (conditional on bridge capability) ---------------
        self._rpc_server.on_request(
            "textDocument/hover",
            lambda req_id, params: handle_hover(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/definition",
            lambda req_id, params: handle_definition(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/references",
            lambda req_id, params: handle_references(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/completion",
            lambda req_id, params: handle_completion(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/rename",
            lambda req_id, params: handle_rename(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/documentSymbol",
            lambda req_id, params: handle_document_symbol(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/semanticTokens/full",
            lambda req_id, params: handle_semantic_tokens_full(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/foldingRange",
            lambda req_id, params: handle_folding_range(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/signatureHelp",
            lambda req_id, params: handle_signature_help(s, req_id, params),
        )
        self._rpc_server.on_request(
            "textDocument/formatting",
            lambda req_id, params: handle_formatting(s, req_id, params),
        )
