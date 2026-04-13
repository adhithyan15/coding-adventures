# frozen_string_literal: true

require_relative "test_helper"
require "fileutils"
require "tmpdir"

GT = CodingAdventures::GrammarTools

class TestCompiledLoader < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("compiled-grammar-loader")
    GT.clear_compiled_grammar_cache!
    remove_exported_consts!
  end

  def teardown
    GT.clear_compiled_grammar_cache!
    remove_exported_consts!
    FileUtils.remove_entry(@tmpdir) if @tmpdir && Dir.exist?(@tmpdir)
  end

  def test_load_token_grammar_from_compiled_file
    grammar = GT.parse_token_grammar("NUMBER = /[0-9]+/")
    path = write_compiled_file("sample_token.rb", GT.compile_token_grammar(grammar, "sample.tokens"))

    loaded = GT.load_token_grammar(path)

    assert_equal grammar.definitions, loaded.definitions
    refute Object.const_defined?(:TOKEN_GRAMMAR, false)
  end

  def test_load_parser_grammar_from_compiled_file
    grammar = GT.parse_parser_grammar("value = NUMBER ;")
    path = write_compiled_file("sample_parser.rb", GT.compile_parser_grammar(grammar, "sample.grammar"))

    loaded = GT.load_parser_grammar(path)

    assert_equal grammar.rules, loaded.rules
    refute Object.const_defined?(:PARSER_GRAMMAR, false)
  end

  def test_loader_caches_compiled_grammar_by_absolute_path
    grammar = GT.parse_token_grammar("NAME = /[a-z]+/")
    path = write_compiled_file("cached_token.rb", GT.compile_token_grammar(grammar, "cached.tokens"))

    first = GT.load_token_grammar(path)
    second = GT.load_token_grammar(path)

    assert_same first, second
  end

  def test_clear_cache_allows_reloading_the_same_compiled_file
    grammar = GT.parse_token_grammar("INT = /[0-9]+/")
    path = write_compiled_file("reloadable_token.rb", GT.compile_token_grammar(grammar, "reloadable.tokens"))

    first = GT.load_token_grammar(path)
    GT.clear_compiled_grammar_cache!
    second = GT.load_token_grammar(path)

    refute_same first, second
    assert_equal first.definitions, second.definitions
  end

  private

  def write_compiled_file(filename, contents)
    path = File.join(@tmpdir, filename)
    File.write(path, contents)
    path
  end

  def remove_exported_consts!
    Object.send(:remove_const, :TOKEN_GRAMMAR) if Object.const_defined?(:TOKEN_GRAMMAR, false)
    Object.send(:remove_const, :PARSER_GRAMMAR) if Object.const_defined?(:PARSER_GRAMMAR, false)
  end
end
