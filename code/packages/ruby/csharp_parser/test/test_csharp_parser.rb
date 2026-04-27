# frozen_string_literal: true

require_relative "test_helper"

class TestCSharpParser < Minitest::Test
  def parse(source, version: nil)
    CodingAdventures::CSharpParser.parse(source, version: version)
  end

  def assert_parses_compilation_unit(source, version: nil)
    ast = parse(source, version: version)
    assert_equal "compilation_unit", ast.rule_name
    ast
  end

  def test_simple_class_declaration
    assert_parses_compilation_unit("class Foo {}")
  end

  def test_public_class_declaration
    assert_parses_compilation_unit("public class Main {}")
  end

  def test_namespaced_class_declaration
    assert_parses_compilation_unit("namespace MyApp { public class Greeter {} }")
  end

  def test_method_inside_class
    assert_parses_compilation_unit("class Program { void Main() {} }")
  end

  def test_grammar_path_exists
    path = CodingAdventures::CSharpParser.resolve_grammar_path(nil)
    assert File.exist?(path),
      "csharp12.0.grammar file should exist at #{path}"
  end

  def test_no_version_uses_default_grammar
    path = CodingAdventures::CSharpParser.resolve_grammar_path(nil)
    assert_match(%r{csharp/csharp12\.0\.grammar$}, path)
    assert File.exist?(path), "Default csharp12.0.grammar should exist"
  end

  def test_empty_string_version_uses_default_grammar
    path = CodingAdventures::CSharpParser.resolve_grammar_path("")
    assert_match(%r{csharp/csharp12\.0\.grammar$}, path)
  end

  def test_resolve_grammar_path_1_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("1.0")
    assert_match(%r{csharp/csharp1\.0\.grammar$}, path)
    assert File.exist?(path), "csharp1.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_5_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("5.0")
    assert_match(%r{csharp/csharp5\.0\.grammar$}, path)
    assert File.exist?(path), "csharp5.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_8_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("8.0")
    assert_match(%r{csharp/csharp8\.0\.grammar$}, path)
    assert File.exist?(path), "csharp8.0.grammar should exist at #{path}"
  end

  def test_resolve_grammar_path_12_0
    path = CodingAdventures::CSharpParser.resolve_grammar_path("12.0")
    assert_match(%r{csharp/csharp12\.0\.grammar$}, path)
    assert File.exist?(path), "csharp12.0.grammar should exist at #{path}"
  end

  def test_all_valid_versions_have_grammar_files
    CodingAdventures::CSharpParser::VALID_VERSIONS.each do |version|
      path = CodingAdventures::CSharpParser.resolve_grammar_path(version)
      assert File.exist?(path),
        "Grammar file for version #{version.inspect} should exist at #{path}"
    end
  end

  def test_valid_versions_count
    assert_equal 12, CodingAdventures::CSharpParser::VALID_VERSIONS.length
  end

  def test_unknown_version_raises_argument_error
    err = assert_raises(ArgumentError) do
      parse("public class Foo {}", version: "99")
    end
    assert_match(/99/, err.message)
  end

  def test_unknown_version_with_java_style_integer_raises
    err = assert_raises(ArgumentError) do
      parse("public class Foo {}", version: "8")
    end
    assert_match(/8/, err.message)
  end

  def test_backward_compatible_no_version
    assert_parses_compilation_unit("public class Foo {}")
  end

  def test_parse_csharp_alias_works
    ast = CodingAdventures::CSharpParser.parse_csharp("public class Foo {}")
    assert_equal "compilation_unit", ast.rule_name
  end

  def test_parse_csharp_alias_accepts_version
    ast = CodingAdventures::CSharpParser.parse_csharp("public class Foo {}", version: "5.0")
    assert_equal "compilation_unit", ast.rule_name
  end

  def test_parse_all_versions_with_class_declaration
    CodingAdventures::CSharpParser::VALID_VERSIONS.each do |version|
      assert_parses_compilation_unit("public class Foo {}", version: version)
    end
  end

  def test_top_level_statements_supported_in_csharp_9_and_later
    %w[9.0 10.0 11.0 12.0].each do |version|
      assert_parses_compilation_unit("int x = 1;", version: version)
    end
    assert_parses_compilation_unit("int x = 1;")
  end

  def test_create_csharp_parser_returns_hash
    result = CodingAdventures::CSharpParser.create_csharp_parser("public class Foo {}")
    assert_instance_of Hash, result
  end

  def test_create_csharp_parser_stores_source
    result = CodingAdventures::CSharpParser.create_csharp_parser("public class Foo {}")
    assert_equal "public class Foo {}", result[:source]
  end

  def test_create_csharp_parser_stores_nil_version
    result = CodingAdventures::CSharpParser.create_csharp_parser("public class Foo {}")
    assert_nil result[:version]
  end

  def test_create_csharp_parser_stores_language
    result = CodingAdventures::CSharpParser.create_csharp_parser("public class Foo {}")
    assert_equal :csharp, result[:language]
  end

  def test_create_csharp_parser_with_version
    result = CodingAdventures::CSharpParser.create_csharp_parser("public class Foo {}", version: "8.0")
    assert_equal "8.0", result[:version]
  end

  def test_create_csharp_parser_raises_for_unknown_version
    assert_raises(ArgumentError) do
      CodingAdventures::CSharpParser.create_csharp_parser("public class Foo {}", version: "bogus")
    end
  end
end
