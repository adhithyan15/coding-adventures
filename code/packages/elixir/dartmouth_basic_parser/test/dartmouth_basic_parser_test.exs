defmodule CodingAdventures.DartmouthBasicParserTest do
  use ExUnit.Case, async: true

  # ============================================================================
  # Test module for CodingAdventures.DartmouthBasicParser
  #
  # These tests exercise the full parsing pipeline: BASIC source text →
  # DartmouthBasicLexer.tokenize/1 → DartmouthBasicParser.parse/1 → AST.
  #
  # All tests call `parse_source/1` unless they specifically need the token-
  # level `parse/1` API. Every successful parse must produce an AST whose root
  # rule_name is "program" — that is the top-level rule in dartmouth_basic.grammar.
  #
  # Test organisation
  # -----------------
  #   create_parser/0      — grammar loading
  #   LET statement        — assignment
  #   PRINT statement      — output (bare, expr, string, comma, semicolon)
  #   INPUT statement      — single and multiple variables
  #   IF statement         — all six relational operators
  #   GOTO / GOSUB / RETURN
  #   FOR / NEXT           — with and without STEP
  #   END / STOP / REM
  #   READ / DATA / RESTORE
  #   DIM / DEF
  #   Expressions          — precedence, right-associativity, unary minus
  #   Multi-line programs  — HELLO WORLD, counted FOR loop
  #   Error cases          — missing =, missing THEN, incomplete FOR
  #   Edge case            — bare line number
  # ============================================================================

  alias CodingAdventures.DartmouthBasicParser

  # ---------------------------------------------------------------------------
  # create_parser/0
  # ---------------------------------------------------------------------------
  #
  # Verify that the grammar loads correctly and contains the expected top-level
  # rules. dartmouth_basic.grammar defines these rules at the top level:
  # program, line, statement, let_stmt, print_stmt, input_stmt, if_stmt,
  # goto_stmt, gosub_stmt, return_stmt, for_stmt, next_stmt, end_stmt,
  # stop_stmt, rem_stmt, read_stmt, data_stmt, restore_stmt, dim_stmt,
  # def_stmt, variable, relop, print_list, print_item, print_sep,
  # expr, term, power, unary, primary, dim_decl.

  describe "create_parser/0" do
    test "returns a ParserGrammar containing the expected core rules" do
      grammar = DartmouthBasicParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)

      # Top-level structural rules
      assert "program" in rule_names
      assert "line" in rule_names
      assert "statement" in rule_names

      # Statement rules — one for each of the 17 statement types
      assert "let_stmt" in rule_names
      assert "print_stmt" in rule_names
      assert "input_stmt" in rule_names
      assert "if_stmt" in rule_names
      assert "goto_stmt" in rule_names
      assert "gosub_stmt" in rule_names
      assert "return_stmt" in rule_names
      assert "for_stmt" in rule_names
      assert "next_stmt" in rule_names
      assert "end_stmt" in rule_names
      assert "stop_stmt" in rule_names
      assert "rem_stmt" in rule_names
      assert "read_stmt" in rule_names
      assert "data_stmt" in rule_names
      assert "restore_stmt" in rule_names
      assert "dim_stmt" in rule_names
      assert "def_stmt" in rule_names

      # Expression hierarchy rules
      assert "expr" in rule_names
      assert "term" in rule_names
      assert "power" in rule_names
      assert "unary" in rule_names
      assert "primary" in rule_names

      # Auxiliary rules
      assert "variable" in rule_names
      assert "relop" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # LET statement
  # ---------------------------------------------------------------------------
  #
  # The LET statement assigns a value to a variable:
  #   LET variable = expr
  #
  # In Dartmouth BASIC the "=" in LET is always assignment, never comparison.
  # Comparison only appears in IF statements (using the relop rule).

  describe "LET statement" do
    test "parses simple LET assignment" do
      # 10 LET X = 5
      # The most basic LET: assign a literal number to a scalar variable.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = 5\n")
      assert ast.rule_name == "program"
    end

    test "parses LET with expression on right-hand side" do
      # 10 LET X = Y + 1
      # The right-hand side can be any arithmetic expression.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = Y + 1\n")
      assert ast.rule_name == "program"
    end

    test "parses LET assigning to array element" do
      # 10 LET A(3) = 42
      # The grammar's `variable` rule covers both scalar (NAME) and array
      # element (NAME LPAREN expr RPAREN) forms.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET A(3) = 42\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # PRINT statement
  # ---------------------------------------------------------------------------
  #
  # PRINT is the most flexible statement in Dartmouth BASIC. It can:
  #   - Print nothing (bare PRINT → blank line)
  #   - Print an expression
  #   - Print a quoted string literal
  #   - Print multiple items separated by COMMA (zone-aligned) or SEMICOLON
  #     (no space between items)
  #   - End with a trailing separator (suppresses final newline at runtime)

  describe "PRINT statement" do
    test "parses bare PRINT" do
      # 10 PRINT
      # A PRINT with no arguments emits a blank line.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT\n")
      assert ast.rule_name == "program"
    end

    test "parses PRINT with expression" do
      # 10 PRINT X + 1
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT X + 1\n")
      assert ast.rule_name == "program"
    end

    test "parses PRINT with string literal" do
      # 10 PRINT "HELLO"
      # STRING tokens are produced by the lexer (double-quoted literals).
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT \"HELLO\"\n")
      assert ast.rule_name == "program"
    end

    test "parses PRINT with comma-separated items" do
      # 10 PRINT X, Y
      # COMMA in PRINT output advances to the next tab zone (every 15 chars
      # on the original Teletype ASR-33). Zones make columnar output easy.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT X, Y\n")
      assert ast.rule_name == "program"
    end

    test "parses PRINT with semicolon-separated items" do
      # 10 PRINT X; Y
      # SEMICOLON in PRINT output suppresses spacing: items are printed
      # immediately adjacent to each other.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT X; Y\n")
      assert ast.rule_name == "program"
    end

    test "parses PRINT with trailing comma" do
      # 10 PRINT X,
      # A trailing COMMA or SEMICOLON suppresses the final newline at runtime,
      # so the next PRINT continues on the same output line.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT X,\n")
      assert ast.rule_name == "program"
    end

    test "parses PRINT with mixed string and expression" do
      # 10 PRINT "VALUE=", X
      {:ok, ast} = DartmouthBasicParser.parse_source("10 PRINT \"VALUE=\", X\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # INPUT statement
  # ---------------------------------------------------------------------------
  #
  # INPUT reads values from the user at runtime:
  #   10 INPUT X
  #   20 INPUT A, B, C
  #
  # The runtime prints a "?" prompt, reads a line, and assigns values to the
  # listed variables in order. The 1964 spec does not allow a prompt string
  # before the variable list — that came in later dialects.

  describe "INPUT statement" do
    test "parses INPUT with single variable" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 INPUT X\n")
      assert ast.rule_name == "program"
    end

    test "parses INPUT with multiple variables" do
      # 10 INPUT A, B, C
      # The grammar uses `variable { COMMA variable }` for the list.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 INPUT A, B, C\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # IF statement — all six relational operators
  # ---------------------------------------------------------------------------
  #
  # The 1964 IF statement has exactly one form:
  #   IF expr relop expr THEN LINE_NUM
  #
  # There is no ELSE clause. The branch target must be a literal line number.
  # The six relational operators are: = < > <= >= <>
  #
  # Note: = is both assignment (in LET) and equality comparison (in IF).
  # The parser knows which one is which from context — LET has EQ immediately
  # after the variable, while IF has EQ in the relop position.

  describe "IF statement" do
    test "parses IF with equals relop" do
      # IF X = Y THEN 50
      {:ok, ast} = DartmouthBasicParser.parse_source("10 IF X = Y THEN 50\n")
      assert ast.rule_name == "program"
    end

    test "parses IF with less-than relop" do
      # IF X < 0 THEN 100
      {:ok, ast} = DartmouthBasicParser.parse_source("10 IF X < 0 THEN 100\n")
      assert ast.rule_name == "program"
    end

    test "parses IF with greater-than relop" do
      # IF X > 0 THEN 100
      {:ok, ast} = DartmouthBasicParser.parse_source("10 IF X > 0 THEN 100\n")
      assert ast.rule_name == "program"
    end

    test "parses IF with less-than-or-equal relop" do
      # IF X <= 10 THEN 200
      {:ok, ast} = DartmouthBasicParser.parse_source("10 IF X <= 10 THEN 200\n")
      assert ast.rule_name == "program"
    end

    test "parses IF with greater-than-or-equal relop" do
      # IF X >= 10 THEN 200
      {:ok, ast} = DartmouthBasicParser.parse_source("10 IF X >= 10 THEN 200\n")
      assert ast.rule_name == "program"
    end

    test "parses IF with not-equal relop" do
      # IF X <> 0 THEN 300
      # <> is the BASIC not-equal operator (some later dialects use #).
      {:ok, ast} = DartmouthBasicParser.parse_source("10 IF X <> 0 THEN 300\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # GOTO statement
  # ---------------------------------------------------------------------------
  #
  # GOTO jumps unconditionally to a named line:
  #   10 GOTO 50
  #
  # In the original Dartmouth BASIC, GOTO was spelled as one word ("GOTO").
  # Some later dialects used "GO TO" (two words), but the 1964 spec uses GOTO.

  describe "GOTO statement" do
    test "parses GOTO with a line number" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 GOTO 50\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # GOSUB and RETURN statements
  # ---------------------------------------------------------------------------
  #
  # GOSUB/RETURN implements subroutines:
  #   10 GOSUB 200       — push return address, jump to line 200
  #   ...
  #   200 PRINT "IN SUB"
  #   210 RETURN         — pop return address, resume at line 20
  #
  # BASIC does not have function parameters — subroutines communicate via
  # global variables. This is a limitation of the 1964 design.

  describe "GOSUB and RETURN statements" do
    test "parses GOSUB with a line number" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 GOSUB 200\n")
      assert ast.rule_name == "program"
    end

    test "parses RETURN statement" do
      {:ok, ast} = DartmouthBasicParser.parse_source("200 RETURN\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # FOR / NEXT loop
  # ---------------------------------------------------------------------------
  #
  # The FOR / NEXT loop counts from a start value to a limit:
  #
  #   10 FOR I = 1 TO 10      — I goes 1, 2, 3, ..., 10 (STEP defaults to 1)
  #   20   PRINT I
  #   30 NEXT I
  #
  #   10 FOR I = 10 TO 1 STEP -1   — I goes 10, 9, 8, ..., 1
  #   20   PRINT I
  #   30 NEXT I
  #
  # The loop variable must be a plain NAME (not an array element).
  # The loop body can contain any statements, including nested FOR/NEXT loops.
  # NEXT ends the loop; the parser just records which variable it names —
  # the compiler/VM enforces matching.

  describe "FOR / NEXT loop" do
    test "parses FOR / NEXT loop without STEP" do
      # Default step is 1 when STEP is omitted.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 FOR I = 1 TO 10\n20 NEXT I\n")
      assert ast.rule_name == "program"
    end

    test "parses FOR / NEXT loop with positive STEP" do
      # 10 FOR I = 0 TO 100 STEP 5  — I goes 0, 5, 10, ..., 100
      {:ok, ast} = DartmouthBasicParser.parse_source("10 FOR I = 0 TO 100 STEP 5\n20 NEXT I\n")
      assert ast.rule_name == "program"
    end

    test "parses FOR / NEXT loop with negative STEP" do
      # 10 FOR I = 10 TO 1 STEP -1  — countdown from 10 to 1
      {:ok, ast} = DartmouthBasicParser.parse_source("10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # END and STOP statements
  # ---------------------------------------------------------------------------
  #
  # END is the normal program-termination statement. Every correct BASIC
  # program ends with END (though the runtime can stop silently at EOF too).
  #
  # STOP halts the program mid-execution and prints "STOP IN LINE n" in the
  # original DTSS system. It is useful for debugging (like a breakpoint).
  # In our VM, both END and STOP terminate execution.

  describe "END and STOP statements" do
    test "parses END statement" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 END\n")
      assert ast.rule_name == "program"
    end

    test "parses STOP statement" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 STOP\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # REM statement
  # ---------------------------------------------------------------------------
  #
  # REM introduces a remark (comment) extending to end of line:
  #   10 REM THIS IS IGNORED BY THE INTERPRETER
  #
  # The lexer's `suppress_rem_content` hook strips everything between REM and
  # the following NEWLINE, so by the time the parser sees the token stream, a
  # REM line looks like: LINE_NUM("10") KEYWORD("REM") NEWLINE("\n").
  # The `rem_stmt` grammar rule therefore has an empty body.

  describe "REM statement" do
    test "parses REM comment" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 REM A COMMENT\n")
      assert ast.rule_name == "program"
    end

    test "parses REM with complex comment text" do
      # Even a long comment with operators and numbers is suppressed by the lexer.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 REM INITIALISE VARIABLES X Y Z TO ZERO\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # READ / DATA / RESTORE statements
  # ---------------------------------------------------------------------------
  #
  # Dartmouth BASIC uses a data pool for static data:
  #
  #   10 DATA 1, 2, 3, 4, 5   — declare values in the pool (in line-order)
  #   20 READ X               — pop the next value (1) and assign to X
  #   30 READ Y, Z            — pop 2 and 3, assign to Y and Z
  #   40 RESTORE              — reset pool pointer to the beginning
  #   50 READ X               — pops 1 again
  #
  # DATA values are numeric literals only (the 1964 spec has no string
  # variables, so strings in DATA are not in the original grammar).

  describe "READ / DATA / RESTORE statements" do
    test "parses READ with single variable" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 READ X\n")
      assert ast.rule_name == "program"
    end

    test "parses READ with multiple variables" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 READ A, B, C\n")
      assert ast.rule_name == "program"
    end

    test "parses DATA with single value" do
      {:ok, ast} = DartmouthBasicParser.parse_source("20 DATA 42\n")
      assert ast.rule_name == "program"
    end

    test "parses DATA with multiple values" do
      {:ok, ast} = DartmouthBasicParser.parse_source("20 DATA 1, 2, 3, 4, 5\n")
      assert ast.rule_name == "program"
    end

    test "parses RESTORE statement" do
      {:ok, ast} = DartmouthBasicParser.parse_source("30 RESTORE\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # DIM statement
  # ---------------------------------------------------------------------------
  #
  # DIM declares array dimensions. Without DIM, arrays default to size 10
  # (indices 0 through 10, giving 11 slots). DIM allows larger arrays:
  #
  #   10 DIM A(100)       — array A can hold indices 0 through 100
  #   20 DIM A(10), B(20) — multiple arrays in one DIM

  describe "DIM statement" do
    test "parses DIM with single array" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 DIM A(10)\n")
      assert ast.rule_name == "program"
    end

    test "parses DIM with multiple arrays" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 DIM A(10), B(20), C(100)\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # DEF statement
  # ---------------------------------------------------------------------------
  #
  # DEF defines a user-created function with a single parameter:
  #
  #   10 DEF FNA(X) = X * X
  #   20 DEF FNB(T) = SIN(T) / COS(T)
  #
  # Function names are FNA through FNZ (26 possible user functions).
  # The formal parameter (here X or T) is a single-letter NAME.
  # The body is any arithmetic expression — it may reference the formal
  # parameter and global variables, but not other user functions (no recursion).

  describe "DEF statement" do
    test "parses DEF with simple expression body" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 DEF FNA(X) = X * X\n")
      assert ast.rule_name == "program"
    end

    test "parses DEF with complex expression body" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 DEF FNB(T) = SIN(T) / COS(T)\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # Expression tests — precedence and associativity
  # ---------------------------------------------------------------------------
  #
  # Dartmouth BASIC uses standard mathematical precedence, encoded as a
  # rule-nesting cascade in the grammar:
  #
  #   expr  (lowest:  + −)
  #     └── term (mid:    * /)
  #           └── power (high:   ^ right-assoc)
  #                 └── unary (−)
  #                       └── primary (atoms)
  #
  # "Higher in the cascade = tighter binding."
  # So `2 + 3 * 4` parses as `2 + (3 * 4)` = 14, not `(2+3) * 4` = 20.
  # And `2 ^ 3 ^ 2` parses as `2 ^ (3 ^ 2)` = 512, not `(2^3)^2` = 64.

  describe "expression precedence and associativity" do
    test "parses addition" do
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = 2 + 3\n")
      assert ast.rule_name == "program"
    end

    test "parses multiplication binding tighter than addition" do
      # `2 + 3 * 4` should parse as `2 + (3 * 4)`.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = 2 + 3 * 4\n")
      assert ast.rule_name == "program"
    end

    test "parses right-associative exponentiation" do
      # `2 ^ 3 ^ 2` should parse as `2 ^ (3 ^ 2)` = 512.
      # The grammar rule is: power = unary [ CARET power ]
      # The self-reference on the right ensures right-to-left grouping.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = 2 ^ 3 ^ 2\n")
      assert ast.rule_name == "program"
    end

    test "parses unary minus" do
      # -Y is the negation of Y. Unary plus is not in the 1964 spec.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = -Y\n")
      assert ast.rule_name == "program"
    end

    test "parses parenthesised sub-expression" do
      # (2 + 3) * 4 = 20 — parentheses override the default precedence.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = (2 + 3) * 4\n")
      assert ast.rule_name == "program"
    end

    test "parses built-in function call" do
      # SIN(Y) — one of the 11 built-in math functions.
      # The grammar rule is: primary = BUILTIN_FN LPAREN expr RPAREN | ...
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = SIN(Y)\n")
      assert ast.rule_name == "program"
    end

    test "parses user-defined function call" do
      # FNA(X) — calls the user function defined with DEF FNA(X) = ...
      # USER_FN is a distinct token type produced by the lexer.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET Y = FNA(X)\n")
      assert ast.rule_name == "program"
    end

    test "parses array element access in expression" do
      # A(3) — array element access via the variable rule's first alternative.
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET X = A(3)\n")
      assert ast.rule_name == "program"
    end

    test "parses complex nested expression" do
      # SIN(X) ^ 2 + COS(X) ^ 2 = 1  (Pythagorean identity)
      {:ok, ast} = DartmouthBasicParser.parse_source("10 LET R = SIN(X) ^ 2 + COS(X) ^ 2\n")
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # Multi-line programs
  # ---------------------------------------------------------------------------
  #
  # Real BASIC programs span multiple lines. The grammar's top-level rule is:
  #   program = { line }
  # which matches zero or more lines. These tests verify that multi-line
  # programs parse correctly as a whole.

  describe "multi-line programs" do
    test "parses Hello World program" do
      source = "10 PRINT \"HELLO, WORLD\"\n20 END\n"
      {:ok, ast} = DartmouthBasicParser.parse_source(source)
      assert ast.rule_name == "program"
    end

    test "parses FOR loop program" do
      # Classic counted loop — prints integers 1 through 5.
      source = """
      10 FOR I = 1 TO 5
      20 PRINT I
      30 NEXT I
      40 END
      """
      {:ok, ast} = DartmouthBasicParser.parse_source(source)
      assert ast.rule_name == "program"
    end

    test "parses a program using GOSUB / RETURN" do
      source = """
      10 GOSUB 100
      20 END
      100 PRINT "IN SUBROUTINE"
      110 RETURN
      """
      {:ok, ast} = DartmouthBasicParser.parse_source(source)
      assert ast.rule_name == "program"
    end

    test "parses program with READ / DATA" do
      source = """
      10 DATA 1, 2, 3
      20 READ A
      30 READ B
      40 READ C
      50 PRINT A + B + C
      60 END
      """
      {:ok, ast} = DartmouthBasicParser.parse_source(source)
      assert ast.rule_name == "program"
    end

    test "parses program with conditional branch" do
      source = """
      10 INPUT X
      20 IF X > 0 THEN 50
      30 PRINT "NEGATIVE OR ZERO"
      40 GOTO 60
      50 PRINT "POSITIVE"
      60 END
      """
      {:ok, ast} = DartmouthBasicParser.parse_source(source)
      assert ast.rule_name == "program"
    end

    test "parses complete example program with all common features" do
      # A program that exercises LET, FOR, NEXT, PRINT, IF, GOTO, END.
      source = """
      10 REM COMPUTE SUM 1 TO N
      20 INPUT N
      30 LET S = 0
      40 FOR I = 1 TO N
      50 LET S = S + I
      60 NEXT I
      70 PRINT "SUM =", S
      80 END
      """
      {:ok, ast} = DartmouthBasicParser.parse_source(source)
      assert ast.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------
  #
  # The parser should return {:error, message} for syntactically invalid BASIC.
  # We test three representative error cases:
  #   1. LET missing the = sign:        10 LET X 5
  #   2. IF missing the THEN keyword:   10 IF X > 0 100
  #   3. FOR missing the TO keyword:    10 FOR I = 1   (incomplete)

  describe "error cases" do
    test "returns error for LET missing =" do
      # `10 LET X 5` is missing the `=` sign required by let_stmt grammar rule.
      result = DartmouthBasicParser.parse_source("10 LET X 5\n")
      assert {:error, _msg} = result
    end

    test "returns error for IF missing THEN" do
      # `10 IF X > 0 100` is missing the THEN keyword.
      result = DartmouthBasicParser.parse_source("10 IF X > 0 100\n")
      assert {:error, _msg} = result
    end

    test "returns error for FOR missing TO" do
      # `10 FOR I = 1` is missing the TO keyword and limit expression.
      result = DartmouthBasicParser.parse_source("10 FOR I = 1\n")
      assert {:error, _msg} = result
    end
  end

  # ---------------------------------------------------------------------------
  # Edge cases
  # ---------------------------------------------------------------------------

  describe "edge cases" do
    test "parses empty program (no lines)" do
      # An empty string is a valid BASIC program — zero lines.
      # The grammar: program = { line }  — zero repetitions is fine.
      {:ok, ast} = DartmouthBasicParser.parse_source("")
      assert ast.rule_name == "program"
    end

    test "parses bare line number (no statement)" do
      # `10\n` — a line number with no statement body.
      # The grammar: line = LINE_NUM [ statement ] NEWLINE
      # The [ statement ] is optional, so a bare line is valid.
      # In the original DTSS BASIC, typing a bare line number deleted that line.
      # In a stored program, it produces a no-op node.
      {:ok, ast} = DartmouthBasicParser.parse_source("10\n")
      assert ast.rule_name == "program"
    end

    test "parse/1 accepts pre-tokenized input" do
      # Verify the two-step API: tokenize first, then parse.
      alias CodingAdventures.DartmouthBasicLexer
      {:ok, tokens} = DartmouthBasicLexer.tokenize("10 LET X = 5\n")
      {:ok, ast} = DartmouthBasicParser.parse(tokens)
      assert ast.rule_name == "program"
    end
  end
end
