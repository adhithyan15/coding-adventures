defmodule CodingAdventures.DartmouthBasicLexer do
  @moduledoc """
  Dartmouth BASIC Lexer — Thin wrapper around the grammar-driven lexer engine.

  ## A Brief History: The Birth of BASIC

  In 1964, John Kemeny and Thomas Kurtz at Dartmouth College created BASIC
  (Beginner's All-purpose Symbolic Instruction Code) on a GE-225 mainframe.
  Their radical idea: every Dartmouth student, regardless of major, should
  be able to write and run programs. The existing languages of the day —
  Fortran, ALGOL, COBOL — required weeks of study before you could do
  anything useful.

  BASIC's answer was radical simplicity:

  - **Line numbers everywhere**: Every statement is prefixed with a number.
    You type `10 PRINT "HELLO"` and the system stores it as line 10. You
    can retype a line to replace it, or type `DELETE 10` to remove it.
    Lines run in numeric order, so you can insert line 15 between 10 and 20
    without retyping the whole program.

  - **GOTO for control flow**: Before structured programming (Dijkstra's
    famous 1968 letter "Go To Statement Considered Harmful" was still four
    years away), GOTO was the only conditional branch. `IF X > 0 THEN 50`
    means "if X is positive, jump to line 50." Simple, direct, powerful.

  - **Every variable is a number**: The 1964 Dartmouth BASIC has no integer
    type. Everything is a floating-point number. Variable names are one
    uppercase letter (A–Z) or one letter followed by one digit (A0–Z9).
    All variables start at 0 automatically.

  - **Built-in math functions**: SIN, COS, TAN, LOG, EXP, SQR, ABS, INT,
    RND, SGN, ATN — the functions a scientist or engineer needs most.

  - **Uppercase only**: The teletypes of 1964 had no lowercase characters.
    The whole language is uppercase. This lexer normalises input to uppercase
    before tokenising (`@case_insensitive true` in the grammar).

  BASIC became the most widely-used programming language of the 1970s and
  80s. When the Altair 8800 home computer appeared in 1975, Bill Gates and
  Paul Allen's first product was Microsoft BASIC for it. When the Apple II,
  Commodore 64, and IBM PC shipped, they came with BASIC in ROM. Tens of
  millions of people learned to program in BASIC.

  ## Where This Package Fits

  This package is Layer 1 of the Dartmouth BASIC pipeline:

  ```
  BASIC source text  (e.g., "10 LET X = 42\\n20 PRINT X\\n")
        │
        ▼  tokenize/1
  ┌──────────────────────────────────┐
  │   dartmouth_basic_lexer          │  ← this package
  │   dartmouth_basic.tokens grammar │
  └──────────────────────────────────┘
        │
        ▼  [{type, value, line, column}, ...]
  ┌──────────────────────────────────┐
  │   dartmouth_basic_parser         │
  └──────────────────────────────────┘
        │
        ▼  AST
  ┌──────────────────────────────────┐
  │   dartmouth_basic_compiler       │
  └──────────────────────────────────┘
  ```

  The lexer converts raw source text into a flat list of typed tokens.
  It knows nothing about grammar (whether `IF` needs a `THEN`) or semantics
  (whether `GOTO 999` refers to an existing line). Those are the parser's
  and compiler's concerns.

  ## Token Types

  After tokenisation the stream contains these token types:

  | Type        | Example        | Notes                                              |
  |-------------|----------------|----------------------------------------------------|
  | `LINE_NUM`  | `"10"`, `"999"`| First number on a line; applied by post-hook       |
  | `NUMBER`    | `"3.14"`, `"42"` | Numeric literal in an expression                |
  | `STRING`    | `"\\"HELLO\\""` | Includes surrounding double quotes               |
  | `KEYWORD`   | `"PRINT"`, `"LET"` | Always uppercase (case_insensitive)           |
  | `BUILTIN_FN`| `"SIN"`, `"RND"` | One of 11 built-in math functions               |
  | `USER_FN`   | `"FNA"`, `"FNZ"` | User-defined function: FN + one letter          |
  | `NAME`      | `"X"`, `"A1"` | Variable name: one letter + optional digit         |
  | `PLUS`      | `"+"`          |                                                    |
  | `MINUS`     | `"-"`          |                                                    |
  | `STAR`      | `"*"`          | Multiplication                                     |
  | `SLASH`     | `"/"`          | Division                                           |
  | `CARET`     | `"^"`          | Exponentiation: `2^3` = 8                          |
  | `EQ`        | `"="`          | Assignment (LET) and equality (IF) — parser decides|
  | `LT`        | `"<"`          |                                                    |
  | `GT`        | `">"`          |                                                    |
  | `LE`        | `"<="`         | ≤ — must appear before LT in grammar              |
  | `GE`        | `">="`         | ≥ — must appear before GT in grammar              |
  | `NE`        | `"<>"`         | ≠ — not-equal                                     |
  | `LPAREN`    | `"("`          |                                                    |
  | `RPAREN`    | `")"`          |                                                    |
  | `COMMA`     | `","`          | PRINT separator: advance to next print zone        |
  | `SEMICOLON` | `";"`          | PRINT separator: no space                          |
  | `NEWLINE`   | `"\\n"`, `"\\r\\n"` | Statement terminator — significant!          |
  | `EOF`       | `""`           | Always the last token                              |

  ## The LINE_NUM Disambiguation

  Here is the problem: a bare integer like `42` appears in two different
  contexts in BASIC:

  ```
  10 LET X = 42     ← the "10" is a line label; the "42" is a value
  20 GOTO 10        ← the "10" is a jump target
  ```

  A regular expression cannot tell these apart — `[0-9]+` matches all three.
  We solve this with a **post-tokenize hook** that re-labels tokens by position:

  ```
  Rule: the first NUMBER token at the start of a physical line
        (position 0, or immediately after NEWLINE) is LINE_NUM.
  ```

  This maps cleanly to the grammar of BASIC:
  ```
  program   := line*
  line      := LINE_NUM statement NEWLINE
  statement := LET ... | PRINT ... | GOTO NUMBER | ...
  ```

  So `GOTO 10` produces `KEYWORD("GOTO") NUMBER("10")` — the GOTO target
  is a plain NUMBER, not a LINE_NUM. Only the line label gets LINE_NUM.

  ## The REM Comment Suppression

  `REM` introduces a remark (comment) that runs to end of line:

  ```basic
  10 REM THIS IS IGNORED
  20 PRINT "HELLO"
  ```

  The grammar does not have a `REM_TEXT` token because the `grammar_tools`
  engine does not have a "consume until newline" mode. Instead, a second
  post-tokenize hook walks the token list and drops everything between
  `KEYWORD("REM")` and the next `NEWLINE`:

  ```
  Before hook: LINE_NUM("10") KEYWORD("REM") NAME("THIS") KEYWORD("IS") ...
  After hook:  LINE_NUM("10") KEYWORD("REM") NEWLINE("\\n")
  ```

  The NEWLINE is kept because the parser needs it to end the statement.
  The REM keyword itself is kept so the parser can recognise that this
  statement IS a remark (and not try to parse it as an expression).

  ## Usage

      {:ok, tokens} = CodingAdventures.DartmouthBasicLexer.tokenize("10 LET X = 5\\n")
      Enum.each(tokens, fn tok ->
        IO.puts("\#{tok.type}: \#{inspect(tok.value)}")
      end)
      # LINE_NUM: "10"
      # KEYWORD: "LET"
      # NAME: "X"
      # EQ: "="
      # NUMBER: "5"
      # NEWLINE: "\\n"
      # EOF: ""

  ## How It Works

  1. `create_lexer/0` reads `dartmouth_basic.tokens` from the shared grammars
     directory and parses it into a `TokenGrammar` struct — a data structure
     that holds all the regex/literal patterns in priority order.

  2. `tokenize/1` passes the source and grammar to `GrammarLexer.tokenize/3`,
     which scans the source character by character, matching the highest-priority
     pattern at each position. Then the two post-tokenize hooks run in order:
     `relabel_line_numbers/1` then `suppress_rem_content/1`.

  3. The grammar is cached in `:persistent_term` (a BEAM-level key-value store
     for read-heavy immutable data). The first call pays the file-read cost;
     every subsequent call finds the cached grammar in O(1) with zero copying.

  ## Grammar File Location

  The grammar lives at `code/grammars/dartmouth_basic.tokens`, five directories
  up from this source file (`lib/coding_adventures/`) — one more than a module
  at `lib/` would need, because this file is in a subdirectory.
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # -------------------------------------------------------------------------
  # Grammar path resolution
  # -------------------------------------------------------------------------
  #
  # __DIR__ is the directory of THIS source file at compile time:
  #   .../code/packages/elixir/dartmouth_basic_lexer/lib/coding_adventures
  #
  # Walking up with ".." five times:
  #   (1) lib/coding_adventures  →  lib
  #   (2) lib                    →  dartmouth_basic_lexer
  #   (3) dartmouth_basic_lexer  →  elixir
  #   (4) elixir                 →  packages
  #   (5) packages               →  code
  #   then append "grammars"     →  code/grammars
  #
  # Note: this module is nested one directory deeper than modules like
  # algol_lexer (which live at lib/algol_lexer.ex, not
  # lib/coding_adventures/algol_lexer.ex), so we need five ".." steps
  # instead of four.
  #
  # Path.expand/1 resolves all ".." components into an absolute path so that
  # File.read! works regardless of the working directory at runtime.
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  # -------------------------------------------------------------------------
  # Public API
  # -------------------------------------------------------------------------

  @doc """
  Tokenize Dartmouth BASIC 1964 source code.

  Returns `{:ok, tokens}` on success, where `tokens` is a list of
  `%Token{type, value, line, column}` structs terminated by an `EOF` token.

  - `NEWLINE` tokens are included — they act as statement terminators in BASIC.
  - All keywords are returned in uppercase (the grammar is case-insensitive).
  - Line-number tokens at the start of each line are typed `LINE_NUM`, not
    `NUMBER`.
  - Text after `REM` is suppressed; only `KEYWORD("REM")` and `NEWLINE` remain.

  Returns `{:error, message}` if the lexer encounters an unrecoverable error.
  In practice, the grammar's `errors:` section emits `UNKNOWN` tokens for
  bad characters rather than returning an error, so errors from this function
  are rare.

  ## Examples

      iex> {:ok, tokens} = DartmouthBasicLexer.tokenize("10 LET X = 1\\n")
      iex> Enum.map(tokens, & &1.type)
      ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE", "EOF"]

      iex> {:ok, tokens} = DartmouthBasicLexer.tokenize("10 REM IGNORE ME\\n")
      iex> Enum.map(tokens, & &1.type)
      ["LINE_NUM", "KEYWORD", "NEWLINE", "EOF"]

  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()

    # Case handling strategy
    # ----------------------
    # The grammar has `case_sensitive: false` (which triggers source downcasing in
    # the GrammarLexer ONLY when case_insensitive is false). However, because the
    # grammar ALSO has `# @case_insensitive true`, the GrammarLexer does NOT
    # downcase the source automatically — it only uses `case_insensitive` for
    # KEYWORD value upcasing. This is a quirk of the Elixir GrammarLexer design.
    #
    # Solution: add a pre-tokenize hook that lowercases the entire source. All
    # grammar patterns are lowercase, so "PRINT" and "print" both tokenise the
    # same way after downcasing. The grammar's `@case_insensitive true` then
    # upcases KEYWORD values, giving us KEYWORD("PRINT") from either input.
    #
    # For NAME, BUILTIN_FN, and USER_FN tokens, the values come out lowercase
    # (e.g., NAME("x"), BUILTIN_FN("sin")). A fourth post-tokenize hook upcases
    # these to match the uppercase-only convention of 1964 Dartmouth BASIC.
    #
    # Post-tokenize hooks (applied in order after tokenisation):
    #   1. relabel_line_numbers — promotes the first NUMBER on each line to LINE_NUM
    #   2. suppress_rem_content — drops tokens between REM and the next NEWLINE
    #   3. upcase_identifiers   — upcases NAME, BUILTIN_FN, USER_FN values
    GrammarLexer.tokenize(source, grammar,
      pre_tokenize_hooks: [&String.downcase/1],
      post_tokenize_hooks: [
        &relabel_line_numbers/1,
        &suppress_rem_content/1,
        &upcase_identifiers/1
      ]
    )
  end

  @doc """
  Parse the `dartmouth_basic.tokens` grammar file and return the `TokenGrammar`.

  The `TokenGrammar` struct holds all token definitions in priority order.
  You can inspect it to see the full set of token names, patterns, and
  keywords.

  This function always reads the grammar fresh from disk — it does NOT use
  the cached copy. Use it for introspection and testing. For tokenisation,
  `tokenize/1` is more efficient because it reuses the cached grammar.

  ## Example

      grammar = DartmouthBasicLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      # => ["LE", "GE", "NE", "LINE_NUM", "NUMBER", ...]

  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "dartmouth_basic.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # -------------------------------------------------------------------------
  # Grammar caching
  # -------------------------------------------------------------------------

  # Cache the parsed grammar in :persistent_term.
  #
  # :persistent_term is a BEAM-level key-value store designed for "write once,
  # read many" data. Unlike ETS (which copies terms on every read), persistent_term
  # returns a reference to the stored term directly — zero copy overhead on reads.
  # This is ideal for a grammar that is loaded once at startup and never changes.
  #
  # The key is a two-tuple {__MODULE__, :grammar} to avoid collisions with other
  # modules that also use persistent_term.
  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        # First call: read the grammar from disk and cache it.
        grammar = create_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        # Subsequent calls: return the cached grammar immediately.
        grammar
    end
  end

  # -------------------------------------------------------------------------
  # Post-tokenize hook 1: relabel_line_numbers/1
  # -------------------------------------------------------------------------
  #
  # PURPOSE
  # -------
  # Every physical line of a Dartmouth BASIC program begins with a line
  # number. For example:
  #
  #   10 LET X = 5
  #   20 GOTO 10
  #
  # Both the "10" at the start of line 1 and the "10" in "GOTO 10" are
  # tokenised as NUMBER by the grammar (they match the same regex). But they
  # are semantically different:
  #   - The leading "10" is a LABEL that names the line.
  #   - The "10" in "GOTO 10" is a JUMP TARGET (also a label, but in
  #     expression position — the parser knows it must refer to a line).
  #
  # The parser expects LINE_NUM for the leading label, so we relabel it here.
  #
  # ALGORITHM
  # ---------
  # We walk the token list with a state machine having two states:
  #
  #   :at_line_start — we are at the beginning of a new physical line.
  #                    We start here before the first token.
  #   :in_line       — we have seen at least one non-NEWLINE token.
  #
  # Transitions:
  #   :at_line_start × NUMBER  → emit LINE_NUM, transition :in_line
  #   :at_line_start × other   → emit token unchanged, transition :in_line
  #   :in_line × NEWLINE       → emit token, transition :at_line_start
  #   :in_line × other         → emit token unchanged, stay :in_line
  #
  # Note: we build the result list in reverse (prepending is O(1) in Elixir)
  # and reverse at the end. This is the idiomatic Elixir pattern for building
  # a list with Enum.reduce.
  defp relabel_line_numbers(tokens) do
    tokens
    |> Enum.reduce({[], :at_line_start}, fn token, {acc, state} ->
      case {state, token.type} do
        {:at_line_start, "NUMBER"} ->
          # This number is in line-number position — relabel it to LINE_NUM.
          # We use map update syntax (%{struct | field: value}) to create a
          # new token with the type field changed; all other fields stay the same.
          {[%{token | type: "LINE_NUM"} | acc], :in_line}

        {:at_line_start, _} ->
          # A non-number at line start (e.g., a blank line followed by NEWLINE,
          # or EOF). No relabelling needed; just transition to :in_line.
          {[token | acc], :in_line}

        {:in_line, "NEWLINE"} ->
          # End of the current statement. The next token begins a new line.
          {[token | acc], :at_line_start}

        {:in_line, _} ->
          # Middle of a line — leave the token alone.
          {[token | acc], :in_line}
      end
    end)
    |> then(fn {acc, _state} -> Enum.reverse(acc) end)
  end

  # -------------------------------------------------------------------------
  # Post-tokenize hook 2: suppress_rem_content/1
  # -------------------------------------------------------------------------
  #
  # PURPOSE
  # -------
  # The `REM` statement introduces a remark (comment) that extends to the
  # end of the current line. In 1964 Dartmouth BASIC, the REM statement is
  # the ONLY form of comment.
  #
  # Example:
  #   10 REM THIS IS A REMARK AND SHOULD NOT AFFECT EXECUTION
  #   20 PRINT "HELLO"    ← this line runs normally
  #
  # After tokenisation (before this hook), line 10 might look like:
  #   LINE_NUM("10") KEYWORD("REM") NAME("THIS") KEYWORD("IS") NAME("A") ...
  #
  # We want:
  #   LINE_NUM("10") KEYWORD("REM") NEWLINE("\\n")
  #
  # The KEYWORD("REM") itself is kept so the parser knows "this is a REM
  # statement, not an expression". The NEWLINE is kept so the parser knows
  # the statement has ended.
  #
  # ALGORITHM
  # ---------
  # Walk the token list with a boolean flag `suppressing`:
  #
  #   suppressing = false initially.
  #
  #   For each token:
  #     - If it is KEYWORD("REM")  → emit it, set suppressing = true.
  #     - If suppressing and NEWLINE → emit it, set suppressing = false.
  #     - If suppressing            → DROP the token (do not emit), stay suppressing.
  #     - Otherwise                 → emit it, suppressing stays false.
  #
  # "Dropping" a token means not adding it to the accumulator.
  defp suppress_rem_content(tokens) do
    tokens
    |> Enum.reduce({[], false}, fn token, {acc, suppressing} ->
      cond do
        # Seeing REM: emit it and start suppressing everything after it.
        token.type == "KEYWORD" and token.value == "REM" ->
          {[token | acc], true}

        # Seeing NEWLINE while suppressing: the comment line has ended.
        # Emit the NEWLINE (it terminates the REM statement) and stop suppressing.
        token.type == "NEWLINE" ->
          {[token | acc], false}

        # Any other token while suppressing: drop it silently.
        suppressing ->
          {acc, true}

        # Normal token outside of a REM comment: emit it unchanged.
        true ->
          {[token | acc], suppressing}
      end
    end)
    |> then(fn {acc, _suppressing} -> Enum.reverse(acc) end)
  end

  # -------------------------------------------------------------------------
  # Post-tokenize hook 3: upcase_identifiers/1
  # -------------------------------------------------------------------------
  #
  # PURPOSE
  # -------
  # Dartmouth BASIC 1964 is uppercase-only — the GE-225 teletypes could only
  # produce uppercase characters. For authenticity, and for consistency with
  # the KEYWORD token values (which are always uppercase due to the grammar's
  # `@case_insensitive true` directive), all identifier tokens should also
  # have uppercase values.
  #
  # The pre-tokenize hook lowercases the entire source so that case-insensitive
  # pattern matching works uniformly. As a result, NAME, BUILTIN_FN, and
  # USER_FN token values come out lowercase (e.g., NAME("x"), BUILTIN_FN("sin"),
  # USER_FN("fna")). This hook upcases those values to restore the uppercase
  # convention expected by callers and by the downstream parser.
  #
  # Token types that ARE upcased:
  #   NAME       — variable names:    NAME("x") → NAME("X")
  #   BUILTIN_FN — built-in funcs:    BUILTIN_FN("sin") → BUILTIN_FN("SIN")
  #   USER_FN    — user-defined:      USER_FN("fna") → USER_FN("FNA")
  #
  # Token types that are NOT touched:
  #   KEYWORD    — already upcased by the GrammarLexer's case_insensitive path
  #   NUMBER     — numeric literals; case has no meaning
  #   STRING     — string content; kept as-is (the lexer already strips quotes)
  #   Operators  — EQ, PLUS, MINUS, etc.; single-character, case irrelevant
  #   NEWLINE    — the escape sequence "\\n"; not alphabetic
  #   EOF        — empty string
  #   LINE_NUM   — numeric digits; no letters
  #   UNKNOWN    — bad character; preserve as-is for error messages
  defp upcase_identifiers(tokens) do
    Enum.map(tokens, fn token ->
      case token.type do
        type when type in ["NAME", "BUILTIN_FN", "USER_FN"] ->
          %{token | value: String.upcase(token.value)}

        _ ->
          token
      end
    end)
  end
end
