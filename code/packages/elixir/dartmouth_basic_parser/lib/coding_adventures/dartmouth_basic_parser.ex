defmodule CodingAdventures.DartmouthBasicParser do
  @moduledoc """
  Dartmouth BASIC Parser — Thin wrapper around the grammar-driven parser engine.

  ## A Brief History: Parsing BASIC

  When John Kemeny and Thomas Kurtz designed Dartmouth BASIC in 1964, parsing
  was a manual, bespoke affair. Each language had its own hand-written parser
  because the field of formal language theory was young. Chomsky had published
  his hierarchy only in 1956; Knuth's LR parsing paper was still two years
  away (1965). The BASIC system on the GE-225 used a simple top-down recursive
  scan because BASIC's grammar is deliberately unambiguous: every statement
  begins with a distinct keyword, and every expression uses the classic
  precedence cascade of arithmetic.

  Today we can parse BASIC using a *grammar-driven* engine: a single file,
  `dartmouth_basic.grammar`, describes the complete syntax in EBNF notation,
  and a reusable parser engine interprets it at runtime. This package is a
  thin adapter that wires together:

    1. The **Dartmouth BASIC lexer** (`dartmouth_basic_lexer`) — converts raw
       source text into a typed token stream with LINE_NUM, KEYWORD, NAME, etc.

    2. The **grammar-driven parser engine** (`parser`) — reads
       `dartmouth_basic.grammar` and applies it to the token stream using
       recursive-descent with packrat memoization.

  The result is an **Abstract Syntax Tree (AST)** — a tree of `ASTNode`
  structs whose `rule_name` fields mirror the grammar rules (`program`, `line`,
  `statement`, `let_stmt`, `expr`, etc.).

  ## The Grammar at a Glance

  Dartmouth BASIC 1964 has 17 statement types:

  | Statement   | Example                      | Purpose                  |
  |-------------|------------------------------|--------------------------|
  | LET         | `10 LET X = 5`               | Variable assignment      |
  | PRINT       | `10 PRINT X, Y`              | Output to terminal       |
  | INPUT       | `10 INPUT A, B`              | Read values from user    |
  | IF...THEN   | `10 IF X > 0 THEN 100`       | Conditional branch       |
  | GOTO        | `10 GOTO 50`                 | Unconditional jump       |
  | GOSUB       | `10 GOSUB 200`               | Subroutine call          |
  | RETURN      | `200 RETURN`                 | Return from subroutine   |
  | FOR         | `10 FOR I = 1 TO 10`         | Loop start               |
  | NEXT        | `30 NEXT I`                  | Loop end                 |
  | END         | `99 END`                     | Normal termination       |
  | STOP        | `99 STOP`                    | Halt with message        |
  | REM         | `10 REM A COMMENT`           | Remark / comment         |
  | READ        | `10 READ X, Y`               | Read from DATA pool      |
  | DATA        | `20 DATA 1, 2, 3`            | Declare data pool values |
  | RESTORE     | `30 RESTORE`                 | Reset DATA pointer       |
  | DIM         | `10 DIM A(100)`              | Dimension an array       |
  | DEF         | `10 DEF FNA(X) = X * X`      | Define a function        |

  Expressions support:
  - `+`, `-` (left-associative, lowest precedence)
  - `*`, `/` (left-associative, medium precedence)
  - `^` (right-associative exponentiation: `2^3^2` = 512, not 64)
  - unary minus (`-X`)
  - parenthesised subexpressions (`(X + 1)`)
  - built-in functions: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN
  - user-defined functions: FNA through FNZ
  - scalar variables (A–Z, A0–Z9)
  - array elements (A(I))

  ## Pipeline Position

  ```
  BASIC source text  (e.g., "10 LET X = 42\\n20 PRINT X\\n")
        │
        ▼  DartmouthBasicLexer.tokenize/1
  ┌──────────────────────────────────┐
  │   dartmouth_basic_lexer          │
  │   dartmouth_basic.tokens         │
  └──────────────────────────────────┘
        │  [{type, value, line, column}, ...]
        ▼  DartmouthBasicParser.parse/1
  ┌──────────────────────────────────┐
  │   dartmouth_basic_parser         │  ← this package
  │   dartmouth_basic.grammar        │
  └──────────────────────────────────┘
        │  %ASTNode{rule_name: "program", ...}
        ▼
  ┌──────────────────────────────────┐
  │   dartmouth_basic_compiler       │
  └──────────────────────────────────┘
  ```

  ## Usage

      {:ok, ast} = CodingAdventures.DartmouthBasicParser.parse_source("10 PRINT \\"HELLO, WORLD\\"\\n20 END\\n")
      ast.rule_name  # => "program"

      # Or tokenize first, then parse:
      {:ok, tokens} = CodingAdventures.DartmouthBasicLexer.tokenize(source)
      {:ok, ast}    = CodingAdventures.DartmouthBasicParser.parse(tokens)

  ## Grammar Caching

  The `dartmouth_basic.grammar` file is read once and cached in `:persistent_term`
  (a BEAM-level key-value store for "write once, read many" immutable data).
  Unlike ETS (which copies terms on every access), `persistent_term` returns a
  direct reference with zero copy overhead. The first call pays the file I/O cost;
  every subsequent call is effectively free.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.DartmouthBasicLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  # ---------------------------------------------------------------------------
  # Grammar path resolution
  # ---------------------------------------------------------------------------
  #
  # __DIR__ is the compile-time directory of THIS source file:
  #   .../code/packages/elixir/dartmouth_basic_parser/lib/coding_adventures
  #
  # Walking up with ".." five times reaches code/:
  #   (1) lib/coding_adventures  →  lib
  #   (2) lib                    →  dartmouth_basic_parser
  #   (3) dartmouth_basic_parser →  elixir
  #   (4) elixir                 →  packages
  #   (5) packages               →  code
  #   then append "grammars"     →  code/grammars
  #
  # Path.expand/1 resolves all ".." components to an absolute path so that
  # File.read! works regardless of the current working directory at runtime.
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Parse a pre-tokenized list of Dartmouth BASIC tokens into an AST.

  Accepts a list of `%Token{}` structs (as produced by `DartmouthBasicLexer.tokenize/1`)
  and returns an AST rooted at the `"program"` rule.

  ## When to use `parse/1` vs `parse_source/1`

  - Use `parse_source/1` for the common case: raw BASIC source → AST.
  - Use `parse/1` when you have already tokenised the source (e.g., in a
    pipeline where you need to inspect or transform the tokens before parsing).

  ## Return value

  - `{:ok, %ASTNode{rule_name: "program", ...}}` on success.
  - `{:error, message}` if parsing fails (invalid syntax).

  ## Example

      {:ok, tokens} = DartmouthBasicLexer.tokenize("10 LET X = 5\\n")
      {:ok, ast}    = DartmouthBasicParser.parse(tokens)
      ast.rule_name  # => "program"

  """
  @spec parse([CodingAdventures.Lexer.Token.t()]) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(tokens) do
    grammar = get_grammar()
    GrammarParser.parse(tokens, grammar)
  end

  @doc """
  Tokenize Dartmouth BASIC source code and parse it into an AST in one call.

  This is the primary entry point for callers who have raw BASIC source text.
  Internally it calls `DartmouthBasicLexer.tokenize/1` followed by
  `GrammarParser.parse/2`.

  ## Return value

  - `{:ok, %ASTNode{rule_name: "program", ...}}` on success.
  - `{:error, message}` if lexing or parsing fails.

  ## Example

      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT \\"HELLO, WORLD\\"\\n20 END\\n")
      ast.rule_name  # => "program"

  ## Multi-line programs

      # A multi-line program spanning several lines:
      src = "10 FOR I = 1 TO 5\\n20 PRINT I\\n30 NEXT I\\n40 END\\n"
      {:ok, ast} = DartmouthBasicParser.parse_source(src)

  """
  @spec parse_source(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse_source(source) do
    case DartmouthBasicLexer.tokenize(source) do
      {:ok, tokens} -> parse(tokens)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse the `dartmouth_basic.grammar` file and return the `ParserGrammar`.

  Returns a `%ParserGrammar{}` struct containing all grammar rules. You can
  inspect it to enumerate the rules, check alternation order, or verify that
  the grammar loaded correctly. The grammar drives the parser engine in
  `GrammarParser.parse/2`.

  This function always reads fresh from disk — it does NOT return the cached
  copy. Use it for introspection and testing. For production parsing, prefer
  `parse/1` or `parse_source/1`, which use the cached grammar.

  ## Example

      grammar = DartmouthBasicParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      # => ["program", "line", "statement", "let_stmt", ...]

  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    grammar_path = Path.join(@grammars_dir, "dartmouth_basic.grammar")
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    grammar
  end

  # ---------------------------------------------------------------------------
  # Grammar caching (private)
  # ---------------------------------------------------------------------------
  #
  # The grammar is cached in :persistent_term — a BEAM key-value store designed
  # for data that is written once and read many times. Unlike ETS, persistent_term
  # returns a direct reference to the stored term (no copying). This means the
  # grammar struct is shared across all processes with zero overhead.
  #
  # Key: {CodingAdventures.DartmouthBasicParser, :grammar}
  #   — Using the module name as part of the key avoids collisions with other
  #     packages that also use persistent_term (e.g., DartmouthBasicLexer).
  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        # First call: load from disk and cache.
        grammar = create_parser()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        # Subsequent calls: return cached grammar directly (zero copy).
        grammar
    end
  end
end
