"""LSP types — all shared data types used across the server.

These types mirror the LSP specification's TypeScript type definitions,
translated to idiomatic Python with dataclasses and enums.

The LSP spec lives at:
https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/

Coordinate System
-----------------

LSP uses a 0-based, line/character coordinate system. Line 0, character 0
is the very first character of the file. This differs from most editors
(which display 1-based line numbers) and from most lexers (which emit
1-based tokens). The LanguageBridge is responsible for converting.

UTF-16 Code Units
-----------------

LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
Unicode codepoints. This is a historical artifact: VS Code is built on
TypeScript, which uses UTF-16 strings internally. See document_manager.py
for the conversion function and a detailed explanation of why this matters.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum
from typing import Any


# ---------------------------------------------------------------------------
# Position and Range — the fundamental coordinate types
# ---------------------------------------------------------------------------


@dataclass
class Position:
    """A cursor position in a document.

    Both ``line`` and ``character`` are 0-based. ``character`` is measured
    in UTF-16 code units (see the module doc above for why).

    Example: in the string ``"hello guitar_emoji world"``, the guitar emoji
    (U+1F3B8) occupies UTF-16 characters 6 and 7 (it requires two UTF-16
    surrogates). ``"world"`` starts at UTF-16 character 8.
    """

    line: int
    character: int


@dataclass
class Range:
    """A span of text in a document, from start (inclusive) to end (exclusive).

    Analogy: think of it like a text selection. ``start`` is where the
    cursor lands when you click, ``end`` is where you drag to.
    """

    start: Position
    end: Position


@dataclass
class Location:
    """A position in a specific file.

    ``uri`` uses the ``file://`` scheme, e.g., ``"file:///home/user/main.py"``.
    """

    uri: str
    range: Range


# ---------------------------------------------------------------------------
# Diagnostic — errors, warnings, hints displayed in the editor
# ---------------------------------------------------------------------------


class DiagnosticSeverity(IntEnum):
    """How serious a diagnostic is. These match the LSP integer codes.

    The editor renders diagnostics as underlined squiggles:
    - Red squiggles = Error
    - Yellow squiggles = Warning
    - Blue squiggles = Information
    - Faint underline = Hint
    """

    ERROR = 1
    WARNING = 2
    INFORMATION = 3
    HINT = 4


@dataclass
class Diagnostic:
    """An error, warning, or hint to display in the editor.

    The editor renders diagnostics as underlined squiggles, with the
    message shown on hover.
    """

    range: Range
    severity: DiagnosticSeverity
    message: str
    code: str = ""  # optional: e.g. "E001"


# ---------------------------------------------------------------------------
# Token — a single lexical token from the language's lexer
# ---------------------------------------------------------------------------


@dataclass
class Token:
    """A single lexical token from the language's lexer.

    The bridge's ``tokenize()`` method returns a list of these. The LSP
    server uses tokens to provide semantic syntax highlighting
    (SemanticTokensProvider).

    Note: ``line`` and ``column`` are 1-based (matching most lexers). The
    bridge must convert to 0-based when building SemanticToken values for
    the LSP response.
    """

    type: str  # e.g. "KEYWORD", "IDENTIFIER", "STRING_LIT"
    value: str  # the actual source text, e.g. "let" or "myVar"
    line: int  # 1-based line number
    column: int  # 1-based column number


# ---------------------------------------------------------------------------
# TextEdit and WorkspaceEdit — text modifications
# ---------------------------------------------------------------------------


@dataclass
class TextEdit:
    """A single text replacement in a document.

    Used by formatting (replace the whole file) and rename (replace each
    occurrence). ``new_text`` replaces the content at ``range``. If
    ``new_text`` is empty, the range is deleted.
    """

    range: Range
    new_text: str


@dataclass
class WorkspaceEdit:
    """Groups TextEdits across potentially multiple files.

    For rename operations that affect a single file, ``changes`` will have
    one key. For multi-file projects, a rename may produce edits across
    many files.
    """

    changes: dict[str, list[TextEdit]] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# HoverResult — content for the hover popup
# ---------------------------------------------------------------------------


@dataclass
class HoverResult:
    """The content to show in the hover popup.

    ``contents`` is Markdown text. VS Code renders it with syntax
    highlighting, bold/italic, code blocks, etc. ``range`` is optional --
    if set, it highlights the symbol in the editor when the hover popup
    is shown.
    """

    contents: str  # Markdown
    range: Range | None = None


# ---------------------------------------------------------------------------
# CompletionItemKind — icon classification for autocomplete
# ---------------------------------------------------------------------------


class CompletionItemKind(IntEnum):
    """Classifies completion items so the editor can show the right icon.

    Each value corresponds to a different icon in the autocomplete dropdown:
    function icon, variable icon, keyword icon, etc.
    """

    TEXT = 1
    METHOD = 2
    FUNCTION = 3
    CONSTRUCTOR = 4
    FIELD = 5
    VARIABLE = 6
    CLASS = 7
    INTERFACE = 8
    MODULE = 9
    PROPERTY = 10
    UNIT = 11
    VALUE = 12
    ENUM = 13
    KEYWORD = 14
    SNIPPET = 15
    COLOR = 16
    FILE = 17
    REFERENCE = 18
    FOLDER = 19
    ENUM_MEMBER = 20
    CONSTANT = 21
    STRUCT = 22
    EVENT = 23
    OPERATOR = 24
    TYPE_PARAMETER = 25


@dataclass
class CompletionItem:
    """A single autocomplete suggestion.

    When the user triggers autocomplete (e.g., by pressing Ctrl+Space or
    typing after a dot), the editor shows a dropdown list of CompletionItems.
    """

    label: str
    kind: CompletionItemKind | None = None
    detail: str = ""
    documentation: str = ""
    insert_text: str = ""
    insert_text_format: int = 0  # 1=plain, 2=snippet


# ---------------------------------------------------------------------------
# SemanticToken — semantic highlighting data
# ---------------------------------------------------------------------------


@dataclass
class SemanticToken:
    """One token's contribution to the semantic highlighting pass.

    Semantic tokens are the "second pass" of syntax highlighting. The
    editor's grammar-based highlighter (TextMate/tmLanguage) does a fast
    regex pass first. Semantic tokens layer on top with accurate,
    context-aware type information.

    ``line`` and ``character`` are 0-based. ``token_type`` and ``modifiers``
    reference entries in the legend returned by ``semantic_token_legend()``
    (see capabilities.py).
    """

    line: int  # 0-based
    character: int  # 0-based, UTF-16 code units
    length: int  # in UTF-16 code units
    token_type: str  # must match an entry in the legend's token_types
    modifiers: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# SymbolKind — classification for the document outline panel
# ---------------------------------------------------------------------------


class SymbolKind(IntEnum):
    """Classifies document symbols for the outline panel.

    These match the LSP integer codes (1-based).
    """

    FILE = 1
    MODULE = 2
    NAMESPACE = 3
    PACKAGE = 4
    CLASS = 5
    METHOD = 6
    PROPERTY = 7
    FIELD = 8
    CONSTRUCTOR = 9
    ENUM = 10
    INTERFACE = 11
    FUNCTION = 12
    VARIABLE = 13
    CONSTANT = 14
    STRING = 15
    NUMBER = 16
    BOOLEAN = 17
    ARRAY = 18
    OBJECT = 19
    KEY = 20
    NULL = 21
    ENUM_MEMBER = 22
    STRUCT = 23
    EVENT = 24
    OPERATOR = 25
    TYPE_PARAMETER = 26


@dataclass
class DocumentSymbol:
    """One entry in the document outline panel.

    The outline shows a tree of named symbols (functions, classes, variables).
    ``children`` allows nesting: a class symbol can have method symbols as
    children.

    ``range`` covers the entire symbol (including its body).
    ``selection_range`` is the smaller range of just the symbol's name
    (used to highlight the name when the user clicks the outline entry).
    """

    name: str
    kind: SymbolKind
    range: Range
    selection_range: Range
    children: list[DocumentSymbol] = field(default_factory=list)


# ---------------------------------------------------------------------------
# FoldingRange — collapsible regions
# ---------------------------------------------------------------------------


@dataclass
class FoldingRange:
    """A collapsible region of the document.

    The editor shows a collapse arrow in the gutter next to ``start_line``.
    When collapsed, lines ``start_line+1`` through ``end_line`` are hidden.
    ``kind`` is one of ``"region"``, ``"imports"``, or ``"comment"``.
    """

    start_line: int  # 0-based
    end_line: int  # 0-based
    kind: str = ""


# ---------------------------------------------------------------------------
# Signature help — function call tooltips
# ---------------------------------------------------------------------------


@dataclass
class ParameterInformation:
    """One parameter in a function signature."""

    label: str
    documentation: str = ""


@dataclass
class SignatureInformation:
    """One function overload's full signature."""

    label: str
    documentation: str = ""
    parameters: list[ParameterInformation] = field(default_factory=list)


@dataclass
class SignatureHelpResult:
    """Shown as a tooltip when the user is typing a function call.

    It shows the function signature with the current parameter highlighted.
    ``active_signature`` indexes into ``signatures``. ``active_parameter``
    indexes into that signature's ``parameters``.

    Example: typing ``foo(a, |`` (cursor after the comma) would show
    ``signatures[0]`` with ``active_parameter=1`` (highlighting the second
    parameter).
    """

    signatures: list[SignatureInformation] = field(default_factory=list)
    active_signature: int = 0
    active_parameter: int = 0
