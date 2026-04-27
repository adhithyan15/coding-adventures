"""LanguageBridge and optional provider protocols.

Design Philosophy: Narrow Interfaces
--------------------------------------

Python's ``typing.Protocol`` with ``@runtime_checkable`` mirrors Go's
implicit interface satisfaction. A class implements a protocol simply by
having the right methods -- no explicit ``implements`` declaration.

We use many small protocols (one per LSP feature) instead of one large
abstract base class. This is the "interface segregation principle" from
SOLID:

    APPROACH A -- fat interface (one big ABC, every method required)::

        class LanguageBridge(ABC):
            def tokenize(self, source: str) -> list[Token]: ...
            def parse(self, source: str) -> tuple[Any, list[Diagnostic]]: ...
            def hover(self, ast: Any, pos: Position) -> HoverResult | None: ...
            def definition(self, ast: Any, pos: Position, uri: str) -> Location | None: ...
            # ... 8 more methods, all REQUIRED

        Problem: a language that only has a lexer and parser must implement
        ALL methods, even features it doesn't support yet.

    APPROACH B -- narrow protocols (what we use)::

        class LanguageBridge(Protocol):
            def tokenize(self, source: str) -> list[Token]: ...
            def parse(self, source: str) -> tuple[Any, list[Diagnostic]]: ...

        class HoverProvider(Protocol):
            def hover(self, ast: Any, pos: Position) -> HoverResult | None: ...

        At runtime, the server checks: ``isinstance(bridge, HoverProvider)``?
        If yes: advertise ``hoverProvider: true`` and call ``bridge.hover()``.
        If no: omit hover from capabilities. No stubs required.

Runtime Capability Detection
------------------------------

Detection uses Python's ``isinstance()`` with ``@runtime_checkable``
Protocol classes::

    if isinstance(bridge, HoverProvider):
        result = bridge.hover(ast, pos)

This mirrors Go's type assertion pattern: ``if hp, ok := bridge.(HoverProvider); ok``
"""

from __future__ import annotations

from typing import Any, Protocol, runtime_checkable

from ls00.types import (
    CompletionItem,
    Diagnostic,
    DocumentSymbol,
    FoldingRange,
    HoverResult,
    Location,
    Position,
    SemanticToken,
    SignatureHelpResult,
    TextEdit,
    Token,
    WorkspaceEdit,
)


# ---------------------------------------------------------------------------
# Required interface — every language bridge must implement this
# ---------------------------------------------------------------------------


@runtime_checkable
class LanguageBridge(Protocol):
    """The required minimum interface every language must implement.

    ``tokenize`` and ``parse`` are the foundation for all other features:
    - ``tokenize`` drives semantic token highlighting (accurate syntax coloring)
    - ``parse`` drives diagnostics, folding, and document symbols

    All other features (hover, go-to-definition, etc.) are optional and
    declared as separate Protocol classes below. The LspServer checks at
    runtime whether the bridge also implements those protocols.
    """

    def tokenize(self, source: str) -> list[Token]:
        """Lex the source string and return the token stream.

        The tokens are used for semantic highlighting. Each Token carries a
        ``type`` string (e.g. "KEYWORD", "IDENTIFIER"), its ``value``, and
        its 1-based ``line`` and ``column`` position. The bridge is
        responsible for converting 1-based positions to 0-based before
        building SemanticToken values.
        """
        ...

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        """Parse the source string and return (ast, diagnostics).

        Returns:
            A tuple of:
            - ast: the parsed abstract syntax tree (may be partial on error)
            - diagnostics: parse errors and warnings as LSP Diagnostic objects

        Even when there are syntax errors, parse should return a partial AST.
        This allows hover, folding, and symbol features to continue working on
        the valid portions of the file.
        """
        ...


# ---------------------------------------------------------------------------
# Optional Provider Protocols
# ---------------------------------------------------------------------------
#
# Each protocol represents one optional LSP feature. A bridge implements
# only the features its language supports. The server uses isinstance()
# to detect support at runtime.
#
# None of these protocols extend LanguageBridge -- they are purely additive.


@runtime_checkable
class HoverProvider(Protocol):
    """Enables hover tooltips.

    When the user moves their mouse over a symbol, the editor sends
    ``textDocument/hover`` with the cursor position. The bridge should
    return Markdown text describing the symbol (type, documentation, etc.).
    """

    def hover(self, ast: Any, pos: Position) -> HoverResult | None:
        """Return hover information for the AST node at the given position.

        Returns:
            - ``HoverResult`` -- hover content to display
            - ``None`` -- no hover info at this position (not an error)
        """
        ...


@runtime_checkable
class DefinitionProvider(Protocol):
    """Enables "Go to Definition" (F12 in VS Code).

    When the user right-clicks on a symbol and chooses "Go to Definition",
    the editor sends ``textDocument/definition``. The bridge looks up the
    symbol in its symbol table and returns the location where it was declared.
    """

    def definition(self, ast: Any, pos: Position, uri: str) -> Location | None:
        """Return the location where the symbol at ``pos`` was declared.

        Returns:
            - ``Location`` -- the declaration location
            - ``None`` -- symbol not found (not an error)
        """
        ...


@runtime_checkable
class ReferencesProvider(Protocol):
    """Enables "Find All References".

    When the user right-clicks and chooses "Find All References", the editor
    sends ``textDocument/references``. The bridge returns every location
    where the symbol is used.
    """

    def references(
        self, ast: Any, pos: Position, uri: str, include_decl: bool
    ) -> list[Location]:
        """Return all uses of the symbol at ``pos``.

        Args:
            include_decl: if True, include the declaration location in results.
        """
        ...


@runtime_checkable
class CompletionProvider(Protocol):
    """Enables autocomplete suggestions.

    When the user pauses typing or presses Ctrl+Space, the editor sends
    ``textDocument/completion``. The bridge returns a list of valid
    completions at the cursor position.
    """

    def completion(self, ast: Any, pos: Position) -> list[CompletionItem]:
        """Return autocomplete suggestions valid at ``pos``."""
        ...


@runtime_checkable
class RenameProvider(Protocol):
    """Enables symbol rename (F2 in VS Code).

    When the user presses F2 on a symbol, the editor sends
    ``textDocument/rename``. The bridge must find all occurrences of
    the symbol and return text edits that replace each one with ``new_name``.
    """

    def rename(
        self, ast: Any, pos: Position, new_name: str
    ) -> WorkspaceEdit | None:
        """Return the set of text edits needed to rename the symbol at ``pos``."""
        ...


@runtime_checkable
class SemanticTokensProvider(Protocol):
    """Enables accurate syntax highlighting.

    The bridge receives the full token stream from the lexer and maps each
    token to a semantic type (keyword, string, number, variable, function, etc.).
    The framework then encodes the result into LSP's compact binary format.
    """

    def semantic_tokens(
        self, source: str, tokens: list[Token]
    ) -> list[SemanticToken]:
        """Return semantic token data for the whole document.

        ``tokens`` is the output of ``tokenize()`` -- the bridge should use
        these rather than re-lexing. The returned SemanticTokens must be
        sorted by line, then by character (ascending), because the LSP
        encoding is delta-based.
        """
        ...


@runtime_checkable
class DocumentSymbolsProvider(Protocol):
    """Enables the document outline panel.

    The bridge walks the AST looking for declaration nodes and returns
    them as a tree of DocumentSymbol objects.
    """

    def document_symbols(self, ast: Any) -> list[DocumentSymbol]:
        """Return the outline tree for the given AST."""
        ...


@runtime_checkable
class FoldingRangesProvider(Protocol):
    """Enables code folding (collapsible blocks).

    The bridge typically marks any AST node that spans multiple lines
    as foldable.
    """

    def folding_ranges(self, ast: Any) -> list[FoldingRange]:
        """Return collapsible regions derived from the AST structure."""
        ...


@runtime_checkable
class SignatureHelpProvider(Protocol):
    """Enables function signature hints.

    When the user types the opening parenthesis of a function call, the
    editor sends ``textDocument/signatureHelp``. The bridge returns the
    function's signature with the active parameter highlighted.
    """

    def signature_help(
        self, ast: Any, pos: Position
    ) -> SignatureHelpResult | None:
        """Return signature hint information for the call at ``pos``.

        Returns:
            - ``SignatureHelpResult`` -- signature data
            - ``None`` -- not inside a call expression
        """
        ...


@runtime_checkable
class FormatProvider(Protocol):
    """Enables document formatting (Format on Save).

    The bridge returns a list of text edits that transform the source into
    its canonical formatted form.
    """

    def format(self, source: str) -> list[TextEdit]:
        """Return the text edits needed to format the document.

        Typically this is a single edit replacing the entire file content.
        """
        ...
