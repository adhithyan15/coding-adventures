"""ls00 — Generic Language Server Protocol (LSP) framework.

This package implements a generic LSP server framework that language-specific
"bridges" plug into. The framework handles all protocol boilerplate:
JSON-RPC transport, document synchronization, parse caching, and capability
advertisement. A language author only writes the LanguageBridge that connects
their lexer/parser to this framework.

Architecture
------------

::

    Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs

How to use this package
------------------------

1. Implement the ``LanguageBridge`` protocol (and any optional provider
   protocols) for your language.
2. Call ``LspServer(bridge, sys.stdin.buffer, sys.stdout.buffer)``.
3. Call ``server.serve()`` -- it blocks until the editor closes the connection.

Quick start
-----------

::

    import sys
    from ls00 import LspServer

    class MyBridge:
        def tokenize(self, source):
            return []  # your lexer here

        def parse(self, source):
            return (None, [])  # your parser here

    server = LspServer(MyBridge(), sys.stdin.buffer, sys.stdout.buffer)
    server.serve()
"""

from ls00.capabilities import (
    build_capabilities,
    encode_semantic_tokens,
    semantic_token_legend,
)
from ls00.document_manager import (
    Document,
    DocumentManager,
    TextChange,
    convert_utf16_offset_to_byte_offset,
)
from ls00.language_bridge import (
    CompletionProvider,
    DefinitionProvider,
    DocumentSymbolsProvider,
    FoldingRangesProvider,
    FormatProvider,
    HoverProvider,
    LanguageBridge,
    ReferencesProvider,
    RenameProvider,
    SemanticTokensProvider,
    SignatureHelpProvider,
)
from ls00.lsp_errors import (
    CONTENT_MODIFIED,
    REQUEST_CANCELLED,
    REQUEST_FAILED,
    SERVER_CANCELLED,
    SERVER_NOT_INITIALIZED,
    UNKNOWN_ERROR_CODE,
)
from ls00.parse_cache import ParseCache, ParseResult
from ls00.server import LspServer
from ls00.types import (
    CompletionItem,
    CompletionItemKind,
    Diagnostic,
    DiagnosticSeverity,
    DocumentSymbol,
    FoldingRange,
    HoverResult,
    Location,
    ParameterInformation,
    Position,
    Range,
    SemanticToken,
    SignatureHelpResult,
    SignatureInformation,
    SymbolKind,
    TextEdit,
    Token,
    WorkspaceEdit,
)

__all__ = [
    # Server
    "LspServer",
    # Bridge protocols
    "LanguageBridge",
    "HoverProvider",
    "DefinitionProvider",
    "ReferencesProvider",
    "CompletionProvider",
    "RenameProvider",
    "SemanticTokensProvider",
    "DocumentSymbolsProvider",
    "FoldingRangesProvider",
    "SignatureHelpProvider",
    "FormatProvider",
    # Types
    "Position",
    "Range",
    "Location",
    "DiagnosticSeverity",
    "Diagnostic",
    "Token",
    "TextEdit",
    "WorkspaceEdit",
    "HoverResult",
    "CompletionItemKind",
    "CompletionItem",
    "SemanticToken",
    "SymbolKind",
    "DocumentSymbol",
    "FoldingRange",
    "ParameterInformation",
    "SignatureInformation",
    "SignatureHelpResult",
    # Document management
    "Document",
    "DocumentManager",
    "TextChange",
    "convert_utf16_offset_to_byte_offset",
    # Parse cache
    "ParseCache",
    "ParseResult",
    # Capabilities
    "build_capabilities",
    "encode_semantic_tokens",
    "semantic_token_legend",
    # Error codes
    "SERVER_NOT_INITIALIZED",
    "UNKNOWN_ERROR_CODE",
    "REQUEST_FAILED",
    "SERVER_CANCELLED",
    "CONTENT_MODIFIED",
    "REQUEST_CANCELLED",
]
