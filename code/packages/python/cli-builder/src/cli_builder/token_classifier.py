"""Token classifier for CLI argument tokens.

=== What is token classification? ===

Before parsing can happen, each raw argv string must be classified into a
typed token event. The classifier answers questions like:

- Is "--verbose" a long boolean flag or a long value-taking flag?
- Is "-lah" three stacked boolean flags or something else?
- Is "-ffile.txt" the flag -f with inline value "file.txt", or what?
- Is "-classpath" a single-dash-long flag or an attempt to stack c+l+a+s+...?

The classifier implements the longest-match-first disambiguation rules from
spec §5.2. "Longest match first" means: before trying to decompose a token
into stacked short flags, we first check if the entire suffix matches a known
``single_dash_long`` flag. This prevents ``-classpath`` from being
misinterpreted as ``-c -l -a ...``.

=== Token type reference ===

| Type                | Example               | Description                          |
|---------------------|-----------------------|--------------------------------------|
| end_of_flags        | --                    | Everything after this is positional  |
| long_flag           | --verbose             | Long flag, value (if any) follows    |
| long_flag_with_value| --output=file.txt     | Long flag + inline value             |
| single_dash_long    | -classpath            | Matched single_dash_long field       |
| short_flag          | -l                    | Single short flag character          |
| short_flag_with_value| -ffile.txt           | Short non-boolean + inline value     |
| stacked_flags       | -lah                  | Multiple boolean short flags         |
| positional          | hello, -              | Positional argument or bare dash     |
| unknown_flag        | --typo                | No match found                       |

=== Why not a DFA? ===

The spec calls this a "token classification DFA", and conceptually it is one:
each character transition either confirms or rules out possible interpretations.
In practice, we implement it as a sequence of explicit checks rather than a
table-driven DFA, because:
1. The token alphabet is variable (depends on the active flag set).
2. The "stacked flags" case requires walking characters in a loop.
3. Explicit code is easier to read and debug than a generated DFA table.

The ModalStateMachine in ``parser.py`` drives the *overall* parse mode; this
classifier handles *per-token* structure.
"""

from __future__ import annotations

from typing import Any


class TokenClassifier:
    """Classifies a single argv token into a typed event dict.

    The classifier must be initialized with the *active flag set* for the
    current command scope. This is because which tokens count as valid flags
    depends entirely on what flags are in scope.

    Usage::

        flags = [{"id": "long-listing", "short": "l", "type": "boolean"}, ...]
        classifier = TokenClassifier(flags)
        event = classifier.classify("--verbose")
        # event == {"type": "long_flag", "name": "verbose", "flag_def": {...}}

    The returned dict always has a ``"type"`` key. Additional keys depend on
    the type:
    - ``long_flag``: ``name``, ``flag_def``
    - ``long_flag_with_value``: ``name``, ``value``, ``flag_def``
    - ``single_dash_long``: ``name``, ``flag_def``
    - ``short_flag``: ``char``, ``flag_def``
    - ``short_flag_with_value``: ``char``, ``value``, ``flag_def``
    - ``stacked_flags``: ``chars``, ``flag_defs``, ``trailing_value``
    - ``positional``: ``value``
    - ``end_of_flags``: (no extra keys)
    - ``unknown_flag``: ``token``
    """

    def __init__(self, active_flags: list[dict[str, Any]]) -> None:
        """Initialize the classifier with the active flag set.

        Args:
            active_flags: List of flag definition dicts in scope.
                Each flag should have fields per spec §2.2.
        """
        self._flags = active_flags

        # Build lookup indices for fast access.
        # Index by short character, long name, and single_dash_long name.
        self._by_short: dict[str, dict[str, Any]] = {}
        self._by_long: dict[str, dict[str, Any]] = {}
        self._by_sdl: dict[str, dict[str, Any]] = {}  # single_dash_long

        for flag in active_flags:
            if flag.get("short"):
                self._by_short[flag["short"]] = flag
            if flag.get("long"):
                self._by_long[flag["long"]] = flag
            if flag.get("single_dash_long"):
                self._by_sdl[flag["single_dash_long"]] = flag

    def classify(self, token: str) -> dict[str, Any]:
        """Classify a single argv token into a typed event dict.

        Applies longest-match-first disambiguation. Rules applied in order:

        1. Exactly ``"--"`` → ``end_of_flags``
        2. Starts with ``"--"`` → long flag family
        3. Exactly ``"-"`` → positional (bare dash = stdin/stdout convention)
        4. Starts with ``"-"`` followed by 2+ chars → check single_dash_long,
           then short flag + possible stacking
        5. Starts with ``"-"`` followed by 1 char → short flag
        6. Otherwise → positional

        Args:
            token: A single element from argv.

        Returns:
            A dict with at least a ``"type"`` key.
        """
        # Rule 1: End-of-flags sentinel
        if token == "--":
            return {"type": "end_of_flags"}

        # Rule 2: Long flags (start with --)
        if token.startswith("--"):
            return self._classify_long(token)

        # Rule 3: Bare dash is always positional
        if token == "-":
            return {"type": "positional", "value": "-"}

        # Rule 4 & 5: Single-dash flags
        if token.startswith("-"):
            return self._classify_single_dash(token)

        # Rule 6: Positional argument
        return {"type": "positional", "value": token}

    # =========================================================================
    # Private helpers
    # =========================================================================

    def _classify_long(self, token: str) -> dict[str, Any]:
        """Classify a ``--``-prefixed token.

        Handles two sub-cases:
        - ``--name`` → LONG_FLAG (value follows as next token if non-boolean)
        - ``--name=value`` → LONG_FLAG_WITH_VALUE (value inline)

        Args:
            token: A token starting with ``"--"``.

        Returns:
            A typed event dict.
        """
        # Strip the leading --
        body = token[2:]

        # Check for inline value: --name=value
        if "=" in body:
            name, _, value = body.partition("=")
            flag_def = self._by_long.get(name)
            if flag_def is None:
                return {"type": "unknown_flag", "token": token}
            return {
                "type": "long_flag_with_value",
                "name": name,
                "value": value,
                "flag_def": flag_def,
            }

        # No inline value: --name
        flag_def = self._by_long.get(body)
        if flag_def is None:
            return {"type": "unknown_flag", "token": token}
        return {"type": "long_flag", "name": body, "flag_def": flag_def}

    def _classify_single_dash(self, token: str) -> dict[str, Any]:
        """Classify a single-dash token (not -- and not bare -).

        Disambiguation order per spec §5.2:

        Step 1 — Single-dash-long match (longest match first):
          Check if the entire suffix matches a ``single_dash_long`` flag name.

        Step 2 — Single-character short flag:
          Check if the first character after ``-`` is a known short flag.
          - Boolean: just the flag.
          - Non-boolean: remainder is inline value (or value follows next).

        Step 3 — Stacked short flags:
          Walk each character; all but the last must be boolean.

        Step 4 — Unknown flag.

        Args:
            token: A token starting with ``"-"`` but not ``"--"``.

        Returns:
            A typed event dict.
        """
        suffix = token[1:]  # everything after the leading -

        # --- Step 1: Single-dash-long match ---
        #
        # We check the FULL suffix, not just a prefix, because single_dash_long
        # names are complete identifiers. "-classpath" is a single flag, not
        # the start of "-c -l -a ...".
        if suffix in self._by_sdl:
            flag_def = self._by_sdl[suffix]
            return {
                "type": "single_dash_long",
                "name": suffix,
                "flag_def": flag_def,
            }

        # If suffix is a single character, just check short flags.
        if len(suffix) == 1:
            return self._classify_short_single(suffix, token)

        # --- Step 2: Check if first character is a known short flag ---
        first_char = suffix[0]
        flag_def = self._by_short.get(first_char)

        if flag_def is not None:
            is_boolean = flag_def.get("type") == "boolean"
            remainder = suffix[1:]  # characters after the flag char

            if is_boolean and remainder:
                # The first char is a known boolean flag, but there's more.
                # Try to parse the remainder as stacked flags.
                return self._classify_stacked(suffix, token)

            if not is_boolean:
                # Non-boolean flag: the remainder (if any) is the inline value.
                if remainder:
                    return {
                        "type": "short_flag_with_value",
                        "char": first_char,
                        "value": remainder,
                        "flag_def": flag_def,
                    }
                else:
                    # Value will be the next token.
                    return {
                        "type": "short_flag",
                        "char": first_char,
                        "flag_def": flag_def,
                    }

        # --- Step 3: Try full stacking ---
        return self._classify_stacked(suffix, token)

    def _classify_short_single(
        self,
        char: str,
        original_token: str,
    ) -> dict[str, Any]:
        """Classify a single-character short flag token (e.g., ``-l``).

        Args:
            char: The single character after ``-``.
            original_token: The full original token for error messages.

        Returns:
            A typed event dict.
        """
        flag_def = self._by_short.get(char)
        if flag_def is None:
            return {"type": "unknown_flag", "token": original_token}
        return {"type": "short_flag", "char": char, "flag_def": flag_def}

    def _classify_stacked(
        self,
        suffix: str,
        original_token: str,
    ) -> dict[str, Any]:
        """Try to classify a multi-character suffix as stacked short flags.

        Stacking rules (spec §5.2, Rule 3):
        - Walk each character in the suffix.
        - If the character matches a BOOLEAN short flag: record it and continue.
        - If the character matches a NON-BOOLEAN short flag: the remaining
          characters are its inline value. Stop.
        - If no match: emit ``unknown_flag``.

        All characters EXCEPT possibly the last must be boolean flags.

        Args:
            suffix: Characters after the leading ``-``.
            original_token: The full original token for error messages.

        Returns:
            A typed event dict. Either ``stacked_flags`` or ``unknown_flag``.
        """
        chars: list[str] = []
        flag_defs: list[dict[str, Any]] = []
        trailing_value: str | None = None

        for i, char in enumerate(suffix):
            flag_def = self._by_short.get(char)
            if flag_def is None:
                # Unknown character in stack — this is an error.
                return {"type": "unknown_flag", "token": original_token}

            is_boolean = flag_def.get("type") == "boolean"
            is_last = i == len(suffix) - 1

            if is_boolean:
                chars.append(char)
                flag_defs.append(flag_def)
            elif not is_boolean and is_last:
                # Non-boolean as the last character, no inline value.
                chars.append(char)
                flag_defs.append(flag_def)
            elif not is_boolean and not is_last:
                # Non-boolean not at end: remaining chars are the inline value.
                chars.append(char)
                flag_defs.append(flag_def)
                trailing_value = suffix[i + 1 :]
                break

        if len(chars) == 1 and trailing_value is None:
            # Single flag, not really "stacked"
            if flag_defs[0].get("type") == "boolean":
                return {
                    "type": "short_flag",
                    "char": chars[0],
                    "flag_def": flag_defs[0],
                }

        return {
            "type": "stacked_flags",
            "chars": chars,
            "flag_defs": flag_defs,
            "trailing_value": trailing_value,
        }
