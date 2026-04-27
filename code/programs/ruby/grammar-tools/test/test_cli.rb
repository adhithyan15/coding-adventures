# frozen_string_literal: true

require "minitest/autorun"
require "pathname"

# Load the main program so we can call its functions directly in tests.
ROOT = begin
  dir = Pathname.new(__FILE__).expand_path.dirname.parent
  20.times do
    break dir if (dir / "../../../../code/specs/grammar-tools.json").exist? ||
                 (dir.parent.parent.parent.parent / "code/specs/grammar-tools.json").exist?
    parent = dir.parent
    break dir if parent == dir
    dir = parent
  end
  # Walk up to the actual repo root
  d = Pathname.new(__FILE__).expand_path
  20.times do
    break d if (d / "code/specs/grammar-tools.json").exist?
    p = d.parent
    break d if p == d
    d = p
  end
  d
end

GRAMMARS_DIR = ROOT / "code/grammars"

# Load main.rb helpers without running main()
load File.expand_path("../main.rb", __dir__)

class TestValidateCommand < Minitest::Test
  def test_succeeds_on_json_pair
    tokens  = GRAMMARS_DIR / "json.tokens"
    grammar = GRAMMARS_DIR / "json.grammar"
    return skip unless tokens.exist? && grammar.exist?
    assert_equal 0, validate_command(tokens.to_s, grammar.to_s)
  end

  def test_succeeds_on_lisp_pair
    tokens  = GRAMMARS_DIR / "lisp.tokens"
    grammar = GRAMMARS_DIR / "lisp.grammar"
    return skip unless tokens.exist? && grammar.exist?
    assert_equal 0, validate_command(tokens.to_s, grammar.to_s)
  end

  def test_returns_1_on_missing_tokens
    assert_equal 1, validate_command("/nonexistent/x.tokens", "any.grammar")
  end

  def test_returns_1_on_missing_grammar
    tokens = GRAMMARS_DIR / "json.tokens"
    return skip unless tokens.exist?
    assert_equal 1, validate_command(tokens.to_s, "/nonexistent/x.grammar")
  end
end

class TestValidateTokensOnly < Minitest::Test
  def test_succeeds_on_json_tokens
    tokens = GRAMMARS_DIR / "json.tokens"
    return skip unless tokens.exist?
    assert_equal 0, validate_tokens_only(tokens.to_s)
  end

  def test_returns_1_on_missing_file
    assert_equal 1, validate_tokens_only("/nonexistent/x.tokens")
  end
end

class TestValidateGrammarOnly < Minitest::Test
  def test_succeeds_on_json_grammar
    grammar = GRAMMARS_DIR / "json.grammar"
    return skip unless grammar.exist?
    assert_equal 0, validate_grammar_only(grammar.to_s)
  end

  def test_returns_1_on_missing_file
    assert_equal 1, validate_grammar_only("/nonexistent/x.grammar")
  end
end

class TestDispatch < Minitest::Test
  def test_unknown_command_returns_2
    assert_equal 2, dispatch("unknown", [])
  end

  def test_validate_wrong_count_returns_2
    assert_equal 2, dispatch("validate", ["one-file.tokens"])
  end

  def test_validate_tokens_no_files_returns_2
    assert_equal 2, dispatch("validate-tokens", [])
  end

  def test_validate_grammar_no_files_returns_2
    assert_equal 2, dispatch("validate-grammar", [])
  end

  def test_validate_dispatches_correctly
    tokens  = GRAMMARS_DIR / "json.tokens"
    grammar = GRAMMARS_DIR / "json.grammar"
    return skip unless tokens.exist? && grammar.exist?
    assert_equal 0, dispatch("validate", [tokens.to_s, grammar.to_s])
  end

  def test_validate_tokens_dispatches_correctly
    tokens = GRAMMARS_DIR / "json.tokens"
    return skip unless tokens.exist?
    assert_equal 0, dispatch("validate-tokens", [tokens.to_s])
  end

  def test_validate_grammar_dispatches_correctly
    grammar = GRAMMARS_DIR / "json.grammar"
    return skip unless grammar.exist?
    assert_equal 0, dispatch("validate-grammar", [grammar.to_s])
  end
end

class TestGenerateCompiledGrammarsDispatch < Minitest::Test
  def setup
    @original_generator = GrammarToolsProgram::CompiledGrammarGenerator

    fake_generator = Class.new do
      def initialize(_root); end

      def run
        17
      end
    end

    GrammarToolsProgram.send(:remove_const, :CompiledGrammarGenerator)
    GrammarToolsProgram.const_set(:CompiledGrammarGenerator, fake_generator)
  end

  def teardown
    GrammarToolsProgram.send(:remove_const, :CompiledGrammarGenerator)
    GrammarToolsProgram.const_set(:CompiledGrammarGenerator, @original_generator)
  end

  def test_dispatches_to_generator
    assert_equal 17, dispatch("generate-compiled-grammars", [])
  end

  def test_rejects_extra_files
    assert_equal 2, dispatch("generate-compiled-grammars", ["unexpected"])
  end
end
