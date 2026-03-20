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

    Special handling for extended features:

    - **Indentation mode**: When ``token_grammar.mode == "indentation"``,
      the tokens ``INDENT``, ``DEDENT``, and ``NEWLINE`` are implicitly
      available (synthesized by the lexer), even if not defined in the
      .tokens file. The grammar can reference them freely.

    - **Aliases**: When a token definition has an alias (e.g.,
      ``STRING_DQ -> STRING``), the grammar may reference either the
      alias (``STRING``) or the original name (``STRING_DQ``). The
      alias is the preferred form (it is what the lexer emits).

    Args:
        token_grammar: A parsed .tokens file.
        parser_grammar: A parsed .grammar file.

    Returns:
        A list of error/warning strings. Errors describe broken references;
        warnings describe unused definitions. An empty list means the two
        grammars are fully consistent.
    """
    issues: list[str] = []

    # Build the set of all token names the parser can reference.
    # This includes both definition names and their aliases.
    defined_tokens = token_grammar.token_names()

    # When indentation mode is active, the lexer synthesizes INDENT,
    # DEDENT, and NEWLINE tokens. These are not in the .tokens file but
    # are valid to reference in the grammar.
    if token_grammar.mode == "indentation":
        defined_tokens |= {"INDENT", "DEDENT", "NEWLINE"}

    # The EOF token is always implicitly available.
    defined_tokens.add("EOF")

    referenced_tokens = parser_grammar.token_references()

    # --- Missing token references (errors) ---
    for ref in sorted(referenced_tokens):
        if ref not in defined_tokens:
            issues.append(
                f"Error: Grammar references token '{ref}' which is not "
                f"defined in the tokens file"
            )

    # --- Unused tokens (warnings) ---
    # Build the set of "effectively referenced" names, accounting for
    # aliases. If the grammar references STRING and a definition has
    # alias=STRING, that definition counts as used.
    alias_to_names: dict[str, list[str]] = {}
    for defn in token_grammar.definitions:
        if defn.alias:
            alias_to_names.setdefault(defn.alias, []).append(defn.name)

    for defn in token_grammar.definitions:
        # A definition is "used" if:
        # 1. Its name is directly referenced, OR
        # 2. Its alias is referenced
        is_used = defn.name in referenced_tokens
        if defn.alias and defn.alias in referenced_tokens:
            is_used = True

        if not is_used:
            issues.append(
                f"Warning: Token '{defn.name}' (line {defn.line_number}) "
                f"is defined but never used in the grammar"
            )

    return issues
