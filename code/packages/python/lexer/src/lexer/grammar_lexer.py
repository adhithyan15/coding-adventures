"""
Grammar-Driven Lexer â€” Tokenization from .tokens Files
=======================================================

The hand-written ``Lexer`` in ``tokenizer.py`` hardcodes which characters map
to which tokens. That works well for a single language, but what if you want
to tokenize Python *and* Ruby *and* JavaScript with the same codebase? You
would need to rewrite the character-dispatching logic for each language.

This module takes a different approach, inspired by classic tools like
`Lex <https://en.wikipedia.org/wiki/Lex_(software)>`_ and
`Flex <https://en.wikipedia.org/wiki/Flex_(lexical_analyser_generator)>`_.
Instead of hardcoding patterns in Python, we read token definitions from a
``.tokens`` file (parsed by the ``grammar_tools`` package) and use those
definitions to drive tokenization at runtime.

How It Works â€” The Big Picture
------------------------------

A ``.tokens`` file looks like this::

    NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
    NUMBER = /[0-9]+/
    PLUS   = "+"
    MINUS  = "-"

    keywords:
      if
      else

Each line defines a token: a name and a pattern. The pattern is either a
regex (``/.../.``) or a literal string (``"..."``). The ``grammar_tools``
package parses this file into a ``TokenGrammar`` object â€” a structured list
of ``TokenDefinition`` objects plus a keyword list.

The ``GrammarLexer`` takes that ``TokenGrammar`` and does the following:

1. **Compile** each token definition into a Python ``re.Pattern`` object.
   Literal patterns are escaped so that characters like ``+`` and ``*`` are
   treated as literal characters, not regex operators.

2. **At each position** in the source code, try each compiled pattern in
   order (first match wins). This is the "priority" mechanism â€” if two
   patterns could match at the same position, the one that appears first
   in the ``.tokens`` file wins.

3. **Emit a Token** with the matched type and value, using the same
   ``Token`` dataclass as the hand-written lexer. The token type is either
   a ``TokenType`` enum member (for backward compatibility with simple
   grammars) or a string (for extended grammars with custom token names
   like Starlark's 45+ token types).

Extended Format Support
-----------------------

Beyond simple token definitions, this lexer handles four extensions from
the ``.tokens`` format:

1. **Skip patterns** (``skip:`` section): Patterns that are matched and
   consumed without producing tokens. Used for comments and inline
   whitespace. These replace the hardcoded whitespace-skipping logic
   when present.

2. **Type aliases** (``-> TYPE`` suffix): When a token definition has an
   alias (e.g., ``STRING_DQ = /.../ -> STRING``), the emitted token uses
   the alias as its type. This lets the grammar reference ``STRING``
   while the tokens file handles the lexical complexity.

3. **Reserved keywords** (``reserved:`` section): Identifiers that are
   syntax errors. When the lexer matches a NAME and its value is in the
   reserved set, it raises a ``LexerError`` immediately.

4. **Indentation mode** (``mode: indentation``): Python-style significant
   whitespace. The lexer maintains an indentation stack and emits synthetic
   INDENT, DEDENT, and NEWLINE tokens. Brackets suppress indentation
   processing (implicit line joining).

Why Two Lexers?
---------------

The hand-written ``Lexer`` is the **reference implementation** â€” clear,
well-documented, and easy to step through in a debugger. The
``GrammarLexer`` is the **grammar-driven alternative** â€” flexible,
language-agnostic, and data-driven. Having both lets us:

- Verify correctness by comparing their outputs
- Demonstrate two fundamentally different approaches to the same problem
- Use the hand-written lexer for teaching and the grammar-driven one
  for production grammar work
"""

from __future__ import annotations

import re
from collections.abc import Callable

from grammar_tools import TokenGrammar

from lexer.tokenizer import TOKEN_CONTEXT_KEYWORD, LexerError, Token, TokenType

# ---------------------------------------------------------------------------
# Lexer Context â€” Callback Interface for Group Transitions
# ---------------------------------------------------------------------------


class LexerContext:
    """Interface that on-token callbacks use to control the lexer.

    When a callback is registered via ``GrammarLexer.set_on_token()``, it
    receives a ``LexerContext`` on every token match. The context provides
    controlled access to the group stack, token emission, and skip control.

    Methods that modify state (push/pop/emit/suppress) take effect after
    the callback returns â€” they do not interrupt the current match.

    Example â€” XML lexer callback::

        def xml_hook(token, ctx):
            if token.type == "OPEN_TAG_START":
                ctx.push_group("tag")
            elif token.type in ("TAG_CLOSE", "SELF_CLOSE"):
                ctx.pop_group()
    """

    def __init__(
        self,
        lexer: GrammarLexer,
        source: str,
        pos_after_token: int,
        previous_token: Token | None = None,
        current_token_line: int = 1,
    ) -> None:
        self._lexer = lexer
        self._source = source
        self._pos_after = pos_after_token
        self._suppressed = False
        self._emitted: list[Token] = []
        self._group_actions: list[tuple[str, str]] = []
        self._skip_enabled: bool | None = None  # None = no change
        self._previous_token = previous_token
        self._current_token_line = current_token_line

    def push_group(self, group_name: str) -> None:
        """Push a pattern group onto the stack.

        The pushed group becomes active for the next token match.
        Raises ValueError if the group name is not defined in the grammar.
        """
        if group_name not in self._lexer._group_patterns:
            raise ValueError(
                f"Unknown pattern group: {group_name!r}. "
                f"Available groups: {sorted(self._lexer._group_patterns.keys())}"
            )
        self._group_actions.append(("push", group_name))

    def pop_group(self) -> None:
        """Pop the current group from the stack.

        If only the default group remains, this is a no-op. The default
        group is the floor and cannot be popped.
        """
        self._group_actions.append(("pop", ""))

    def active_group(self) -> str:
        """Return the name of the currently active group."""
        return self._lexer._group_stack[-1]

    def group_stack_depth(self) -> int:
        """Return the depth of the group stack (always >= 1)."""
        return len(self._lexer._group_stack)

    def emit(self, token: Token) -> None:
        """Inject a synthetic token after the current one.

        Emitted tokens do NOT trigger the callback (prevents infinite
        loops). Multiple emit() calls produce tokens in call order.
        """
        self._emitted.append(token)

    def suppress(self) -> None:
        """Suppress the current token â€” do not include it in output."""
        self._suppressed = True

    def peek(self, offset: int = 1) -> str:
        """Peek at a source character past the current token.

        Args:
            offset: Number of characters ahead (1 = immediately after token).

        Returns:
            The character, or '' if past EOF.
        """
        idx = self._pos_after + offset - 1
        if 0 <= idx < len(self._source):
            return self._source[idx]
        return ""

    def peek_str(self, length: int) -> str:
        """Peek at the next ``length`` characters past the current token."""
        return self._source[self._pos_after:self._pos_after + length]

    def set_skip_enabled(self, enabled: bool) -> None:
        """Toggle skip pattern processing.

        When disabled, skip patterns (whitespace, comments) are not tried.
        Useful for groups where whitespace is significant (e.g., CDATA).
        """
        self._skip_enabled = enabled

    # -- Extension: Token Lookbehind ------------------------------------------

    def previous_token(self) -> Token | None:
        """Return the most recently emitted token, or None at start of input.

        "Emitted" means the token actually made it into the output list â€”
        suppressed tokens are not counted. This provides **lookbehind**
        capability for context-sensitive decisions.

        For example, in JavaScript ``/`` is a regex literal after ``=``,
        ``(`` or ``,`` but a division operator after ``)``, ``]``,
        identifiers, or numbers. The callback can check
        ``ctx.previous_token()`` to decide which interpretation to use.

        Returns:
            The last token in the output list, or None if no tokens
            have been emitted yet.
        """
        return self._previous_token

    # -- Extension: Bracket Depth Tracking ------------------------------------

    def bracket_depth(self, kind: str | None = None) -> int:
        """Return the current nesting depth for a specific bracket type,
        or the total depth across all types if no argument is given.

        Depth starts at 0 and increments on each opener (``(``, ``[``,
        ``{``), decrements on each closer (``)``, ``]``, ``}``). The count
        never goes below 0 â€” unmatched closers are clamped.

        This is essential for template literal interpolation in languages
        like JavaScript, Kotlin, and Ruby, where ``}`` at brace-depth 0
        closes the interpolation rather than being part of a nested
        expression.

        Args:
            kind: Optional bracket type to query â€” one of ``"paren"``,
                ``"bracket"``, or ``"brace"``. If None, returns the sum
                of all three depths.
        """
        return self._lexer.bracket_depth(kind)

    # -- Extension: Newline Detection -----------------------------------------

    def preceded_by_newline(self) -> bool:
        """Return True if a newline appeared between the previous token
        and the current token (i.e., they are on different lines).

        This is used by languages with automatic semicolon insertion
        (JavaScript, Go) to detect line breaks that trigger implicit
        statement termination.

        Returns False if there is no previous token (start of input).
        """
        if self._previous_token is None:
            return False
        return self._previous_token.line < self._current_token_line


# ---------------------------------------------------------------------------
# The Grammar-Driven Lexer
# ---------------------------------------------------------------------------


class GrammarLexer:
    """A lexer driven by a ``TokenGrammar`` (parsed from a ``.tokens`` file).

    Instead of hardcoded character-matching logic, this lexer:

    1. Compiles each token definition's pattern into a regex
    2. At each position, tries each regex in definition order (first match wins)
    3. Emits a ``Token`` with the matched type and value

    This is fundamentally different from the hand-written ``Lexer``:

    - **Hand-written**: dispatch on first character, custom read methods
    - **Grammar-driven**: regex matching in priority order

    Both produce ``Token`` objects, so the parser does not care which lexer
    generated them.

    Usage::

        from grammar_tools import parse_token_grammar
        from lexer import GrammarLexer

        grammar = parse_token_grammar(open("python.tokens").read())
        tokens = GrammarLexer("x = 1 + 2", grammar).tokenize()

    Attributes:
        _source: The complete source code string being tokenized.
        _grammar: The ``TokenGrammar`` that defines which tokens to recognize.
        _pos: The current position (index) in the source string.
        _line: The current line number (1-based), for error reporting.
        _column: The current column number (1-based), for error reporting.
        _keyword_set: A pre-computed frozenset of keywords for O(1) lookup.
        _reserved_set: Reserved keywords that cause lex errors.
        _patterns: Compiled regex patterns paired with their definition names
            and aliases, in the order they should be tried.
        _skip_patterns: Compiled patterns for text that should be consumed
            silently (no token emitted). Used for comments and whitespace.
        _alias_map: Maps definition names to their aliases. When a token
            matches a definition with an alias, the alias is used as the
            emitted token type.
        _indentation_mode: Whether Python-style INDENT/DEDENT tracking is on.
    """

    def __init__(self, source: str, grammar: TokenGrammar) -> None:
        """Initialize the grammar-driven lexer.

        Args:
            source: The raw source code text to tokenize.
            grammar: A ``TokenGrammar`` object (typically parsed from a
                ``.tokens`` file using ``grammar_tools.parse_token_grammar``).
        """
        # For case-insensitive languages, lowercase the source text so that
        # regex patterns match regardless of input casing. We keep the original
        # source so that string literal values can preserve their original case
        # (e.g., 'Ada' should tokenize as STRING("Ada"), not STRING("ada")).
        # Keywords are uppercased explicitly via value.upper() in token emission.
        self._original_source = source
        self._source = (
            source.lower() if (grammar.case_insensitive or not grammar.case_sensitive)
            else source
        )
        self._grammar = grammar
        self._pos = 0
        self._line = 1
        self._column = 1

        # Case-insensitive mode â€” when True, keyword matching ignores case.
        # Enabled via ``# @case_insensitive true`` in the .tokens file.
        # When active:
        #   - keywords are stored in uppercase in the keyword/reserved sets
        #   - NAME values are compared via value.upper() against those sets
        #   - matched keyword tokens have their value normalized to uppercase
        self._case_insensitive = grammar.case_insensitive

        # Pre-compute keyword set for fast membership testing.
        # When the lexer matches a NAME token, it checks this set to decide
        # whether the value should be reclassified as a KEYWORD.
        # In case-insensitive mode, every keyword is stored as uppercase so
        # that ``value.upper()`` lookups will find a match regardless of the
        # original casing in the source (e.g., "SELECT", "select", "Select"
        # all map to the same uppercase entry).
        if self._case_insensitive:
            self._keyword_set = frozenset(kw.upper() for kw in grammar.keywords)
        else:
            self._keyword_set = frozenset(grammar.keywords)

        # Reserved keywords cause immediate lex errors. In Starlark, words
        # like "class" and "import" are reserved â€” using them is a syntax
        # error at lex time rather than a confusing parse error later.
        # Apply the same uppercase normalisation in case-insensitive mode so
        # that reserved-word detection is also case-insensitive.
        if self._case_insensitive:
            self._reserved_set = frozenset(
                kw.upper() for kw in grammar.reserved_keywords
            )
        else:
            self._reserved_set = frozenset(grammar.reserved_keywords)

        # Indentation mode flag. When active, the lexer maintains an
        # indentation stack and emits INDENT/DEDENT/NEWLINE tokens.
        self._indentation_mode = grammar.mode == "indentation"
        self._layout_mode = grammar.mode == "layout"
        self._layout_keyword_set: frozenset[str] = frozenset(
            getattr(grammar, "layout_keywords", []) or []
        )

        # Whether the grammar has skip patterns defined. When skip patterns
        # exist, they replace the default whitespace-skipping behavior.
        self._has_skip_patterns = len(grammar.skip_definitions) > 0

        # Escape mode controls STRING token processing.
        # None = standard (strip quotes + process escapes).
        # "none" = strip quotes only, no escape processing.
        self._escape_mode = grammar.escape_mode

        # Case sensitivity mode. When False, the lexer lowercases input
        # before matching and promotes NAME â†’ KEYWORD for lowercased values
        # that match keywords. Used by case-insensitive languages like VHDL.
        self._case_sensitive = grammar.case_sensitive

        # Build alias map: definition name â†’ alias name.
        # For example, STRING_DQ â†’ STRING. When we match STRING_DQ, we
        # emit the token type as STRING (the alias).
        self._alias_map: dict[str, str] = {}
        for defn in grammar.definitions:
            if defn.alias:
                self._alias_map[defn.name] = defn.alias

        # Compile token patterns into regex objects.
        # -------------------------------------------
        # Order matters â€” patterns are tried in the order they appear in the
        # .tokens file. This is the "first match wins" rule from Lex/Flex.
        self._patterns: list[tuple[str, re.Pattern[str]]] = []
        for defn in grammar.definitions:
            if defn.is_regex:
                pattern = re.compile(defn.pattern)
            else:
                pattern = re.compile(re.escape(defn.pattern))
            self._patterns.append((defn.name, pattern))

        # Compile skip patterns (comments, whitespace, etc.).
        # These are tried before token patterns at each position.
        self._skip_patterns: list[re.Pattern[str]] = []
        for defn in grammar.skip_definitions:
            if defn.is_regex:
                self._skip_patterns.append(re.compile(defn.pattern))
            else:
                self._skip_patterns.append(re.compile(re.escape(defn.pattern)))

        # Compile error patterns (error recovery tokens).
        # These are tried as a last resort when no normal token matches.
        # Error tokens allow graceful degradation for malformed inputs â€”
        # for example, CSS emits BAD_STRING for unclosed strings instead
        # of crashing with a LexerError.
        self._error_patterns: list[tuple[str, re.Pattern[str]]] = []
        for defn in grammar.error_definitions:
            if defn.is_regex:
                pattern = re.compile(defn.pattern)
            else:
                pattern = re.compile(re.escape(defn.pattern))
            self._error_patterns.append((defn.name, pattern))

        # --- Pattern groups ---
        # Compile per-group patterns. The "default" group uses the
        # top-level definitions. Named groups use their own definitions.
        # When no groups are defined, _group_patterns has only "default".
        self._group_patterns: dict[str, list[tuple[str, re.Pattern[str]]]] = {
            "default": list(self._patterns),
        }
        for group_name, group in grammar.groups.items():
            compiled: list[tuple[str, re.Pattern[str]]] = []
            for defn in group.definitions:
                if defn.is_regex:
                    pat = re.compile(defn.pattern)
                else:
                    pat = re.compile(re.escape(defn.pattern))
                compiled.append((defn.name, pat))
                # Register aliases from group definitions
                if defn.alias:
                    self._alias_map[defn.name] = defn.alias
            self._group_patterns[group_name] = compiled

        # The group stack. Bottom is always "default". Top is the active
        # group whose patterns are tried during token matching.
        self._group_stack: list[str] = ["default"]

        # On-token callback â€” None means no callback (zero overhead).
        self._on_token: (
            Callable[[Token, LexerContext], None] | None
        ) = None

        # Skip enabled flag â€” can be toggled by callbacks for groups
        # where whitespace is significant (e.g., CDATA, comments).
        self._skip_enabled: bool = True

        # -- Extension: Token lookbehind --
        # The most recently emitted token, for lookbehind in callbacks.
        # Updated after each token push (including callback-emitted tokens).
        # Reset to None on each tokenize() call.
        self._last_emitted_token: Token | None = None

        # -- Extension: Bracket depth tracking --
        # Per-type bracket nesting depth counters. Tracks ``()``, ``[]``,
        # and ``{}`` independently. Updated after each token match in both
        # standard and indentation modes. Exposed to callbacks via
        # ``LexerContext.bracket_depth()``.
        self._bracket_depths: dict[str, int] = {
            "paren": 0,
            "bracket": 0,
            "brace": 0,
        }

        # -- Extension: Context keywords --
        # Pre-computed set of context-sensitive keywords for O(1) lookup.
        # Words in this set are emitted as NAME with TOKEN_CONTEXT_KEYWORD flag.
        self._context_keyword_set: frozenset[str] = frozenset(
            grammar.context_keywords if hasattr(grammar, 'context_keywords') and grammar.context_keywords else []
        )

        # Transform hooks â€” pluggable pipeline stages for language-specific
        # processing. Hooks compose left-to-right: if three pre_tokenize hooks
        # A, B, C are registered, source flows through A â†’ B â†’ C.
        # When no hooks are registered, zero overhead â€” the existing tokenize()
        # path executes unchanged.
        self._pre_tokenize_hooks: list[Callable[[str], str]] = []
        self._post_tokenize_hooks: list[Callable[[list[Token]], list[Token]]] = []

    def set_on_token(
        self,
        callback: Callable[[Token, LexerContext], None] | None,
    ) -> None:
        """Register a callback that fires on every token match.

        The callback receives the matched token and a ``LexerContext``.
        It can use the context to push/pop groups, emit extra tokens,
        or suppress the current token.

        Only one callback can be registered. Pass None to clear.

        The callback is NOT invoked for:
        - Skip pattern matches (they produce no tokens)
        - Tokens emitted via ``context.emit()`` (prevents infinite loops)
        - The EOF token
        """
        self._on_token = callback

    def add_pre_tokenize(self, hook: Callable[[str], str]) -> None:
        """Register a text transform to run before tokenization.

        The hook receives the raw source string and returns a (possibly
        modified) source string. Multiple hooks compose left-to-right:
        source flows through A â†’ B â†’ C before tokenization.

        Use cases:
        - COBOL/FORTRAN column stripping
        - C #include file insertion
        - Line continuation / splicing

        Args:
            hook: A function str â†’ str.
        """
        self._pre_tokenize_hooks.append(hook)

    def add_post_tokenize(self, hook: Callable[[list[Token]], list[Token]]) -> None:
        """Register a token transform to run after tokenization.

        The hook receives the full token list (including EOF) and returns
        a (possibly modified) token list. Multiple hooks compose left-to-right.

        Use cases:
        - C #define macro expansion
        - Token filtering or reclassification
        - Inserting synthetic tokens

        Args:
            hook: A function list[Token] â†’ list[Token].
        """
        self._post_tokenize_hooks.append(hook)

    # -- Extension: Bracket depth -----------------------------------------------

    def bracket_depth(self, kind: str | None = None) -> int:
        """Return the current nesting depth for a specific bracket type,
        or the total depth across all types if no argument is given.

        This is the public API used by LexerContext to expose bracket
        depth to callbacks. Language packages use this for template
        literal interpolation and similar nested constructs.

        Args:
            kind: One of ``"paren"``, ``"bracket"``, ``"brace"``, or
                None for the total across all types.
        """
        if kind is None:
            return sum(self._bracket_depths.values())
        return self._bracket_depths.get(kind, 0)

    def _update_bracket_depth(self, value: str) -> None:
        """Update bracket depth counters based on a token's text value.

        Called after each token match to track bracket nesting. Opening
        brackets increment the appropriate counter; closing brackets
        decrement it (clamped to 0 so unmatched closers don't go negative).

        Args:
            value: The matched token's text value.
        """
        if value == "(":
            self._bracket_depths["paren"] += 1
        elif value == ")":
            self._bracket_depths["paren"] = max(0, self._bracket_depths["paren"] - 1)
        elif value == "[":
            self._bracket_depths["bracket"] += 1
        elif value == "]":
            self._bracket_depths["bracket"] = max(0, self._bracket_depths["bracket"] - 1)
        elif value == "{":
            self._bracket_depths["brace"] += 1
        elif value == "}":
            self._bracket_depths["brace"] = max(0, self._bracket_depths["brace"] - 1)

    # -- Main tokenization entry point ----------------------------------------

    def tokenize(self) -> list[Token]:
        """Tokenize the source code using the grammar's token definitions.

        The tokenization pipeline has three stages:

        1. **Pre-tokenize hooks** â€” transform the source text before lexing.
           Each hook receives a string and returns a string. Multiple hooks
           compose left-to-right (A â†’ B â†’ C).

        2. **Core tokenization** â€” the existing grammar-driven lexer logic,
           dispatching to standard or indentation mode.

        3. **Post-tokenize hooks** â€” transform the token list after lexing.
           Each hook receives a token list and returns a token list.

        When no hooks are registered, this is equivalent to the original
        tokenize() â€” zero overhead.

        Returns:
            A list of ``Token`` objects, always ending with an EOF token.

        Raises:
            LexerError: If an unexpected character is encountered, a reserved
                keyword is used, or indentation is inconsistent.
        """
        # Stage 1: Pre-tokenize hooks transform the source text.
        # Common use cases: COBOL column stripping, C #include resolution,
        # line continuation splicing. Each hook is str â†’ str.
        if self._pre_tokenize_hooks:
            source = self._source
            for hook in self._pre_tokenize_hooks:
                source = hook(source)
            self._source = source

        # Stage 2: Core tokenization â€” dispatch to standard or indentation mode.
        if self._indentation_mode:
            tokens = self._tokenize_indentation()
        elif self._layout_mode:
            tokens = self._tokenize_layout()
        else:
            tokens = self._tokenize_standard()

        # Stage 3: Post-tokenize hooks transform the token list.
        # Common use cases: C #define expansion, token filtering,
        # conditional compilation. Each hook is list[Token] â†’ list[Token].
        if self._post_tokenize_hooks:
            for hook in self._post_tokenize_hooks:
                tokens = hook(tokens)

        return tokens

    # -- Standard (non-indentation) tokenization ------------------------------

    def _tokenize_standard(self) -> list[Token]:
        """Tokenize without indentation tracking.

        The algorithm:

        1. While there are characters left:
           a. If skip patterns exist and skip is enabled, try them.
           b. If no skip patterns, use default whitespace skip.
           c. If the current character is a newline, emit NEWLINE.
           d. Try active group's token patterns (first match wins).
           e. If callback registered, invoke it and process actions.
           f. If nothing matches, try error patterns as fallback.
           g. If still nothing, raise LexerError.
        2. Append EOF.

        When pattern groups are active, the lexer uses ``_group_stack[-1]``
        to determine which set of patterns to try. When a callback is
        registered via ``set_on_token()``, it fires after each token match
        and can push/pop groups, emit extra tokens, or suppress the
        current token.
        """
        tokens: list[Token] = []

        while self._pos < len(self._source):
            char = self._source[self._pos]

            # --- Skip patterns (grammar-defined) ---
            # When the grammar has skip patterns AND skip is enabled, they
            # take over whitespace handling. The callback can disable skip
            # processing for groups where whitespace is significant (CDATA).
            if self._has_skip_patterns:
                if self._skip_enabled and self._try_skip():
                    continue
            else:
                # --- Default whitespace skip ---
                # Without skip patterns, use the hardcoded behavior: skip
                # spaces, tabs, carriage returns silently.
                if char in " \t\r":
                    self._advance()
                    continue

            # --- Newlines become NEWLINE tokens ---
            # Newlines are structural â€” they mark line boundaries.
            if char == "\n":
                tokens.append(Token(
                    type=TokenType.NEWLINE,
                    value="\\n",
                    line=self._line,
                    column=self._column,
                ))
                self._advance()
                continue

            # --- Try active group's token patterns (first match wins) ---
            # The active group is the top of the group stack. When no
            # groups are defined, this is always "default" (the top-level
            # definitions), preserving backward compatibility.
            active_group = self._group_stack[-1]
            token = self._try_match_token_in_group(active_group)
            if token is not None:
                # --- Context keyword flagging ---
                # If the matched token is a NAME whose value is in the
                # context keyword set, flag it with TOKEN_CONTEXT_KEYWORD.
                # This lets the parser decide whether it's a keyword or
                # identifier based on syntactic position.
                if (
                    self._context_keyword_set
                    and token.type in ("NAME", TokenType.NAME)
                    and token.value in self._context_keyword_set
                ):
                    token = Token(
                        type=token.type,
                        value=token.value,
                        line=token.line,
                        column=token.column,
                        flags=(token.flags or 0) | TOKEN_CONTEXT_KEYWORD,
                    )

                # --- Update bracket depth ---
                self._update_bracket_depth(token.value)

                # --- Invoke on-token callback ---
                # The callback can push/pop groups, emit extra tokens,
                # suppress the current token, or toggle skip processing.
                # Emitted tokens do NOT re-trigger the callback.
                if self._on_token is not None:
                    ctx = LexerContext(
                        self,
                        self._source,
                        self._pos,
                        previous_token=self._last_emitted_token,
                        current_token_line=token.line,
                    )
                    self._on_token(token, ctx)

                    # Apply suppression: if the callback suppressed this
                    # token, don't add it to the output.
                    if not ctx._suppressed:
                        tokens.append(token)
                        self._last_emitted_token = token

                    # Append any tokens emitted by the callback.
                    for emitted in ctx._emitted:
                        tokens.append(emitted)
                        self._last_emitted_token = emitted

                    # Apply group stack actions in order.
                    for action, group_name in ctx._group_actions:
                        if action == "push":
                            self._group_stack.append(group_name)
                        elif action == "pop" and len(self._group_stack) > 1:
                            self._group_stack.pop()

                    # Apply skip toggle if the callback changed it.
                    if ctx._skip_enabled is not None:
                        self._skip_enabled = ctx._skip_enabled
                else:
                    tokens.append(token)
                    self._last_emitted_token = token
                continue

            # --- Try error patterns as fallback ---
            # Error patterns allow graceful degradation: instead of crashing,
            # the lexer emits an error token (e.g., BAD_STRING for unclosed
            # strings). The parser can then handle or report these tokens.
            error_token = self._try_match_error_token()
            if error_token is not None:
                tokens.append(error_token)
                continue

            raise LexerError(
                f"Unexpected character: {char!r}",
                line=self._line,
                column=self._column,
            )

        # --- Append EOF sentinel ---
        tokens.append(Token(
            type=TokenType.EOF,
            value="",
            line=self._line,
            column=self._column,
        ))

        # Reset state for reuse (in case tokenize is called again).
        self._group_stack = ["default"]
        self._skip_enabled = True
        self._last_emitted_token = None
        self._bracket_depths = {"paren": 0, "bracket": 0, "brace": 0}

        return tokens

    # -- Indentation mode tokenization ----------------------------------------
    #
    # Indentation-sensitive languages like Python and Starlark use whitespace
    # to delimit blocks. Instead of braces or ``end`` keywords, the amount
    # of leading whitespace on each line determines the block structure.
    #
    # The algorithm is based on CPython's tokenizer (documented in PEP 7 and
    # the Python Language Reference, section 2.1.8). It works as follows:
    #
    # 1. Maintain an **indent stack** â€” a stack of indentation levels. It
    #    starts with [0] (no indentation).
    #
    # 2. At the beginning of each **logical line**:
    #    a. Count the number of leading spaces.
    #    b. If it's greater than the top of the stack â†’ push and emit INDENT.
    #    c. If it's less â†’ pop until we find a matching level, emitting DEDENT
    #       for each pop. If no level matches, it's an indentation error.
    #    d. If it's equal â†’ same block, nothing to emit.
    #
    # 3. **Implicit line joining**: Inside brackets (parentheses, square
    #    brackets, curly braces), newlines are ignored. This means:
    #      f(1,
    #        2,
    #        3)
    #    is a single logical line. We track bracket depth to know when to
    #    suppress NEWLINE/INDENT/DEDENT.
    #
    # 4. **Blank lines** (lines containing only whitespace or comments) do
    #    not emit NEWLINE tokens and don't affect indentation.
    #
    # 5. At **EOF**, emit DEDENTs for every remaining indent level on the
    #    stack (plus a final NEWLINE if the file didn't end with one).

    def _tokenize_indentation(self) -> list[Token]:
        """Tokenize with Python-style indentation tracking.

        This method implements the full indentation algorithm described above.
        It processes the source line-by-line conceptually, but character-by-
        character in practice.

        Returns:
            A list of tokens with synthetic INDENT/DEDENT/NEWLINE tokens.
        """
        tokens: list[Token] = []

        # The indent stack tracks nesting levels. Each entry is the number
        # of spaces at that indentation level. Starts at 0 (top-level).
        indent_stack: list[int] = [0]

        # Bracket depth for implicit line joining. When > 0, newlines are
        # silently consumed and indentation changes are ignored.
        bracket_depth = 0

        # Track whether we're at the start of a logical line (need to
        # process indentation).
        at_line_start = True

        while self._pos < len(self._source):
            # --- Beginning of a logical line: process indentation ---
            if at_line_start and bracket_depth == 0:
                indent_tokens = self._process_line_start(indent_stack)
                if indent_tokens is None:
                    # Blank/comment-only line â€” was consumed, continue
                    continue
                tokens.extend(indent_tokens)
                at_line_start = False
                continue

            char = self._source[self._pos]

            # --- Skip patterns (comments, inline whitespace) ---
            if self._try_skip():
                continue

            # --- Newline handling ---
            if char == "\n":
                if bracket_depth > 0:
                    # Inside brackets: implicit line joining.
                    # Consume the newline silently â€” no NEWLINE token.
                    self._advance()
                else:
                    # End of a logical line: emit NEWLINE.
                    tokens.append(Token(
                        type="NEWLINE",
                        value="\\n",
                        line=self._line,
                        column=self._column,
                    ))
                    self._advance()
                    at_line_start = True
                continue

            # --- Track bracket depth for implicit line joining ---
            # Opening brackets increase depth; closing brackets decrease it.
            # This is how Python allows multi-line expressions inside brackets:
            #   x = (1 +
            #        2 +
            #        3)
            if char in "([{":
                bracket_depth += 1
            elif char in ")]}":
                bracket_depth = max(0, bracket_depth - 1)

            # --- Try each token pattern ---
            token = self._try_match_token()
            if token is not None:
                tokens.append(token)
                continue

            # --- Try error patterns as fallback ---
            error_token = self._try_match_error_token()
            if error_token is not None:
                tokens.append(error_token)
                continue

            raise LexerError(
                f"Unexpected character: {char!r}",
                line=self._line,
                column=self._column,
            )

        # --- EOF cleanup ---
        # If we didn't end with a newline, emit one.
        if tokens and tokens[-1].type != "NEWLINE":
            tokens.append(Token(
                type="NEWLINE",
                value="\\n",
                line=self._line,
                column=self._column,
            ))

        # Emit DEDENTs for each remaining indentation level.
        while len(indent_stack) > 1:
            indent_stack.pop()
            tokens.append(Token(
                type="DEDENT",
                value="",
                line=self._line,
                column=self._column,
            ))

        # Append EOF.
        tokens.append(Token(
            type="EOF",
            value="",
            line=self._line,
            column=self._column,
        ))

        return tokens

    def _tokenize_layout(self) -> list[Token]:
        return self._apply_layout(self._tokenize_standard())

    def _apply_layout(self, tokens: list[Token]) -> list[Token]:
        result: list[Token] = []
        layout_stack: list[int] = []
        pending_layouts = 0
        suppress_depth = 0

        for index, token in enumerate(tokens):
            type_name = token.type if isinstance(token.type, str) else token.type.name

            if type_name == "NEWLINE":
                result.append(token)
                next_token = self._next_layout_token(tokens, index + 1)
                if suppress_depth == 0 and next_token is not None:
                    while layout_stack and next_token.column < layout_stack[-1]:
                        result.append(self._virtual_layout_token("VIRTUAL_RBRACE", "}", next_token))
                        layout_stack.pop()

                    next_type = next_token.type if isinstance(next_token.type, str) else next_token.type.value
                    if (
                        layout_stack
                        and next_type != "EOF"
                        and next_token.value != "}"
                        and next_token.column == layout_stack[-1]
                    ):
                        result.append(self._virtual_layout_token("VIRTUAL_SEMICOLON", ";", next_token))
                continue

            if type_name == "EOF":
                while layout_stack:
                    result.append(self._virtual_layout_token("VIRTUAL_RBRACE", "}", token))
                    layout_stack.pop()
                result.append(token)
                continue

            if pending_layouts > 0:
                if token.value == "{":
                    pending_layouts -= 1
                else:
                    for _ in range(pending_layouts):
                        layout_stack.append(token.column)
                        result.append(self._virtual_layout_token("VIRTUAL_LBRACE", "{", token))
                    pending_layouts = 0

            result.append(token)

            if not self._is_virtual_layout_token(token):
                if token.value in ("(", "[", "{"):
                    suppress_depth += 1
                elif token.value in (")", "]", "}") and suppress_depth > 0:
                    suppress_depth -= 1

            if self._is_layout_keyword(token):
                pending_layouts += 1

        return result

    def _next_layout_token(self, tokens: list[Token], start_index: int) -> Token | None:
        for token in tokens[start_index:]:
            type_name = token.type if isinstance(token.type, str) else token.type.name
            if type_name != "NEWLINE":
                return token
        return None

    def _virtual_layout_token(self, type_name: str, value: str, anchor: Token) -> Token:
        return Token(type=type_name, value=value, line=anchor.line, column=anchor.column)

    def _is_virtual_layout_token(self, token: Token) -> bool:
        type_name = token.type if isinstance(token.type, str) else token.type.name
        return type_name.startswith("VIRTUAL_")

    def _is_layout_keyword(self, token: Token) -> bool:
        if not self._layout_keyword_set:
            return False
        value = token.value or ""
        return value in self._layout_keyword_set or value.lower() in self._layout_keyword_set

    def _process_line_start(
        self, indent_stack: list[int],
    ) -> list[Token] | None:
        """Process the beginning of a logical line for indentation.

        Counts leading spaces, skips blank/comment-only lines, and emits
        INDENT or DEDENT tokens as needed.

        Args:
            indent_stack: The current indentation stack (modified in place).

        Returns:
            A list of INDENT/DEDENT tokens to emit, or None if the line
            was blank/comment-only and was fully consumed.
        """
        # Count leading spaces. Tabs are not allowed in leading whitespace
        # for indentation-sensitive languages (they cause ambiguity).
        indent = 0
        while self._pos < len(self._source):
            char = self._source[self._pos]
            if char == " ":
                indent += 1
                self._advance()
            elif char == "\t":
                raise LexerError(
                    "Tab character in leading whitespace is not allowed "
                    "in indentation mode (use spaces)",
                    line=self._line,
                    column=self._column,
                )
            else:
                break

        # Check if this is a blank line or a comment-only line.
        # These don't affect indentation and don't emit NEWLINE.
        if self._pos >= len(self._source):
            return []  # EOF after whitespace
        char = self._source[self._pos]
        if char == "\n":
            # Blank line â€” consume and continue
            self._advance()
            return None
        if char == "#":
            # Comment-only line â€” consume everything up to newline
            while self._pos < len(self._source) and self._source[self._pos] != "\n":
                self._advance()
            if self._pos < len(self._source):
                self._advance()  # consume the \n
            return None

        # This is a real line with content. Compare indentation to the stack.
        tokens: list[Token] = []
        current_indent = indent_stack[-1]

        if indent > current_indent:
            # Deeper indentation â†’ new block. Push and emit INDENT.
            indent_stack.append(indent)
            tokens.append(Token(
                type="INDENT",
                value="",
                line=self._line,
                column=1,
            ))
        elif indent < current_indent:
            # Shallower indentation â†’ closing one or more blocks.
            # Pop the stack until we find the matching level.
            while indent_stack[-1] > indent:
                indent_stack.pop()
                tokens.append(Token(
                    type="DEDENT",
                    value="",
                    line=self._line,
                    column=1,
                ))
            # After popping, the top of the stack must equal the current
            # indent. If not, the indentation is inconsistent.
            if indent_stack[-1] != indent:
                raise LexerError(
                    f"Indentation level {indent} does not match any "
                    f"outer indentation level (stack: {indent_stack})",
                    line=self._line,
                    column=1,
                )

        return tokens

    # -- Shared helpers -------------------------------------------------------

    def _try_skip(self) -> bool:
        """Try to match and consume a skip pattern at the current position.

        Skip patterns are defined in the ``skip:`` section of a .tokens file.
        They match text that should be consumed without emitting a token â€”
        typically comments and inline whitespace.

        Returns:
            True if a skip pattern matched (text was consumed), False otherwise.
        """
        remaining = self._source[self._pos:]
        for pattern in self._skip_patterns:
            match = pattern.match(remaining)
            if match:
                for _ in range(len(match.group(0))):
                    self._advance()
                return True
        return False

    def _try_match_token(self) -> Token | None:
        """Try to match a token pattern at the current position.

        Uses the default group's patterns (``self._patterns``). This method
        is used by the indentation tokenizer which does not support groups.

        Returns:
            A Token if a pattern matched, None otherwise.
        """
        return self._try_match_token_in_group("default")

    def _try_match_token_in_group(self, group_name: str) -> Token | None:
        """Try to match a token pattern from a specific group.

        Tries each compiled pattern in the named group in priority order
        (first match wins). Handles keyword detection, reserved word
        checking, aliases, and string escape processing.

        Args:
            group_name: The pattern group to use (e.g., "default", "tag").

        Returns:
            A Token if a pattern matched, None otherwise.
        """
        remaining = self._source[self._pos:]
        patterns = self._group_patterns.get(group_name, self._patterns)

        for token_name, pattern in patterns:
            match = pattern.match(remaining)
            if match:
                value = match.group(0)
                original_value = self._original_source[
                    self._pos : self._pos + len(value)
                ]
                start_line = self._line
                start_column = self._column

                # Determine the token type for this match.
                token_type = self._resolve_token_type(token_name, value)

                # Handle STRING tokens: strip quotes and process escapes.
                # We check the effective type (after alias resolution) so
                # that STRING_DQ -> STRING still gets escape processing.
                #
                # Escape mode controls what happens to STRING values:
                # - None (default): strip quotes AND process escape sequences
                # - "none": strip quotes only, leave escapes as-is
                #
                # IMPORTANT: use the original (non-lowercased) source for
                # the string body so that 'Ada' tokenizes as STRING("Ada"),
                # not STRING("ada"). The lowercased source is only for
                # pattern matching; string content should be case-preserved.
                effective_name = self._alias_map.get(token_name, token_name)
                if effective_name == "STRING" or token_name == "STRING":
                    inner = original_value[1:-1]
                    if self._escape_mode != "none":
                        inner = self._process_escapes(inner)
                    token = Token(
                        type=token_type,
                        value=inner,
                        line=start_line,
                        column=start_column,
                    )
                else:
                    # In case-insensitive mode, KEYWORD values are normalised
                    # to uppercase so that "select", "SELECT", and "Select"
                    # all produce a KEYWORD token with value "SELECT".
                    # ``@case_insensitive true`` grammars preserve the
                    # original spelling for non-keyword tokens, while
                    # legacy ``case_sensitive=False`` grammars keep the
                    # historic lowercased value emission.
                    emit_value = (
                        original_value if self._case_insensitive else value
                    )
                    if self._case_insensitive and token_type in (
                        TokenType.KEYWORD, "KEYWORD"
                    ):
                        emit_value = value.upper()
                    token = Token(
                        type=token_type,
                        value=emit_value,
                        line=start_line,
                        column=start_column,
                    )

                # Advance position by the number of characters matched.
                for _ in range(len(value)):
                    self._advance()

                return token

        return None

    def _try_match_error_token(self) -> Token | None:
        """Try to match an error pattern at the current position.

        Error patterns are a fallback â€” they are only tried when no normal
        token pattern matches. This allows graceful degradation for malformed
        inputs. For example, CSS can emit a ``BAD_STRING`` token for an
        unclosed string instead of crashing with a ``LexerError``.

        Error tokens are emitted with the pattern name as the token type
        (e.g., ``"BAD_STRING"``). Downstream consumers can check for these
        error token types and report or recover accordingly.

        Returns:
            A Token if an error pattern matched, None otherwise.
        """
        remaining = self._source[self._pos:]

        for error_name, pattern in self._error_patterns:
            match = pattern.match(remaining)
            if match:
                value = match.group(0)
                start_line = self._line
                start_column = self._column

                token = Token(
                    type=error_name,
                    value=value,
                    line=start_line,
                    column=start_column,
                )

                for _ in range(len(value)):
                    self._advance()

                return token

        return None

    def _advance(self) -> None:
        """Move position forward by one character, tracking line and column.

        When we encounter a newline character, we increment the line counter
        and reset the column to 1. For all other characters, we just increment
        the column.
        """
        if self._pos < len(self._source):
            if self._source[self._pos] == "\n":
                self._line += 1
                self._column = 1
            else:
                self._column += 1
            self._pos += 1

    def _resolve_token_type(
        self, token_name: str, value: str,
    ) -> TokenType | str:
        """Map a token definition name to a token type for emission.

        The resolution order:

        1. **Reserved keyword check**: If the effective type is NAME and the
           value is a reserved keyword, raise a LexerError immediately.

        2. **Keyword detection**: If the effective type is NAME and the value
           is a keyword, return KEYWORD (or the string "KEYWORD" in extended
           mode).

        3. **Alias resolution**: If the definition has an alias, use the alias
           name as the token type (e.g., STRING_DQ â†’ STRING).

        4. **Enum mapping**: Try to find a matching TokenType enum member.
           This provides backward compatibility with simple grammars.

        5. **String fallback**: If no enum member exists, use the token name
           as a string. This handles Starlark's 45+ custom token names.

        Args:
            token_name: The definition name from the grammar
                (e.g., ``"NAME"``, ``"STRING_DQ"``, ``"FLOOR_DIV"``).
            value: The actual matched text from the source code.

        Returns:
            A TokenType enum member or a string token type name.
        """
        # Resolve alias: STRING_DQ â†’ STRING, FLOOR_DIV_EQUALS â†’ FLOOR_DIV_EQUALS
        effective_name = self._alias_map.get(token_name, token_name)

        # Reserved keyword check â€” error on reserved identifiers.
        # In Starlark, "class", "import", etc. are reserved. Using them
        # is a lex error rather than a confusing parse error.
        # In case-insensitive mode, compare the uppercased value so that
        # "CLASS", "class", and "Class" all trigger the same error.
        lookup_value = value.upper() if self._case_insensitive else value
        if effective_name == "NAME" and lookup_value in self._reserved_set:
            raise LexerError(
                f"Reserved keyword '{value}' cannot be used as an identifier",
                line=self._line,
                column=self._column,
            )

        # Keyword detection â€” reclassify NAME â†’ KEYWORD when the value
        # matches a known keyword.
        # In case-insensitive mode we test against the uppercase lookup value
        # (the set already stores entries in uppercase, see __init__).
        if effective_name == "NAME" and lookup_value in self._keyword_set:
            try:
                return TokenType.KEYWORD
            except (KeyError, AttributeError):
                return "KEYWORD"

        # Try to map to a TokenType enum member (backward compatibility).
        try:
            return TokenType[effective_name]
        except KeyError:
            # No enum member â€” use the string name. This is the normal
            # path for extended grammars (Starlark, etc.) that define
            # token names beyond the basic TokenType enum.
            return effective_name

    @staticmethod
    def _process_escapes(s: str) -> str:
        r"""Process escape sequences in a string value.

        This handles the same escape sequences as the hand-written lexer:

        - ``\n`` becomes a newline character
        - ``\t`` becomes a tab character
        - ``\\`` becomes a literal backslash
        - ``\"`` becomes a literal double quote
        - Any other ``\X`` becomes just ``X`` (unknown escapes pass through)

        This ensures that ``GrammarLexer`` produces identical string values
        to the hand-written ``Lexer``.

        Args:
            s: The raw string content (after removing surrounding quotes).

        Returns:
            The string with escape sequences resolved.
        """
        result: list[str] = []
        i = 0
        while i < len(s):
            if s[i] == "\\" and i + 1 < len(s):
                next_char = s[i + 1]

                # Standard escape sequences. This map covers all escapes
                # defined by JSON (RFC 8259 section 7) and most programming
                # languages. Previously only \n, \t, \\, and \" were handled;
                # \b, \f, \r, and \/ were added for JSON support.
                escape_map = {
                    "n": "\n",      # line feed
                    "t": "\t",      # tab
                    "r": "\r",      # carriage return
                    "b": "\b",      # backspace
                    "f": "\f",      # form feed
                    "\\": "\\",     # literal backslash
                    '"': '"',       # literal double quote
                    "/": "/",       # solidus (JSON allows \/ as an escape)
                }

                if next_char in escape_map:
                    result.append(escape_map[next_char])
                    i += 2
                elif next_char == "u" and i + 5 < len(s):
                    # Unicode escape: \uXXXX where XXXX is exactly 4 hex digits.
                    # This is required by JSON (RFC 8259) and supported by most
                    # programming languages. We validate that the 4 characters
                    # after \u are valid hex digits, then convert to the
                    # corresponding Unicode character.
                    hex_str = s[i + 2 : i + 6]
                    if len(hex_str) == 4 and all(
                        c in "0123456789abcdefABCDEF" for c in hex_str
                    ):
                        result.append(chr(int(hex_str, 16)))
                        i += 6
                    else:
                        # Invalid hex digits â€” pass through as-is.
                        result.append(next_char)
                        i += 2
                else:
                    # Unknown escape â€” pass through the escaped character.
                    result.append(next_char)
                    i += 2
            else:
                result.append(s[i])
                i += 1
        return "".join(result)
