from haskell_lexer import create_haskell_lexer, tokenize_haskell


def test_default_version_uses_haskell2010() -> None:
    lexer = create_haskell_lexer("x")
    tokens = lexer.tokenize()
    assert tokens[0].type_name == "NAME"


def test_layout_tokens_are_emitted() -> None:
    tokens = tokenize_haskell("let\n  x = y\nin x")
    types = [token.type_name for token in tokens]
    assert "VIRTUAL_LBRACE" in types
    assert "VIRTUAL_RBRACE" in types


def test_versioned_grammar_is_available() -> None:
    tokens = tokenize_haskell("x", "98")
    assert tokens[0].type_name == "NAME"
