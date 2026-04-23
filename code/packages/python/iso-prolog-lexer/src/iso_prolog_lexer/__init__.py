"""ISO/Core Prolog lexer."""

from iso_prolog_lexer.tokenizer import (
    ISO_PROLOG_TOKENS_PATH,
    create_iso_prolog_lexer,
    tokenize_iso_prolog,
)

__all__ = [
    "__version__",
    "ISO_PROLOG_TOKENS_PATH",
    "create_iso_prolog_lexer",
    "tokenize_iso_prolog",
]

__version__ = "0.1.0"
