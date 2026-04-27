"""Capabilities, SemanticTokenLegend, and encode_semantic_tokens.

What Are Capabilities?
-----------------------

During the LSP initialize handshake, the server sends back a "capabilities"
object telling the editor which LSP features it supports. The editor uses
this to decide which requests to send. If a capability is absent, the editor
won't send the corresponding requests -- so no "Go to Definition" button
appears unless ``definitionProvider`` is true.

Building capabilities dynamically (based on the bridge's protocol
implementations) means the server is always honest about what it can do.

Semantic Token Legend
-----------------------

Semantic tokens use a compact binary encoding. Instead of sending
``{"type":"keyword"}`` per token, LSP sends an integer index into a legend.
The legend must be declared in the capabilities so the editor knows what
each index means.

Example legend::

    tokenTypes:     ["namespace","type","class","enum",...,"keyword","string","number",...]
    tokenModifiers: ["declaration","definition","readonly","static",...]

A token with type index 14 and modifiers bitmask 0b0001 means:
    type = tokenTypes[14] = "keyword", modifiers = [tokenModifiers[0]] = ["declaration"]
"""

from __future__ import annotations

from typing import Any

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
from ls00.types import SemanticToken


def build_capabilities(bridge: LanguageBridge) -> dict[str, Any]:
    """Inspect the bridge at runtime and return the LSP capabilities object.

    Uses ``isinstance()`` with ``@runtime_checkable`` Protocol classes to
    check which optional provider protocols the bridge implements. Only
    advertises capabilities for features the bridge actually supports.

    This mirrors Go's type assertion pattern: ``if _, ok := bridge.(HoverProvider); ok``
    """
    # textDocumentSync=2 means "incremental": the editor sends only changed
    # ranges, not the full file, on every keystroke. We always advertise this.
    caps: dict[str, Any] = {
        "textDocumentSync": 2,
    }

    # Check each optional provider via isinstance().
    # Python's @runtime_checkable Protocol + isinstance() is equivalent to
    # Go's type assertion: bridge.(HoverProvider)

    if isinstance(bridge, HoverProvider):
        caps["hoverProvider"] = True

    if isinstance(bridge, DefinitionProvider):
        caps["definitionProvider"] = True

    if isinstance(bridge, ReferencesProvider):
        caps["referencesProvider"] = True

    if isinstance(bridge, CompletionProvider):
        # completionProvider is an object, not a boolean, because it can
        # include triggerCharacters.
        caps["completionProvider"] = {
            "triggerCharacters": [" ", "."],
        }

    if isinstance(bridge, RenameProvider):
        caps["renameProvider"] = True

    if isinstance(bridge, DocumentSymbolsProvider):
        caps["documentSymbolProvider"] = True

    if isinstance(bridge, FoldingRangesProvider):
        caps["foldingRangeProvider"] = True

    if isinstance(bridge, SignatureHelpProvider):
        caps["signatureHelpProvider"] = {
            "triggerCharacters": ["(", ","],
        }

    if isinstance(bridge, FormatProvider):
        caps["documentFormattingProvider"] = True

    if isinstance(bridge, SemanticTokensProvider):
        caps["semanticTokensProvider"] = {
            "legend": semantic_token_legend(),
            "full": True,
        }

    return caps


# ---------------------------------------------------------------------------
# Semantic Token Legend
# ---------------------------------------------------------------------------


def semantic_token_legend() -> dict[str, list[str]]:
    """Return the full legend for all supported semantic token types and modifiers.

    Why a Fixed Legend?
    ~~~~~~~~~~~~~~~~~~~~

    The legend is sent once in the capabilities response. Afterwards, each
    semantic token is encoded as an integer index into this legend rather
    than a string. This makes the per-token encoding much smaller.

    The ordering matters: index 0 in ``tokenTypes`` corresponds to
    ``"namespace"``, index 1 to ``"type"``, etc. These match the standard
    LSP token types.
    """
    return {
        # Standard LSP token types (in the order VS Code expects them).
        "tokenTypes": [
            "namespace",      # 0
            "type",           # 1
            "class",          # 2
            "enum",           # 3
            "interface",      # 4
            "struct",         # 5
            "typeParameter",  # 6
            "parameter",      # 7
            "variable",       # 8
            "property",       # 9
            "enumMember",     # 10
            "event",          # 11
            "function",       # 12
            "method",         # 13
            "macro",          # 14
            "keyword",        # 15
            "modifier",       # 16
            "comment",        # 17
            "string",         # 18
            "number",         # 19
            "regexp",         # 20
            "operator",       # 21
            "decorator",      # 22
        ],
        # Standard LSP token modifiers (bitmask flags).
        "tokenModifiers": [
            "declaration",    # bit 0
            "definition",     # bit 1
            "readonly",       # bit 2
            "static",         # bit 3
            "deprecated",     # bit 4
            "abstract",       # bit 5
            "async",          # bit 6
            "modification",   # bit 7
            "documentation",  # bit 8
            "defaultLibrary", # bit 9
        ],
    }


# ---------------------------------------------------------------------------
# Semantic Token Encoding
# ---------------------------------------------------------------------------


def _token_type_index(token_type: str) -> int:
    """Return the integer index for a semantic token type string.

    Returns -1 if the type is not in the legend (the caller should skip
    such tokens).
    """
    legend = semantic_token_legend()
    try:
        return legend["tokenTypes"].index(token_type)
    except ValueError:
        return -1


def _token_modifier_mask(modifiers: list[str]) -> int:
    """Return the bitmask for a list of modifier strings.

    The LSP semantic tokens encoding represents modifiers as a bitmask:
    - ``"declaration"`` -> bit 0 -> value 1
    - ``"definition"`` -> bit 1 -> value 2
    - both -> value 3 (bitwise OR)

    Unknown modifiers are silently ignored.
    """
    legend = semantic_token_legend()
    modifier_list = legend["tokenModifiers"]
    mask = 0
    for mod in modifiers:
        if mod in modifier_list:
            mask |= (1 << modifier_list.index(mod))
    return mask


def encode_semantic_tokens(tokens: list[SemanticToken]) -> list[int]:
    """Convert a list of SemanticToken values to the LSP compact integer encoding.

    The LSP Semantic Token Encoding
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    LSP encodes semantic tokens as a flat array of integers, grouped in
    5-tuples::

        [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]

    Where "delta" means: the difference from the PREVIOUS token's position.
    This delta encoding makes most values small (often 0 or 1), which
    compresses well and is efficient to parse.

    Example: three tokens on different lines::

        Token A: line=0, char=0, len=3, type="keyword",  modifiers=[]
        Token B: line=0, char=4, len=5, type="function", modifiers=["declaration"]
        Token C: line=1, char=0, len=8, type="variable", modifiers=[]

    Encoded as::

        [0, 0, 3, 15, 0,   # A: deltaLine=0, deltaChar=0 (first token)
         0, 4, 5, 12, 1,   # B: deltaLine=0, deltaChar=4 (same line)
         1, 0, 8,  8, 0]   # C: deltaLine=1, deltaChar=0 (next line)

    Note: when deltaLine > 0, deltaStartChar is relative to column 0 of the
    new line (i.e., absolute for that line). When deltaLine == 0, deltaStartChar
    is relative to the previous token's start character.
    """
    if not tokens:
        return []

    # Sort by (line, character) ascending. The delta encoding requires
    # tokens to be in document order.
    sorted_tokens = sorted(tokens, key=lambda t: (t.line, t.character))

    data: list[int] = []
    prev_line = 0
    prev_char = 0

    for tok in sorted_tokens:
        type_idx = _token_type_index(tok.token_type)
        if type_idx == -1:
            # Unknown token type -- skip it.
            continue

        delta_line = tok.line - prev_line
        if delta_line == 0:
            # Same line: character offset is relative to previous token.
            delta_char = tok.character - prev_char
        else:
            # Different line: character offset is absolute.
            delta_char = tok.character

        mod_mask = _token_modifier_mask(tok.modifiers)

        data.extend([delta_line, delta_char, tok.length, type_idx, mod_mask])

        prev_line = tok.line
        prev_char = tok.character

    return data
