# frozen_string_literal: true

require_relative "test_helper"

# Tests for the grammar compiler (compiler.rb).
#
# The compiler transforms in-memory TokenGrammar and ParserGrammar objects
# into Ruby source code. Tests verify:
#
#   1. The generated code contains the expected header / DO NOT EDIT comment.
#   2. The generated code is valid Ruby (eval-able without errors).
#   3. Loading the generated code recreates an equivalent grammar object.
#   4. All grammar features round-trip: aliases, skip patterns, error patterns,
#      groups, keywords, mode, escape_mode, case_sensitive.
#   5. Edge cases: empty grammars, special chars in patterns, nested elements.
#
# Round-trip fidelity
# -------------------
#
#   original = GT.parse_token_grammar(source)
#   code     = GT.compile_token_grammar(original)
#   binding  = Object.new
#   # eval code in isolated namespace
#   loaded   = ... TOKEN_GRAMMAR constant ...
#   assert_equal original.definitions, loaded.definitions

GT = CodingAdventures::GrammarTools

module GrammarToolsCompilerTests
  # Eval generated Ruby code in a fresh anonymous module and return the
  # TOKEN_GRAMMAR or PARSER_GRAMMAR constant.
  def eval_token_grammar(code)
    m = Module.new
    m.module_eval(code)
    m.const_get(:TOKEN_GRAMMAR)
  end

  def eval_parser_grammar(code)
    m = Module.new
    m.module_eval(code)
    m.const_get(:PARSER_GRAMMAR)
  end
end

# ============================================================================
# compile_token_grammar — output structure
# ============================================================================

class TestCompileTokenGrammarOutput < Minitest::Test
  include GrammarToolsCompilerTests

  def test_do_not_edit_header
    code = GT.compile_token_grammar(GT::TokenGrammar.new)
    assert_includes code, "DO NOT EDIT"
  end

  def test_source_line_present_when_given
    code = GT.compile_token_grammar(GT::TokenGrammar.new, "json.tokens")
    assert_includes code, "json.tokens"
  end

  def test_source_line_omitted_when_empty
    code = GT.compile_token_grammar(GT::TokenGrammar.new, "")
    refute_includes code, "# Source:"
  end

  def test_requires_grammar_tools
    code = GT.compile_token_grammar(GT::TokenGrammar.new)
    assert_includes code, "coding_adventures_grammar_tools"
  end

  def test_defines_token_grammar_constant
    code = GT.compile_token_grammar(GT::TokenGrammar.new)
    assert_includes code, "TOKEN_GRAMMAR"
  end
end

# ============================================================================
# compile_token_grammar — round-trip tests
# ============================================================================

class TestCompileTokenGrammarRoundTrip < Minitest::Test
  include GrammarToolsCompilerTests

  def test_empty_grammar
    original = GT::TokenGrammar.new
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal original.definitions,       loaded.definitions
    assert_equal original.keywords,          loaded.keywords
    assert_equal original.version,           loaded.version
    assert_equal original.case_sensitive,    loaded.case_sensitive
    assert_equal original.case_insensitive,  loaded.case_insensitive
  end

  def test_single_regex_token
    original = GT.parse_token_grammar("NUMBER = /[0-9]+/")
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal 1, loaded.definitions.length
    assert_equal "NUMBER",   loaded.definitions[0].name
    assert_equal "[0-9]+",   loaded.definitions[0].pattern
    assert_equal true,        loaded.definitions[0].is_regex
  end

  def test_single_literal_token
    original = GT.parse_token_grammar('PLUS = "+"')
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal "PLUS", loaded.definitions[0].name
    assert_equal "+",    loaded.definitions[0].pattern
    assert_equal false,  loaded.definitions[0].is_regex
  end

  def test_alias
    original = GT.parse_token_grammar('STRING_DQ = /"[^"]*"/ -> STRING')
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal "STRING", loaded.definitions[0].alias_name
  end

  def test_keywords
    source = "NAME = /[a-z]+/\nkeywords:\n  if\n  else\n  while\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal %w[if else while], loaded.keywords
  end

  def test_skip_definitions
    source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal 1, loaded.skip_definitions.length
    assert_equal "WHITESPACE", loaded.skip_definitions[0].name
  end

  def test_error_definitions
    source = "STRING = /\"[^\"]*\"/\nerrors:\n  BAD_STRING = /\"[^\"\\n]*/\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal 1, loaded.error_definitions.length
    assert_equal "BAD_STRING", loaded.error_definitions[0].name
  end

  def test_mode_indentation
    source = "mode: indentation\nNAME = /[a-z]+/"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal "indentation", loaded.mode
  end

  def test_mode_layout_with_layout_keywords
    source = <<~TOKENS
      mode: layout
      NAME = /[a-z]+/
      layout_keywords:
        let
        where
    TOKENS
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal "layout", loaded.mode
    assert_equal %w[let where], loaded.layout_keywords
  end

  def test_escape_mode_none
    source = "escapes: none\nSTRING = /\"[^\"]*\"/"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal "none", loaded.escape_mode
  end

  def test_version
    source = "# @version 3\nNAME = /[a-z]+/"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal 3, loaded.version
  end

  def test_case_insensitive
    source = "# @case_insensitive true\nNAME = /[a-z]+/"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal true, loaded.case_insensitive
  end

  def test_pattern_groups
    source = "TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\n  EQ = \"=\"\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert loaded.groups.key?("tag")
    assert_equal 2, loaded.groups["tag"].definitions.length
  end

  def test_special_regex_characters
    source = 'STRING = /"([^"\\\\]|\\\\["\\\\\\/bfnrt]|\\\\u[0-9a-fA-F]{4})*"/'
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal original.definitions[0].pattern, loaded.definitions[0].pattern
  end

  def test_context_keywords
    source = "NAME = /[a-z]+/\ncontext_keywords:\n  async\n  await\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal %w[async await], loaded.context_keywords
  end

  def test_soft_keywords
    source = "NAME = /[a-z]+/\nsoft_keywords:\n  match\n  case\n  type\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal %w[match case type], loaded.soft_keywords
  end

  def test_soft_keywords_and_context_keywords_together
    source = "NAME = /[a-z]+/\ncontext_keywords:\n  async\nsoft_keywords:\n  match\n"
    original = GT.parse_token_grammar(source)
    code     = GT.compile_token_grammar(original)
    loaded   = eval_token_grammar(code)
    assert_equal %w[async], loaded.context_keywords
    assert_equal %w[match], loaded.soft_keywords
  end
end

# ============================================================================
# compile_parser_grammar — output structure
# ============================================================================

class TestCompileParserGrammarOutput < Minitest::Test
  include GrammarToolsCompilerTests

  def test_do_not_edit_header
    code = GT.compile_parser_grammar(GT::ParserGrammar.new)
    assert_includes code, "DO NOT EDIT"
  end

  def test_defines_parser_grammar_constant
    code = GT.compile_parser_grammar(GT::ParserGrammar.new)
    assert_includes code, "PARSER_GRAMMAR"
  end

  def test_requires_grammar_tools
    code = GT.compile_parser_grammar(GT::ParserGrammar.new)
    assert_includes code, "coding_adventures_grammar_tools"
  end
end

# ============================================================================
# compile_parser_grammar — round-trip tests
# ============================================================================

class TestCompileParserGrammarRoundTrip < Minitest::Test
  include GrammarToolsCompilerTests

  def test_empty_grammar
    original = GT::ParserGrammar.new
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_equal 0,  loaded.version
    assert_equal [], loaded.rules
  end

  def test_single_rule_token_reference
    original = GT.parse_parser_grammar("value = NUMBER ;")
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_equal 1,       loaded.rules.length
    assert_equal "value", loaded.rules[0].name
    assert_equal GT::RuleReference.new(name: "NUMBER", is_token: true),
                 loaded.rules[0].body
  end

  def test_alternation_round_trip
    original = GT.parse_parser_grammar("value = A | B | C ;")
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_instance_of GT::Alternation, loaded.rules[0].body
    assert_equal 3, loaded.rules[0].body.choices.length
  end

  def test_sequence_round_trip
    original = GT.parse_parser_grammar("pair = KEY COLON VALUE ;")
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_instance_of GT::Sequence, loaded.rules[0].body
  end

  def test_repetition_round_trip
    original = GT.parse_parser_grammar("stmts = { stmt } ;")
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_instance_of GT::Repetition, loaded.rules[0].body
  end

  def test_optional_round_trip
    original = GT.parse_parser_grammar("expr = NUMBER [ PLUS NUMBER ] ;")
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    # The outer body is a Sequence (NUMBER then OptionalElement)
    assert_instance_of GT::Sequence, loaded.rules[0].body
    assert_instance_of GT::OptionalElement, loaded.rules[0].body.elements[1]
  end

  def test_literal_round_trip
    original = GT.parse_parser_grammar('start = "hello" ;')
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_equal GT::Literal.new(value: "hello"), loaded.rules[0].body
  end

  def test_version_round_trip
    original = GT.parse_parser_grammar("# @version 4\nvalue = NUMBER ;")
    code     = GT.compile_parser_grammar(original)
    loaded   = eval_parser_grammar(code)
    assert_equal 4, loaded.version
  end

  def test_json_grammar_full_round_trip
    source = <<~GRAMMAR
      value    = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
      object   = LBRACE [ pair { COMMA pair } ] RBRACE ;
      pair     = STRING COLON value ;
      array    = LBRACKET [ value { COMMA value } ] RBRACKET ;
    GRAMMAR
    original = GT.parse_parser_grammar(source)
    code     = GT.compile_parser_grammar(original, "json.grammar")
    loaded   = eval_parser_grammar(code)
    assert_equal 4,       loaded.rules.length
    assert_equal "value", loaded.rules[0].name
    assert_equal "array", loaded.rules[3].name
  end
end
