"""Tests for capability building and semantic token legend.

These tests verify that:
1. Minimal bridges only advertise textDocumentSync
2. Full bridges advertise all implemented capabilities
3. The semantic token legend contains required token types
"""

from __future__ import annotations

from typing import Any

from ls00 import (
    CompletionItem,
    Diagnostic,
    DocumentSymbol,
    FoldingRange,
    HoverResult,
    Location,
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
    semantic_token_legend,
)


# ---------------------------------------------------------------------------
# Test bridges
# ---------------------------------------------------------------------------


class MinimalBridge:
    """Implements ONLY the required LanguageBridge protocol."""

    def tokenize(self, source: str) -> list[Token]:
        return []

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        return source, []


class MockBridge:
    """Implements LanguageBridge + HoverProvider + DocumentSymbolsProvider."""

    def tokenize(self, source: str) -> list[Token]:
        return []

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        return source, []

    def hover(self, ast: Any, pos: Position) -> HoverResult | None:
        return HoverResult(contents="test hover")

    def document_symbols(self, ast: Any) -> list[DocumentSymbol]:
        return [DocumentSymbol(
            name="main",
            kind=SymbolKind.FUNCTION,
            range=Range(Position(0, 0), Position(10, 1)),
            selection_range=Range(Position(0, 9), Position(0, 13)),
        )]


class FullMockBridge:
    """Implements all optional provider interfaces."""

    def tokenize(self, source: str) -> list[Token]:
        return []

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        return source, []

    def hover(self, ast: Any, pos: Position) -> HoverResult | None:
        return HoverResult(contents="test hover")

    def definition(self, ast: Any, pos: Position, uri: str) -> Location | None:
        return Location(uri=uri, range=Range(start=pos, end=pos))

    def references(
        self, ast: Any, pos: Position, uri: str, include_decl: bool
    ) -> list[Location]:
        return [Location(uri=uri, range=Range(start=pos, end=pos))]

    def completion(self, ast: Any, pos: Position) -> list[CompletionItem]:
        return [CompletionItem(label="foo")]

    def rename(
        self, ast: Any, pos: Position, new_name: str
    ) -> WorkspaceEdit | None:
        return WorkspaceEdit(changes={
            "file:///test.txt": [TextEdit(
                range=Range(start=pos, end=pos),
                new_text=new_name,
            )],
        })

    def semantic_tokens(
        self, source: str, tokens: list[Token]
    ) -> list[SemanticToken]:
        return []

    def document_symbols(self, ast: Any) -> list[DocumentSymbol]:
        return []

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
# Tests
# ---------------------------------------------------------------------------


class TestBuildCapabilitiesMinimal:
    """Minimal bridge should only advertise textDocumentSync."""

    def test_text_document_sync_always_present(self) -> None:
        bridge = MinimalBridge()
        caps = build_capabilities(bridge)
        assert caps["textDocumentSync"] == 2

    def test_optional_capabilities_absent(self) -> None:
        """Optional capabilities should NOT be present for a minimal bridge."""
        bridge = MinimalBridge()
        caps = build_capabilities(bridge)

        optional = [
            "hoverProvider",
            "definitionProvider",
            "referencesProvider",
            "completionProvider",
            "renameProvider",
            "documentSymbolProvider",
            "foldingRangeProvider",
            "signatureHelpProvider",
            "documentFormattingProvider",
            "semanticTokensProvider",
        ]
        for cap in optional:
            assert cap not in caps, f"minimal bridge should not advertise {cap}"


class TestBuildCapabilitiesFullBridge:
    """Full bridge should advertise all implemented capabilities."""

    def test_hover_provider_present(self) -> None:
        bridge = MockBridge()
        caps = build_capabilities(bridge)
        assert caps.get("hoverProvider") is True

    def test_document_symbol_provider_present(self) -> None:
        bridge = MockBridge()
        caps = build_capabilities(bridge)
        assert caps.get("documentSymbolProvider") is True

    def test_semantic_tokens_not_present_for_mock(self) -> None:
        """MockBridge doesn't implement SemanticTokensProvider."""
        bridge = MockBridge()
        caps = build_capabilities(bridge)
        assert "semanticTokensProvider" not in caps

    def test_semantic_tokens_present_for_full(self) -> None:
        """FullMockBridge implements SemanticTokensProvider."""
        bridge = FullMockBridge()
        caps = build_capabilities(bridge)
        assert "semanticTokensProvider" in caps
        st = caps["semanticTokensProvider"]
        assert st["full"] is True

    def test_all_capabilities_for_full_bridge(self) -> None:
        """FullMockBridge should advertise all capabilities."""
        bridge = FullMockBridge()
        caps = build_capabilities(bridge)

        expected = [
            "textDocumentSync",
            "hoverProvider",
            "definitionProvider",
            "referencesProvider",
            "completionProvider",
            "renameProvider",
            "documentSymbolProvider",
            "foldingRangeProvider",
            "signatureHelpProvider",
            "documentFormattingProvider",
            "semanticTokensProvider",
        ]
        for cap in expected:
            assert cap in caps, f"expected capability {cap} for full bridge"


class TestSemanticTokenLegend:
    """Verify the legend contains required token types."""

    def test_non_empty(self) -> None:
        legend = semantic_token_legend()
        assert len(legend["tokenTypes"]) > 0
        assert len(legend["tokenModifiers"]) > 0

    def test_required_types_present(self) -> None:
        legend = semantic_token_legend()
        required = ["keyword", "string", "number", "variable", "function"]
        for rt in required:
            assert rt in legend["tokenTypes"], f"legend missing required type {rt}"
