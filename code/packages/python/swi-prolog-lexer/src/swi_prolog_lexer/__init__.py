"""SWI-Prolog lexer."""

from swi_prolog_lexer.tokenizer import (
    SWI_PROLOG_TOKENS_PATH,
    create_swi_prolog_lexer,
    tokenize_swi_prolog,
)

__all__ = [
    "__version__",
    "SWI_PROLOG_TOKENS_PATH",
    "create_swi_prolog_lexer",
    "tokenize_swi_prolog",
]

__version__ = "0.1.0"
