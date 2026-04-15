defmodule CodingAdventures.VhdlLexerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.VhdlLexer

  # ===========================================================================
  # Helper Functions
  # ===========================================================================
  #
  # These helpers reduce boilerplate in tests. Instead of pattern-matching
  # on {:ok, tokens} and filtering EOF in every test, we extract common
  # patterns into reusable functions.

  # Tokenize and return only non-EOF tokens (the "meat" of the output).
  defp tokenize!(source) do
    {:ok, tokens} = VhdlLexer.tokenize(source)

    tokens
    |> Enum.reject(&(&1.type == "EOF"))
  end

  # Extract {type, value} pairs from non-EOF tokens.
  defp types_and_values(source) do
    tokenize!(source)
    |> Enum.map(&{&1.type, &1.value})
  end

  # Extract just the token types from non-EOF tokens.
  defp token_types(source) do
    tokenize!(source)
    |> Enum.map(& &1.type)
  end

  # ===========================================================================
  # Grammar Loading
  # ===========================================================================

  describe "create_lexer/0" do
    test "returns a TokenGrammar with expected token definitions" do
      grammar = VhdlLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)

      # Core token types from vhdl.tokens
      assert "NAME" in names
      assert "NUMBER" in names
      assert "REAL_NUMBER" in names
      assert "BASED_LITERAL" in names
      assert "STRING" in names
      assert "BIT_STRING" in names
      assert "CHAR_LITERAL" in names
      assert "EXTENDED_IDENT" in names
      assert "SEMICOLON" in names
      assert "LPAREN" in names
      assert "RPAREN" in names
    end

    test "includes keyword definitions" do
      grammar = VhdlLexer.create_lexer()
      assert length(grammar.keywords) > 0
      assert "entity" in grammar.keywords
      assert "architecture" in grammar.keywords
      assert "signal" in grammar.keywords
      assert "port" in grammar.keywords
      assert "begin" in grammar.keywords
    end

    test "supports selecting an explicit language edition" do
      default_names =
        VhdlLexer.create_lexer()
        |> Map.fetch!(:definitions)
        |> Enum.map(& &1.name)

      versioned_names =
        VhdlLexer.create_lexer("2008")
        |> Map.fetch!(:definitions)
        |> Enum.map(& &1.name)

      assert default_names == versioned_names
    end

    test "raises for an unknown language edition" do
      assert_raise ArgumentError, ~r/Unknown VHDL version/, fn ->
        VhdlLexer.create_lexer("2099")
      end
    end
  end

  # ===========================================================================
  # Entity Declarations
  # ===========================================================================
  #
  # The entity declaration is VHDL's equivalent of a Verilog module header.
  # It declares the interface (ports) of a design unit:
  #
  #   entity counter is
  #     port (
  #       clk : in std_logic;
  #       count : out std_logic_vector(7 downto 0)
  #     );
  #   end counter;

  describe "tokenize/1 — entity declarations" do
    test "tokenizes a simple entity declaration" do
      source = "entity counter is end counter;"
      tv_list = types_and_values(source)

      assert {"KEYWORD", "entity"} in tv_list
      assert {"NAME", "counter"} in tv_list
      assert {"KEYWORD", "is"} in tv_list
      assert {"KEYWORD", "end"} in tv_list
      assert {"SEMICOLON", ";"} in tv_list
    end

    test "tokenizes entity with port declaration" do
      source = """
      entity half_adder is
        port (
          a : in std_logic;
          b : in std_logic;
          sum_out : out std_logic;
          carry : out std_logic
        );
      end half_adder;
      """

      types = token_types(source)

      assert "KEYWORD" in types
      assert "NAME" in types
      assert "LPAREN" in types
      assert "RPAREN" in types
      assert "COLON" in types
      assert "SEMICOLON" in types
    end
  end

  # ===========================================================================
  # Architecture Bodies
  # ===========================================================================
  #
  # The architecture body contains the implementation of an entity.
  # It is the VHDL equivalent of the body of a Verilog module:
  #
  #   architecture behavioral of counter is
  #   begin
  #     -- implementation here
  #   end behavioral;

  describe "tokenize/1 — architecture" do
    test "tokenizes architecture declaration" do
      source = "architecture behavioral of counter is begin end behavioral;"
      tv_list = types_and_values(source)

      assert {"KEYWORD", "architecture"} in tv_list
      assert {"NAME", "behavioral"} in tv_list
      assert {"KEYWORD", "of"} in tv_list
      assert {"NAME", "counter"} in tv_list
      assert {"KEYWORD", "is"} in tv_list
      assert {"KEYWORD", "begin"} in tv_list
      assert {"KEYWORD", "end"} in tv_list
    end

    test "tokenizes signal assignment in architecture" do
      source = "y <= a and b;"
      tv_list = types_and_values(source)

      assert {"NAME", "y"} in tv_list
      assert {"LESS_EQUALS", "<="} in tv_list
      assert {"NAME", "a"} in tv_list
      assert {"KEYWORD", "and"} in tv_list
      assert {"NAME", "b"} in tv_list
      assert {"SEMICOLON", ";"} in tv_list
    end
  end

  # ===========================================================================
  # Case Insensitivity
  # ===========================================================================
  #
  # VHDL is case-insensitive: ENTITY, Entity, and entity are identical.
  # The lexer normalizes all NAME and KEYWORD values to lowercase.
  # This is tested thoroughly because it is a key difference from Verilog.

  describe "tokenize/1 — case insensitivity" do
    test "normalizes uppercase keywords to lowercase" do
      tv_list = types_and_values("ENTITY ARCHITECTURE BEGIN END")

      assert {"KEYWORD", "entity"} in tv_list
      assert {"KEYWORD", "architecture"} in tv_list
      assert {"KEYWORD", "begin"} in tv_list
      assert {"KEYWORD", "end"} in tv_list
    end

    test "normalizes mixed-case keywords to lowercase" do
      tv_list = types_and_values("Entity Architecture Begin End")

      assert {"KEYWORD", "entity"} in tv_list
      assert {"KEYWORD", "architecture"} in tv_list
      assert {"KEYWORD", "begin"} in tv_list
      assert {"KEYWORD", "end"} in tv_list
    end

    test "normalizes uppercase identifiers to lowercase" do
      tv_list = types_and_values("MY_SIGNAL MyCounter DataBus")

      assert {"NAME", "my_signal"} in tv_list
      assert {"NAME", "mycounter"} in tv_list
      assert {"NAME", "databus"} in tv_list
    end

    test "mixed case source produces same tokens as lowercase" do
      {:ok, upper_tokens} = VhdlLexer.tokenize("ENTITY Counter IS END Counter;")
      {:ok, lower_tokens} = VhdlLexer.tokenize("entity counter is end counter;")

      # Compare types and values (ignoring line/column differences)
      upper_tv = Enum.map(upper_tokens, &{&1.type, &1.value})
      lower_tv = Enum.map(lower_tokens, &{&1.type, &1.value})

      assert upper_tv == lower_tv
    end

    test "extended identifiers preserve case" do
      # Extended identifiers (\name\) are case-sensitive in VHDL
      source = "\\MySpecial\\"
      tokens = tokenize!(source)
      ext = Enum.find(tokens, &(&1.type == "EXTENDED_IDENT"))

      assert ext != nil
      assert ext.value == "\\myspecial\\"
    end
  end

  # ===========================================================================
  # Character Literals
  # ===========================================================================
  #
  # VHDL character literals are single characters between tick marks:
  #   '0'  '1'  'X'  'Z'  'U'  'H'  'L'  '-'
  #
  # These represent std_logic values, the fundamental signal type in VHDL.
  # The tick (') also serves as the attribute access operator (signal'length),
  # so ordering in the grammar is critical.

  describe "tokenize/1 — character literals" do
    test "tokenizes std_logic character literals" do
      source = "'0' '1' 'X' 'Z'"
      tokens = tokenize!(source)
      chars = Enum.filter(tokens, &(&1.type == "CHAR_LITERAL"))

      assert length(chars) == 4
      assert Enum.map(chars, & &1.value) == ["'0'", "'1'", "'x'", "'z'"]
    end

    test "tokenizes don't-care character literal" do
      source = "'-'"
      tokens = tokenize!(source)
      [char_tok] = tokens

      assert char_tok.type == "CHAR_LITERAL"
      assert char_tok.value == "'-'"
    end
  end

  # ===========================================================================
  # Bit String Literals
  # ===========================================================================
  #
  # VHDL uses prefix letters to indicate the base of a bit string:
  #   B"1010"   — binary
  #   O"77"     — octal
  #   X"FF"     — hexadecimal
  #   D"42"     — decimal (VHDL-2008)
  #
  # These are the VHDL equivalent of Verilog's sized literals:
  #   Verilog 8'hFF  →  VHDL X"FF"

  describe "tokenize/1 — bit string literals" do
    test "tokenizes binary bit string" do
      tv_list = types_and_values(~s(B"1010"))
      assert {"BIT_STRING", ~s(b"1010")} in tv_list
    end

    test "tokenizes hexadecimal bit string" do
      tv_list = types_and_values(~s(X"FF"))
      assert {"BIT_STRING", ~s(x"ff")} in tv_list
    end

    test "tokenizes octal bit string" do
      tv_list = types_and_values(~s(O"77"))
      assert {"BIT_STRING", ~s(o"77")} in tv_list
    end

    test "tokenizes decimal bit string (VHDL-2008)" do
      tv_list = types_and_values(~s(D"42"))
      assert {"BIT_STRING", ~s(d"42")} in tv_list
    end

    test "tokenizes lowercase prefix bit strings" do
      tv_list = types_and_values(~s(x"AB" b"1100"))
      assert {"BIT_STRING", ~s(x"ab")} in tv_list
      assert {"BIT_STRING", ~s(b"1100")} in tv_list
    end

    test "tokenizes bit string with underscores" do
      tv_list = types_and_values(~s(X"FF_00"))
      assert {"BIT_STRING", ~s(x"ff_00")} in tv_list
    end
  end

  # ===========================================================================
  # Number Literals
  # ===========================================================================
  #
  # VHDL supports three kinds of numeric literals:
  #
  # 1. Plain integers: 42, 1_000_000
  # 2. Real numbers: 3.14, 1.0E-3
  # 3. Based literals: 16#FF#, 2#1010#, 8#77#
  #
  # Based literals use a unique syntax: base#digits#[exponent]
  # The base is a decimal number (2–16), and digits are enclosed in # marks.

  describe "tokenize/1 — number literals" do
    test "tokenizes plain integers" do
      tv_list = types_and_values("42")
      assert {"NUMBER", "42"} in tv_list
    end

    test "tokenizes integers with underscores" do
      tv_list = types_and_values("1_000_000")
      assert {"NUMBER", "1_000_000"} in tv_list
    end

    test "tokenizes real numbers" do
      tv_list = types_and_values("3.14")
      assert {"REAL_NUMBER", "3.14"} in tv_list
    end

    test "tokenizes real numbers with exponent" do
      tv_list = types_and_values("1.5e10")
      assert {"REAL_NUMBER", "1.5e10"} in tv_list
    end

    test "tokenizes based literals (hexadecimal)" do
      tv_list = types_and_values("16#FF#")
      assert {"BASED_LITERAL", "16#ff#"} in tv_list
    end

    test "tokenizes based literals (binary)" do
      tv_list = types_and_values("2#1010#")
      assert {"BASED_LITERAL", "2#1010#"} in tv_list
    end

    test "tokenizes based literals (octal)" do
      tv_list = types_and_values("8#77#")
      assert {"BASED_LITERAL", "8#77#"} in tv_list
    end
  end

  # ===========================================================================
  # String Literals
  # ===========================================================================
  #
  # VHDL strings are double-quoted. To include a quote character inside
  # a string, you double it: "He said ""hello""" → He said "hello"
  #
  # Note: The grammar specifies escapes: none, so no backslash processing.
  # The doubled-quote escaping is handled by the regex itself.

  describe "tokenize/1 — strings" do
    test "tokenizes a simple string" do
      tv_list = types_and_values(~s("hello"))
      assert {"STRING", "hello"} in tv_list
    end

    test "tokenizes a string with doubled quotes" do
      # VHDL uses "" for escaped quotes: "He said ""hi"""
      source = ~s("He said ""hi""")
      tv_list = types_and_values(source)

      string_tokens = Enum.filter(tv_list, fn {type, _val} -> type == "STRING" end)
      assert length(string_tokens) == 1
    end
  end

  # ===========================================================================
  # Operators
  # ===========================================================================
  #
  # VHDL operators differ significantly from Verilog:
  #
  #   - Logical operations (and, or, xor, etc.) are keywords, not symbols
  #   - := is variable assignment (Verilog uses =)
  #   - <= is signal assignment (Verilog uses <= for non-blocking)
  #   - /= is "not equal" (Verilog uses !=)
  #   - => is the association arrow (used in port maps)
  #   - ** is exponentiation
  #   - <> is the "box" (unconstrained range)
  #   - & is concatenation (Verilog uses {a, b})

  describe "tokenize/1 — operators" do
    test "tokenizes two-character operators" do
      source = ":= <= >= => /= ** <>"
      types = token_types(source)

      assert "VAR_ASSIGN" in types
      assert "LESS_EQUALS" in types
      assert "GREATER_EQUALS" in types
      assert "ARROW" in types
      assert "NOT_EQUALS" in types
      assert "POWER" in types
      assert "BOX" in types
    end

    test "tokenizes single-character operators" do
      source = "+ - * / & < > = '"
      types = token_types(source)

      assert "PLUS" in types
      assert "MINUS" in types
      assert "STAR" in types
      assert "SLASH" in types
      assert "AMPERSAND" in types
      assert "LESS_THAN" in types
      assert "GREATER_THAN" in types
      assert "EQUALS" in types
      assert "TICK" in types
    end

    test "tokenizes pipe operator" do
      types = token_types("|")
      assert "PIPE" in types
    end
  end

  # ===========================================================================
  # Delimiters
  # ===========================================================================

  describe "tokenize/1 — delimiters" do
    test "tokenizes all delimiters" do
      source = "( ) [ ] ; , . :"
      types = token_types(source)

      assert types == [
               "LPAREN", "RPAREN", "LBRACKET", "RBRACKET",
               "SEMICOLON", "COMMA", "DOT", "COLON"
             ]
    end
  end

  # ===========================================================================
  # Keywords
  # ===========================================================================
  #
  # VHDL has a large keyword set. Many concepts that would be operators
  # in other languages are keywords in VHDL:
  #
  #   Logical: and, or, xor, nand, nor, xnor, not
  #   Shift:   sll, srl, sla, sra, rol, ror
  #   Arith:   mod, rem, abs

  describe "tokenize/1 — keywords" do
    test "recognizes structural keywords" do
      tv_list = types_and_values("entity architecture component port generic")

      assert {"KEYWORD", "entity"} in tv_list
      assert {"KEYWORD", "architecture"} in tv_list
      assert {"KEYWORD", "component"} in tv_list
      assert {"KEYWORD", "port"} in tv_list
      assert {"KEYWORD", "generic"} in tv_list
    end

    test "recognizes logical operator keywords" do
      tv_list = types_and_values("and or xor nand nor xnor not")

      assert {"KEYWORD", "and"} in tv_list
      assert {"KEYWORD", "or"} in tv_list
      assert {"KEYWORD", "xor"} in tv_list
      assert {"KEYWORD", "nand"} in tv_list
      assert {"KEYWORD", "nor"} in tv_list
      assert {"KEYWORD", "xnor"} in tv_list
      assert {"KEYWORD", "not"} in tv_list
    end

    test "recognizes shift operator keywords" do
      tv_list = types_and_values("sll srl sla sra rol ror")

      assert {"KEYWORD", "sll"} in tv_list
      assert {"KEYWORD", "srl"} in tv_list
      assert {"KEYWORD", "sla"} in tv_list
      assert {"KEYWORD", "sra"} in tv_list
      assert {"KEYWORD", "rol"} in tv_list
      assert {"KEYWORD", "ror"} in tv_list
    end

    test "recognizes control flow keywords" do
      tv_list = types_and_values("if then else elsif case when loop for while")

      assert {"KEYWORD", "if"} in tv_list
      assert {"KEYWORD", "then"} in tv_list
      assert {"KEYWORD", "else"} in tv_list
      assert {"KEYWORD", "elsif"} in tv_list
      assert {"KEYWORD", "case"} in tv_list
      assert {"KEYWORD", "when"} in tv_list
      assert {"KEYWORD", "loop"} in tv_list
      assert {"KEYWORD", "for"} in tv_list
      assert {"KEYWORD", "while"} in tv_list
    end

    test "recognizes type-related keywords" do
      tv_list = types_and_values("signal variable constant type subtype array record")

      assert {"KEYWORD", "signal"} in tv_list
      assert {"KEYWORD", "variable"} in tv_list
      assert {"KEYWORD", "constant"} in tv_list
      assert {"KEYWORD", "type"} in tv_list
      assert {"KEYWORD", "subtype"} in tv_list
      assert {"KEYWORD", "array"} in tv_list
      assert {"KEYWORD", "record"} in tv_list
    end

    test "recognizes arithmetic keywords" do
      tv_list = types_and_values("mod rem abs")

      assert {"KEYWORD", "mod"} in tv_list
      assert {"KEYWORD", "rem"} in tv_list
      assert {"KEYWORD", "abs"} in tv_list
    end
  end

  # ===========================================================================
  # Comments
  # ===========================================================================
  #
  # VHDL uses double-dash (--) for single-line comments. There are no
  # block comments in the base VHDL standard (VHDL-2008 adds /* */ but
  # we target the core language).

  describe "tokenize/1 — comments" do
    test "skips single-line comments" do
      source = "signal clk -- this is the clock\n: in std_logic;"
      types = token_types(source)

      # The comment should be stripped; only signal, clk, :, in, etc. remain
      assert "KEYWORD" in types
      assert "NAME" in types
      refute "COMMENT" in types
    end

    test "comment at end of source" do
      source = "entity foo is -- trailing comment"
      tv_list = types_and_values(source)

      assert {"KEYWORD", "entity"} in tv_list
      assert {"NAME", "foo"} in tv_list
      assert {"KEYWORD", "is"} in tv_list
    end
  end

  # ===========================================================================
  # Whitespace
  # ===========================================================================

  describe "tokenize/1 — whitespace" do
    test "skips whitespace between tokens" do
      source = "  entity   counter  ;  "
      types = token_types(source)

      assert types == ["KEYWORD", "NAME", "SEMICOLON"]
    end
  end

  # ===========================================================================
  # Position Tracking
  # ===========================================================================

  describe "tokenize/1 — position tracking" do
    test "tracks line and column of first token" do
      {:ok, tokens} = VhdlLexer.tokenize("entity foo;")
      [first | _rest] = tokens

      assert first.line == 1
      assert first.column == 1
    end

    test "tracks column position of subsequent tokens" do
      {:ok, tokens} = VhdlLexer.tokenize("entity foo")
      non_eof = Enum.reject(tokens, &(&1.type == "EOF"))
      [kw, name] = non_eof

      # "entity" starts at column 1, "foo" starts at column 8
      assert kw.column == 1
      assert name.column == 8
    end
  end

  # ===========================================================================
  # Error Cases
  # ===========================================================================

  describe "tokenize/1 — errors" do
    test "errors on unexpected character" do
      {:error, msg} = VhdlLexer.tokenize("\x01")
      assert msg =~ "Unexpected character"
    end
  end

  # ===========================================================================
  # Complete Snippets
  # ===========================================================================
  #
  # These tests verify that larger, realistic VHDL snippets tokenize
  # correctly end-to-end. They exercise multiple token types in
  # combination, as they would appear in real code.

  describe "tokenize/1 — complete snippets" do
    test "tokenizes a half adder entity and architecture" do
      source = """
      entity half_adder is
        port (
          a, b : in std_logic;
          sum_out, carry : out std_logic
        );
      end half_adder;

      architecture dataflow of half_adder is
      begin
        sum_out <= a xor b;
        carry <= a and b;
      end dataflow;
      """

      {:ok, tokens} = VhdlLexer.tokenize(source)

      # Verify structural keywords are present
      keyword_vals =
        tokens
        |> Enum.filter(&(&1.type == "KEYWORD"))
        |> Enum.map(& &1.value)

      assert "entity" in keyword_vals
      assert "architecture" in keyword_vals
      assert "port" in keyword_vals
      assert "begin" in keyword_vals
      assert "end" in keyword_vals
      assert "xor" in keyword_vals
      assert "and" in keyword_vals

      # Verify identifiers are lowercased
      name_vals =
        tokens
        |> Enum.filter(&(&1.type == "NAME"))
        |> Enum.map(& &1.value)

      assert "half_adder" in name_vals
      assert "sum_out" in name_vals
      assert "carry" in name_vals
      assert "std_logic" in name_vals
      assert "dataflow" in name_vals
    end

    test "tokenizes a process with case statement" do
      source = """
      process (sel)
      begin
        case sel is
          when "00" => y <= a;
          when "01" => y <= b;
          when others => y <= '0';
        end case;
      end process;
      """

      {:ok, tokens} = VhdlLexer.tokenize(source)

      keyword_vals =
        tokens
        |> Enum.filter(&(&1.type == "KEYWORD"))
        |> Enum.map(& &1.value)

      assert "process" in keyword_vals
      assert "begin" in keyword_vals
      assert "case" in keyword_vals
      assert "when" in keyword_vals
      assert "others" in keyword_vals
      assert "end" in keyword_vals

      # Verify string literals are present
      string_vals =
        tokens
        |> Enum.filter(&(&1.type == "STRING"))
        |> Enum.map(& &1.value)

      assert "00" in string_vals
      assert "01" in string_vals

      # Verify character literal
      char_vals =
        tokens
        |> Enum.filter(&(&1.type == "CHAR_LITERAL"))
        |> Enum.map(& &1.value)

      assert "'0'" in char_vals

      # Verify arrow operator
      arrow_types = Enum.filter(tokens, &(&1.type == "ARROW"))
      assert length(arrow_types) >= 3
    end

    test "tokenizes a signal declaration with based literal" do
      source = "constant MASK : std_logic_vector(7 downto 0) := X\"FF\";"

      {:ok, tokens} = VhdlLexer.tokenize(source)
      tv_list =
        tokens
        |> Enum.reject(&(&1.type == "EOF"))
        |> Enum.map(&{&1.type, &1.value})

      assert {"KEYWORD", "constant"} in tv_list
      assert {"NAME", "mask"} in tv_list
      assert {"KEYWORD", "downto"} in tv_list
      assert {"NUMBER", "7"} in tv_list
      assert {"NUMBER", "0"} in tv_list
      assert {"VAR_ASSIGN", ":="} in tv_list
      assert {"BIT_STRING", ~s(x"ff")} in tv_list
    end

    test "tokenizes component instantiation with port map" do
      source = """
      U1 : half_adder port map (
        a => x,
        b => y,
        sum_out => s,
        carry => c
      );
      """

      {:ok, tokens} = VhdlLexer.tokenize(source)

      keyword_vals =
        tokens
        |> Enum.filter(&(&1.type == "KEYWORD"))
        |> Enum.map(& &1.value)

      assert "port" in keyword_vals
      assert "map" in keyword_vals

      # Verify arrow operators for port associations
      arrows = Enum.filter(tokens, &(&1.type == "ARROW"))
      assert length(arrows) == 4
    end

    test "tokenizes generate statement with for loop" do
      source = """
      gen: for i in 0 to 7 generate
        u: inv port map (a => data(i), y => result(i));
      end generate gen;
      """

      {:ok, tokens} = VhdlLexer.tokenize(source)

      keyword_vals =
        tokens
        |> Enum.filter(&(&1.type == "KEYWORD"))
        |> Enum.map(& &1.value)

      assert "for" in keyword_vals
      assert "in" in keyword_vals
      assert "to" in keyword_vals
      assert "generate" in keyword_vals
    end
  end
end
