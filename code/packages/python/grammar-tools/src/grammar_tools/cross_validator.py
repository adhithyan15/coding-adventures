"""
cross_validator.py — Cross-validates a .tokens file and a .grammar file.

The whole point of having two separate grammar files is that they reference
each other: the .grammar file uses UPPERCASE names to refer to tokens
defined in the .tokens file. This module checks that the two files are
consistent.

Why cross-validate?
-------------------

Each file can be valid on its own but broken when used together:

- A grammar might reference ``SEMICOLON``, but the .tokens file only
  defines ``SEMI``. Each file is fine individually, but the pair is broken.
- A .tokens file might define ``TILDE = "~"`` that no grammar rule ever
  uses. This is not an error — it might be intentional — but it is worth
  warning about because unused tokens add complexity without value.

This is analogous to how a C compiler checks that every function you call
is actually declared (and vice versa, warns about unused functions).

What we check
-------------

1. **Missing token references**: Every UPPERCASE name in the grammar must
   correspond to a token definition. If not, the generated parser will try
   to match a token type that the lexer never produces.

2. **Unused tokens**: Every token defined in the .tokens file should ideally
   be referenced somewhere in the grammar. Unused tokens suggest either a
   typo or leftover cruft. We report these as warnings, not errors.

3. **Usage report**: We list which tokens and rules are actually used, which
   helps users understand their grammar.
"""

from __future__ import annotations

from grammar_tools.parser_grammar import ParserGrammar
from grammar_tools.token_grammar import TokenGrammar


def cross_validate(
    token_grammar: TokenGrammar,
    parser_grammar: ParserGrammar,
) -> list[str]:
    """Cross-validate a token grammar and a parser grammar.

    Checks that every UPPERCASE name referenced in the parser grammar
    exists in the token grammar, and warns about tokens that are defined
    but never used.

    Args:
        token_grammar: A parsed .tokens file.
        parser_grammar: A parsed .grammar file.

    Returns:
        A list of error/warning strings. Errors describe broken references;
        warnings describe unused definitions. An empty list means the two
        grammars are fully consistent.
    """
    issues: list[str] = []

    defined_tokens = token_grammar.token_names()
    referenced_tokens = parser_grammar.token_references()

    # --- Missing token references (errors) ---
    # Every UPPERCASE name used in the grammar must exist in the tokens file.
    for ref in sorted(referenced_tokens):
        if ref not in defined_tokens:
            issues.append(
                f"Error: Grammar references token '{ref}' which is not "
                f"defined in the tokens file"
            )

    # --- Unused tokens (warnings) ---
    # Every token defined in the .tokens file should ideally be used
    # somewhere in the grammar.
    for defn in token_grammar.definitions:
        if defn.name not in referenced_tokens:
            issues.append(
                f"Warning: Token '{defn.name}' (line {defn.line_number}) "
                f"is defined but never used in the grammar"
            )

    return issues
