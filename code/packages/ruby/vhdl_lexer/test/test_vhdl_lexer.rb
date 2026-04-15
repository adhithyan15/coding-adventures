# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the VHDL Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with vhdl.tokens, correctly tokenizes VHDL source code.
#
# VHDL has several unique token types compared to Verilog:
# - Based literals: 16#FF#, 2#1010#
# - Bit string literals: X"FF", B"1010"
# - Character literals: '0', '1', 'Z'
# - Keyword operators: and, or, xor, not (instead of &, |, ^, ~)
# - No preprocessor directives
# - Case-insensitive identifiers (normalized to lowercase)
# ================================================================

class TestVhdlLexer < Minitest::Test
  TT = CodingAdventures::Lexer::TokenType

  def tokenize(source)
    CodingAdventures::VhdlLexer.tokenize(source)
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  def token_values(source)
    tokenize(source).map(&:value)
  end

  # ------------------------------------------------------------------
  # Grammar path resolution
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::VhdlLexer::VHDL_TOKENS_PATH),
      "vhdl.tokens file should exist at #{CodingAdventures::VhdlLexer::VHDL_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # Basic signal declaration: signal clk : std_logic;
  # ------------------------------------------------------------------

  def test_signal_declaration
    tokens = tokenize("signal clk : std_logic;")
    types = tokens.map(&:type)
    assert_equal [TT::KEYWORD, TT::NAME, TT::COLON, TT::NAME, TT::SEMICOLON, TT::EOF], types
  end

  def test_signal_declaration_values
    tokens = tokenize("signal clk : std_logic;")
    values = tokens.map(&:value)
    assert_equal ["signal", "clk", ":", "std_logic", ";", ""], values
  end

  # ------------------------------------------------------------------
  # Case insensitivity
  # ------------------------------------------------------------------
  #
  # VHDL is case-insensitive. "ENTITY", "Entity", and "entity" are
  # identical. The lexer normalizes NAME and KEYWORD values to
  # lowercase after tokenization.
  # ------------------------------------------------------------------

  def test_case_insensitive_keywords
    tokens_upper = tokenize("ENTITY")
    tokens_lower = tokenize("entity")
    tokens_mixed = tokenize("Entity")

    assert_equal TT::KEYWORD, tokens_upper[0].type
    assert_equal TT::KEYWORD, tokens_lower[0].type
    assert_equal TT::KEYWORD, tokens_mixed[0].type

    assert_equal "entity", tokens_upper[0].value
    assert_equal "entity", tokens_lower[0].value
    assert_equal "entity", tokens_mixed[0].value
  end

  def test_case_insensitive_names
    tokens_upper = tokenize("MY_SIGNAL")
    tokens_lower = tokenize("my_signal")
    tokens_mixed = tokenize("My_Signal")

    assert_equal TT::NAME, tokens_upper[0].type
    assert_equal "my_signal", tokens_upper[0].value
    assert_equal "my_signal", tokens_lower[0].value
    assert_equal "my_signal", tokens_mixed[0].value
  end

  def test_case_insensitive_full_statement
    tokens1 = tokenize("SIGNAL CLK : STD_LOGIC;")
    tokens2 = tokenize("signal clk : std_logic;")

    values1 = tokens1.map(&:value)
    values2 = tokens2.map(&:value)
    assert_equal values1, values2
  end

  # ------------------------------------------------------------------
  # Keywords
  # ------------------------------------------------------------------
  #
  # VHDL has a large keyword set. Many are structural (entity,
  # architecture, begin, end), some are type-related (signal,
  # variable, constant), and some are logical operators (and, or,
  # xor, not).
  # ------------------------------------------------------------------

  def test_keyword_entity
    tokens = tokenize("entity")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "entity", tokens[0].value
  end

  def test_keyword_architecture
    tokens = tokenize("architecture")
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "architecture", tokens[0].value
  end

  def test_keyword_signal_variable_constant
    tokens = tokenize("signal variable constant")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[signal variable constant], keywords
  end

  def test_keyword_port_generic_map
    tokens = tokenize("port generic map")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[port generic map], keywords
  end

  def test_keyword_process_begin_end
    tokens = tokenize("process begin end")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[process begin end], keywords
  end

  def test_keyword_if_then_else_elsif
    tokens = tokenize("if then else elsif")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[if then else elsif], keywords
  end

  def test_keyword_case_when_others
    tokens = tokenize("case when others")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[case when others], keywords
  end

  def test_keyword_logical_operators
    tokens = tokenize("and or xor nand nor xnor not")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[and or xor nand nor xnor not], keywords
  end

  def test_keyword_shift_operators
    tokens = tokenize("sll srl sla sra rol ror")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[sll srl sla sra rol ror], keywords
  end

  def test_keyword_type_subtype_array_record
    tokens = tokenize("type subtype array record")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[type subtype array record], keywords
  end

  def test_keyword_library_use
    tokens = tokenize("library use")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[library use], keywords
  end

  def test_keyword_in_out_inout_buffer
    tokens = tokenize("in out inout buffer")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[in out inout buffer], keywords
  end

  def test_keyword_generate_for_while_loop
    tokens = tokenize("generate for while loop")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[generate for while loop], keywords
  end

  def test_keyword_function_procedure_return
    tokens = tokenize("function procedure return")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[function procedure return], keywords
  end

  def test_keyword_component_configuration_package
    tokens = tokenize("component configuration package")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[component configuration package], keywords
  end

  def test_keyword_to_downto_range
    tokens = tokenize("to downto range")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[to downto range], keywords
  end

  def test_keyword_is_of_new_null
    tokens = tokenize("is of new null")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[is of new null], keywords
  end

  def test_keyword_wait_until_after
    tokens = tokenize("wait until after")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[wait until after], keywords
  end

  def test_keyword_abs_mod_rem
    tokens = tokenize("abs mod rem")
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_equal %w[abs mod rem], keywords
  end

  def test_name_not_keyword
    tokens = tokenize("counter")
    assert_equal TT::NAME, tokens[0].type
    assert_equal "counter", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Numbers -- plain integers
  # ------------------------------------------------------------------

  def test_plain_number
    tokens = tokenize("42")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "42", tokens[0].value
  end

  def test_number_with_underscores
    tokens = tokenize("1_000_000")
    assert_equal TT::NUMBER, tokens[0].type
    assert_equal "1_000_000", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Based literals -- base#digits# notation
  # ------------------------------------------------------------------
  #
  # VHDL uses explicit base notation:
  #   16#FF#   -- hex 255
  #   2#1010#  -- binary 10
  #   8#77#    -- octal 63
  #
  # This is more verbose but clearer than Verilog's 8'hFF notation.
  # ------------------------------------------------------------------

  def test_based_literal_hex
    tokens = tokenize("16#FF#")
    assert_equal "16#ff#", tokens[0].value
  end

  def test_based_literal_binary
    tokens = tokenize("2#1010#")
    assert_equal "2#1010#", tokens[0].value
  end

  def test_based_literal_octal
    tokens = tokenize("8#77#")
    assert_equal "8#77#", tokens[0].value
  end

  def test_based_literal_with_exponent
    tokens = tokenize("16#FF#E2")
    assert_equal "16#ff#e2", tokens[0].value
  end

  def test_based_literal_with_underscores
    tokens = tokenize("2#1010_0011#")
    assert_equal "2#1010_0011#", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Bit string literals -- prefix"digits" notation
  # ------------------------------------------------------------------
  #
  # Bit strings are the VHDL equivalent of Verilog's sized literals:
  #   X"FF"    -- hex (4 bits per digit)
  #   B"1010"  -- binary (1 bit per digit)
  #   O"77"    -- octal (3 bits per digit)
  #   D"42"    -- decimal (VHDL-2008)
  # ------------------------------------------------------------------

  def test_bit_string_hex
    tokens = tokenize('X"FF"')
    assert_equal 'x"ff"', tokens[0].value
  end

  def test_bit_string_binary
    tokens = tokenize('B"1010"')
    assert_equal 'b"1010"', tokens[0].value
  end

  def test_bit_string_octal
    tokens = tokenize('O"77"')
    assert_equal 'o"77"', tokens[0].value
  end

  def test_bit_string_lowercase_prefix
    tokens = tokenize('x"ff"')
    assert_equal 'x"ff"', tokens[0].value
  end

  def test_bit_string_with_underscores
    tokens = tokenize('X"FF_00"')
    assert_equal 'x"ff_00"', tokens[0].value
  end

  # ------------------------------------------------------------------
  # Character literals
  # ------------------------------------------------------------------
  #
  # Character literals are single characters in tick marks:
  #   '0'  '1'  'X'  'Z'  'U'  'H'  'L'  '-'
  #
  # These are std_logic values in VHDL, used for signal assignments:
  #   clk <= '1';
  # ------------------------------------------------------------------

  def test_char_literal_zero
    tokens = tokenize("'0'")
    assert_equal "'0'", tokens[0].value
  end

  def test_char_literal_one
    tokens = tokenize("'1'")
    assert_equal "'1'", tokens[0].value
  end

  def test_char_literal_z
    tokens = tokenize("'Z'")
    assert_equal "'z'", tokens[0].value
  end

  def test_char_literal_x
    tokens = tokenize("'X'")
    assert_equal "'x'", tokens[0].value
  end

  def test_char_literal_dont_care
    tokens = tokenize("'-'")
    assert_equal "'-'", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Real numbers
  # ------------------------------------------------------------------

  def test_real_number
    tokens = tokenize("3.14")
    assert_equal "3.14", tokens[0].value
  end

  def test_real_with_exponent
    tokens = tokenize("1.5e-3")
    assert_equal "1.5e-3", tokens[0].value
  end

  def test_real_with_positive_exponent
    tokens = tokenize("2.5E+10")
    assert_equal "2.5e+10", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Strings
  # ------------------------------------------------------------------
  #
  # VHDL strings use double quotes. Escaped quotes are doubled:
  #   "He said ""hello"""  becomes  He said "hello"
  # ------------------------------------------------------------------

  def test_string_literal
    tokens = tokenize('"hello"')
    assert_equal TT::STRING, tokens[0].type
    assert_equal "hello", tokens[0].value
  end

  def test_string_with_escaped_quotes
    tokens = tokenize('"He said ""hi"""')
    assert_equal TT::STRING, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Extended identifiers
  # ------------------------------------------------------------------
  #
  # Extended identifiers are enclosed in backslashes and preserve
  # case (they are NOT normalized to lowercase):
  #   \My_Name\  stays as  \My_Name\
  # ------------------------------------------------------------------

  def test_extended_identifier
    tokens = tokenize('\my_name\\')
    assert_equal "EXTENDED_IDENT", tokens[0].type
  end

  # ------------------------------------------------------------------
  # Two-character operators
  # ------------------------------------------------------------------
  #
  # VHDL operators differ significantly from Verilog:
  #   :=  -- variable assignment (Verilog uses =)
  #   <=  -- signal assignment AND less-or-equal
  #   =>  -- port map arrow (no Verilog equivalent)
  #   /=  -- not-equal (Verilog uses !=)
  #   **  -- exponentiation
  #   <>  -- unconstrained range (box)
  # ------------------------------------------------------------------

  def test_var_assign
    tokens = tokenize("a := b")
    assert_equal ":=", tokens[1].value
  end

  def test_signal_assign_less_equals
    tokens = tokenize("a <= b")
    assert_equal "<=", tokens[1].value
  end

  def test_greater_equals
    tokens = tokenize("a >= b")
    assert_equal ">=", tokens[1].value
  end

  def test_arrow
    tokens = tokenize("a => b")
    assert_equal "=>", tokens[1].value
  end

  def test_not_equals
    tokens = tokenize("a /= b")
    assert_equal "/=", tokens[1].value
  end

  def test_power_operator
    tokens = tokenize("a ** b")
    assert_equal "**", tokens[1].value
  end

  def test_box_operator
    tokens = tokenize("a <>")
    assert_equal "<>", tokens[1].value
  end

  # ------------------------------------------------------------------
  # Single-character operators
  # ------------------------------------------------------------------

  def test_arithmetic_operators
    tokens = tokenize("+ - * /")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["+", "-", "*", "/"], values
  end

  def test_ampersand_concatenation
    tokens = tokenize("a & b")
    assert_equal "&", tokens[1].value
  end

  def test_less_than_greater_than
    tokens = tokenize("< >")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_equal ["<", ">"], values
  end

  def test_equals
    tokens = tokenize("=")
    assert_equal "=", tokens[0].value
  end

  def test_pipe
    tokens = tokenize("|")
    assert_equal "|", tokens[0].value
  end

  def test_tick_attribute
    # Tick after an identifier is for attribute access: signal'length
    # The character literal test above covers tick around characters.
    tokens = tokenize("a'length")
    # Should tokenize as: NAME("a") TICK("'") NAME("length")
    values = tokens.reject { |t| t.type == TT::EOF }.map(&:value)
    assert_includes values, "'"
  end

  # ------------------------------------------------------------------
  # Delimiters
  # ------------------------------------------------------------------

  def test_parens
    tokens = tokenize("( )")
    types = tokens.map(&:type)
    assert_equal [TT::LPAREN, TT::RPAREN, TT::EOF], types
  end

  def test_brackets
    tokens = tokenize("[ ]")
    types = tokens.map(&:type)
    assert_equal [TT::LBRACKET, TT::RBRACKET, TT::EOF], types
  end

  def test_semicolon
    tokens = tokenize(";")
    assert_equal TT::SEMICOLON, tokens[0].type
  end

  def test_comma
    tokens = tokenize(",")
    assert_equal ",", tokens[0].value
  end

  def test_dot
    tokens = tokenize(".")
    assert_equal ".", tokens[0].value
  end

  def test_colon
    tokens = tokenize(":")
    assert_equal TT::COLON, tokens[0].type
  end

  # ------------------------------------------------------------------
  # Comments (should be skipped)
  # ------------------------------------------------------------------
  #
  # VHDL uses -- for single-line comments (like Ada and Haskell).
  # There are no block comments in standard VHDL (VHDL-2008 added
  # /* */ but we target the core language).
  # ------------------------------------------------------------------

  def test_line_comment_skipped
    tokens = tokenize("signal a : std_logic; -- this is a comment")
    types = tokens.map(&:type)
    refute types.any? { |t| t.to_s.include?("COMMENT") }
    assert_equal [TT::KEYWORD, TT::NAME, TT::COLON, TT::NAME, TT::SEMICOLON, TT::EOF], types
  end

  def test_comment_only
    tokens = tokenize("-- just a comment")
    assert_equal [TT::EOF], tokens.map(&:type)
  end

  # ------------------------------------------------------------------
  # Entity declaration (realistic example)
  # ------------------------------------------------------------------
  #
  # A minimal VHDL entity:
  #   entity counter is
  #     port (
  #       clk : in std_logic;
  #       rst : in std_logic;
  #       count : out std_logic_vector(7 downto 0)
  #     );
  #   end entity counter;
  # ------------------------------------------------------------------

  def test_entity_declaration
    source = <<~VHDL
      entity counter is
        port (
          clk : in std_logic;
          count : out std_logic_vector(7 downto 0)
        );
      end entity counter;
    VHDL
    tokens = tokenize(source)
    # Should tokenize without errors and produce reasonable tokens
    assert tokens.length > 15
    assert_equal TT::KEYWORD, tokens[0].type
    assert_equal "entity", tokens[0].value
    assert_equal TT::EOF, tokens.last.type
  end

  # ------------------------------------------------------------------
  # Architecture body (realistic example)
  # ------------------------------------------------------------------

  def test_architecture_body
    source = <<~VHDL
      architecture rtl of counter is
        signal count_reg : std_logic_vector(7 downto 0);
      begin
        count <= count_reg;
      end architecture rtl;
    VHDL
    tokens = tokenize(source)
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_includes keywords, "architecture"
    assert_includes keywords, "of"
    assert_includes keywords, "is"
    assert_includes keywords, "signal"
    assert_includes keywords, "begin"
    assert_includes keywords, "end"
  end

  # ------------------------------------------------------------------
  # Process statement (realistic example)
  # ------------------------------------------------------------------

  def test_process_statement
    source = <<~VHDL
      process(clk)
      begin
        if rising_edge(clk) then
          q <= d;
        end if;
      end process;
    VHDL
    tokens = tokenize(source)
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_includes keywords, "process"
    assert_includes keywords, "begin"
    assert_includes keywords, "if"
    assert_includes keywords, "then"
    assert_includes keywords, "end"
  end

  # ------------------------------------------------------------------
  # Signal assignment with operators
  # ------------------------------------------------------------------

  def test_signal_assignment_with_operators
    source = "y <= (a and b) or (c xor d);"
    tokens = tokenize(source)
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_includes keywords, "and"
    assert_includes keywords, "or"
    assert_includes keywords, "xor"
  end

  # ------------------------------------------------------------------
  # Library and use clauses
  # ------------------------------------------------------------------

  def test_library_use_clause
    source = <<~VHDL
      library ieee;
      use ieee.std_logic_1164.all;
    VHDL
    tokens = tokenize(source)
    keywords = tokens.select { |t| t.type == TT::KEYWORD }.map(&:value)
    assert_includes keywords, "library"
    assert_includes keywords, "use"
    assert_includes keywords, "all"
  end

  # ------------------------------------------------------------------
  # Complete VHDL snippet (integration test)
  # ------------------------------------------------------------------
  #
  # This test verifies that a realistic, multi-line VHDL snippet
  # tokenizes correctly from start to finish, including library
  # clauses, entity declarations, architecture bodies, and
  # process statements.
  # ------------------------------------------------------------------

  def test_complete_vhdl_snippet
    source = <<~VHDL
      library ieee;
      use ieee.std_logic_1164.all;

      entity flipflop is
        port (
          clk : in std_logic;
          d   : in std_logic;
          q   : out std_logic
        );
      end entity flipflop;

      architecture rtl of flipflop is
      begin
        process(clk)
        begin
          if rising_edge(clk) then
            q <= d;
          end if;
        end process;
      end architecture rtl;
    VHDL
    tokens = tokenize(source)

    # Should tokenize without errors
    assert_equal TT::EOF, tokens.last.type

    # Should have a reasonable number of tokens
    assert tokens.length > 40

    # All keywords should be lowercase (case normalization)
    keywords = tokens.select { |t| t.type == TT::KEYWORD }
    keywords.each do |kw|
      assert_equal kw.value.downcase, kw.value,
        "Keyword #{kw.value.inspect} should be lowercase"
    end

    # All names should be lowercase
    names = tokens.select { |t| t.type == TT::NAME }
    names.each do |name|
      assert_equal name.value.downcase, name.value,
        "Name #{name.value.inspect} should be lowercase"
    end
  end

  # ------------------------------------------------------------------
  # Case normalization does NOT affect non-NAME/KEYWORD tokens
  # ------------------------------------------------------------------

  def test_string_case_preserved
    tokens = tokenize('"Hello World"')
    assert_equal "Hello World", tokens[0].value
  end

  def test_number_unchanged
    tokens = tokenize("42")
    assert_equal "42", tokens[0].value
  end

  def test_bit_string_case_preserved
    tokens = tokenize('X"FF"')
    assert_equal 'x"ff"', tokens[0].value
  end

  def test_char_literal_case_preserved
    tokens = tokenize("'A'")
    assert_equal "'a'", tokens[0].value
  end

  # ------------------------------------------------------------------
  # Version constant
  # ------------------------------------------------------------------

  def test_version_defined
    assert_equal "0.1.0", CodingAdventures::VhdlLexer::VERSION
  end

  def test_default_version_matches_explicit_2008
    default_tokens = tokenize("entity e is end entity e;")
    explicit_tokens = CodingAdventures::VhdlLexer.tokenize(
      "entity e is end entity e;",
      version: "2008"
    )
    assert_equal default_tokens.map(&:value), explicit_tokens.map(&:value)
  end

  def test_rejects_unknown_version
    error = assert_raises(ArgumentError) do
      CodingAdventures::VhdlLexer.tokenize("entity e is end entity e;", version: "2099")
    end
    assert_match(/Unknown VHDL version/, error.message)
  end
end
