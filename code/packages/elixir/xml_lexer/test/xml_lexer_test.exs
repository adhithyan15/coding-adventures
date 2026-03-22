defmodule CodingAdventures.XmlLexerTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for the XML Lexer.

  These tests verify that the XML lexer correctly tokenizes XML documents
  using pattern groups and the on-token callback. The tests cover:

  1. Basic elements -- open/close tags, text content
  2. Attributes -- single and double quoted values
  3. Self-closing tags -- `<br/>`
  4. Comments -- `<!-- ... -->`
  5. CDATA sections -- `<![CDATA[ ... ]]>`
  6. Processing instructions -- `<?xml ... ?>`
  7. Entity references -- `&amp;`, `&#65;`, `&#x41;`
  8. Nested structures -- tags within tags
  9. Mixed content -- text interspersed with elements
  10. Edge cases -- empty input, text-only, whitespace handling
  """

  alias CodingAdventures.XmlLexer

  # ---------------------------------------------------------------------------
  # Helpers -- extract token types and (type, value) pairs
  # ---------------------------------------------------------------------------

  defp token_pairs(source) do
    {:ok, tokens} = XmlLexer.tokenize(source)

    tokens
    |> Enum.reject(&(&1.type == "EOF"))
    |> Enum.map(&{&1.type, &1.value})
  end

  defp token_types(source) do
    {:ok, tokens} = XmlLexer.tokenize(source)

    tokens
    |> Enum.reject(&(&1.type == "EOF"))
    |> Enum.map(& &1.type)
  end

  # ===========================================================================
  # Grammar Loading
  # ===========================================================================

  describe "create_lexer/0" do
    test "returns a TokenGrammar with XML token definitions" do
      grammar = XmlLexer.create_lexer()
      names = Enum.map(grammar.definitions, & &1.name)
      assert "TEXT" in names
      assert "ENTITY_REF" in names
      assert "CHAR_REF" in names
      assert "COMMENT_START" in names
      assert "CDATA_START" in names
      assert "PI_START" in names
      assert "CLOSE_TAG_START" in names
      assert "OPEN_TAG_START" in names
    end

    test "grammar includes pattern groups" do
      grammar = XmlLexer.create_lexer()
      group_names = Map.keys(grammar.groups)
      assert "tag" in group_names
      assert "comment" in group_names
      assert "cdata" in group_names
      assert "pi" in group_names
    end
  end

  # ===========================================================================
  # Basic Tags
  # ===========================================================================

  describe "tokenize/1 -- basic tags" do
    test "simple element: <p>text</p>" do
      pairs = token_pairs("<p>text</p>")

      assert pairs == [
               {"OPEN_TAG_START", "<"},
               {"TAG_NAME", "p"},
               {"TAG_CLOSE", ">"},
               {"TEXT", "text"},
               {"CLOSE_TAG_START", "</"},
               {"TAG_NAME", "p"},
               {"TAG_CLOSE", ">"}
             ]
    end

    test "element with namespace prefix: <ns:tag>" do
      types = token_types("<ns:tag>content</ns:tag>")

      assert types == [
               "OPEN_TAG_START",
               "TAG_NAME",
               "TAG_CLOSE",
               "TEXT",
               "CLOSE_TAG_START",
               "TAG_NAME",
               "TAG_CLOSE"
             ]

      pairs = token_pairs("<ns:tag>content</ns:tag>")
      {_type, value} = Enum.at(pairs, 1)
      assert value == "ns:tag"
    end

    test "empty element: <div></div>" do
      pairs = token_pairs("<div></div>")

      assert pairs == [
               {"OPEN_TAG_START", "<"},
               {"TAG_NAME", "div"},
               {"TAG_CLOSE", ">"},
               {"CLOSE_TAG_START", "</"},
               {"TAG_NAME", "div"},
               {"TAG_CLOSE", ">"}
             ]
    end

    test "self-closing tag: <br/>" do
      pairs = token_pairs("<br/>")

      assert pairs == [
               {"OPEN_TAG_START", "<"},
               {"TAG_NAME", "br"},
               {"SELF_CLOSE", "/>"}
             ]
    end

    test "self-closing with space: <br />" do
      pairs = token_pairs("<br />")

      assert pairs == [
               {"OPEN_TAG_START", "<"},
               {"TAG_NAME", "br"},
               {"SELF_CLOSE", "/>"}
             ]
    end
  end

  # ===========================================================================
  # Attributes
  # ===========================================================================

  describe "tokenize/1 -- attributes" do
    test "double-quoted attribute" do
      pairs = token_pairs(~s(<div class="main">))

      assert pairs == [
               {"OPEN_TAG_START", "<"},
               {"TAG_NAME", "div"},
               {"TAG_NAME", "class"},
               {"ATTR_EQUALS", "="},
               {"ATTR_VALUE", ~s("main")},
               {"TAG_CLOSE", ">"}
             ]
    end

    test "single-quoted attribute" do
      pairs = token_pairs("<div class='main'>")

      assert pairs == [
               {"OPEN_TAG_START", "<"},
               {"TAG_NAME", "div"},
               {"TAG_NAME", "class"},
               {"ATTR_EQUALS", "="},
               {"ATTR_VALUE", "'main'"},
               {"TAG_CLOSE", ">"}
             ]
    end

    test "multiple attributes" do
      pairs = token_pairs(~s(<a href="url" target="_blank">))
      tag_names = for {"TAG_NAME", v} <- pairs, do: v
      assert tag_names == ["a", "href", "target"]
      attr_values = for {"ATTR_VALUE", v} <- pairs, do: v
      assert attr_values == [~s("url"), ~s("_blank")]
    end

    test "attribute on self-closing tag" do
      types = token_types(~s(<img src="photo.jpg"/>))
      assert "SELF_CLOSE" in types
      assert "ATTR_VALUE" in types
    end
  end

  # ===========================================================================
  # Comments
  # ===========================================================================

  describe "tokenize/1 -- comments" do
    test "simple comment" do
      pairs = token_pairs("<!-- hello -->")

      assert pairs == [
               {"COMMENT_START", "<!--"},
               {"COMMENT_TEXT", " hello "},
               {"COMMENT_END", "-->"}
             ]
    end

    test "comment preserves whitespace" do
      pairs = token_pairs("<!--  spaces  and\ttabs  -->")
      texts = for {"COMMENT_TEXT", v} <- pairs, do: v
      assert texts == ["  spaces  and\ttabs  "]
    end

    test "comment with dashes" do
      pairs = token_pairs("<!-- a-b-c -->")
      texts = for {"COMMENT_TEXT", v} <- pairs, do: v
      assert texts == [" a-b-c "]
    end

    test "comment between elements" do
      types = token_types("<a/><!-- mid --><b/>")
      assert "COMMENT_START" in types
      assert "COMMENT_END" in types
    end
  end

  # ===========================================================================
  # CDATA Sections
  # ===========================================================================

  describe "tokenize/1 -- CDATA sections" do
    test "simple CDATA" do
      pairs = token_pairs("<![CDATA[raw text]]>")

      assert pairs == [
               {"CDATA_START", "<![CDATA["},
               {"CDATA_TEXT", "raw text"},
               {"CDATA_END", "]]>"}
             ]
    end

    test "CDATA with angle brackets" do
      pairs = token_pairs("<![CDATA[<not a tag>]]>")
      texts = for {"CDATA_TEXT", v} <- pairs, do: v
      assert texts == ["<not a tag>"]
    end

    test "CDATA preserves whitespace" do
      pairs = token_pairs("<![CDATA[  hello\n  world  ]]>")
      texts = for {"CDATA_TEXT", v} <- pairs, do: v
      assert texts == ["  hello\n  world  "]
    end

    test "CDATA with single bracket" do
      pairs = token_pairs("<![CDATA[a]b]]>")
      texts = for {"CDATA_TEXT", v} <- pairs, do: v
      assert texts == ["a]b"]
    end
  end

  # ===========================================================================
  # Processing Instructions
  # ===========================================================================

  describe "tokenize/1 -- processing instructions" do
    test "XML declaration" do
      pairs = token_pairs(~s(<?xml version="1.0"?>))

      assert pairs == [
               {"PI_START", "<?"},
               {"PI_TARGET", "xml"},
               {"PI_TEXT", ~s( version="1.0")},
               {"PI_END", "?>"}
             ]
    end

    test "stylesheet processing instruction" do
      types = token_types(~s(<?xml-stylesheet type="text/xsl"?>))
      assert List.first(types) == "PI_START"
      assert Enum.at(types, 1) == "PI_TARGET"
      assert List.last(types) == "PI_END"
    end
  end

  # ===========================================================================
  # Entity and Character References
  # ===========================================================================

  describe "tokenize/1 -- entity and character references" do
    test "named entity reference" do
      pairs = token_pairs("a&amp;b")

      assert pairs == [
               {"TEXT", "a"},
               {"ENTITY_REF", "&amp;"},
               {"TEXT", "b"}
             ]
    end

    test "decimal character reference" do
      pairs = token_pairs("&#65;")
      assert pairs == [{"CHAR_REF", "&#65;"}]
    end

    test "hex character reference" do
      pairs = token_pairs("&#x41;")
      assert pairs == [{"CHAR_REF", "&#x41;"}]
    end

    test "multiple entities in text" do
      types = token_types("&lt;hello&gt;")
      assert types == ["ENTITY_REF", "TEXT", "ENTITY_REF"]
    end
  end

  # ===========================================================================
  # Nested and Mixed Content
  # ===========================================================================

  describe "tokenize/1 -- nested and mixed content" do
    test "nested elements" do
      types = token_types("<a><b>text</b></a>")
      assert Enum.count(types, &(&1 == "OPEN_TAG_START")) == 2
      assert Enum.count(types, &(&1 == "CLOSE_TAG_START")) == 2
    end

    test "mixed content: text with child elements" do
      pairs = token_pairs("<p>Hello <b>world</b>!</p>")
      texts = for {"TEXT", v} <- pairs, do: v
      assert texts == ["Hello ", "world", "!"]
    end

    test "full document with PI, comment, tags, and entities" do
      source =
        ~s(<?xml version="1.0"?>) <>
          "<!-- A greeting -->" <>
          ~s(<root lang="en">) <>
          "<greeting>Hello &amp; welcome</greeting>" <>
          "</root>"

      {:ok, tokens} = XmlLexer.tokenize(source)
      types = Enum.map(tokens, & &1.type)

      # PI present
      assert "PI_START" in types
      assert "PI_END" in types

      # Comment present
      assert "COMMENT_START" in types
      assert "COMMENT_END" in types

      # Tags present
      assert Enum.count(types, &(&1 == "OPEN_TAG_START")) == 2
      assert Enum.count(types, &(&1 == "CLOSE_TAG_START")) == 2

      # Entity ref present
      assert "ENTITY_REF" in types

      # Ends with EOF
      assert List.last(types) == "EOF"
    end

    test "CDATA inside element" do
      source = "<script><![CDATA[x < y]]></script>"
      types = token_types(source)
      assert "CDATA_START" in types
      assert "CDATA_TEXT" in types
      assert "CDATA_END" in types
    end
  end

  # ===========================================================================
  # Edge Cases
  # ===========================================================================

  describe "tokenize/1 -- edge cases" do
    test "empty input produces only EOF" do
      {:ok, tokens} = XmlLexer.tokenize("")
      assert length(tokens) == 1
      assert List.first(tokens).type == "EOF"
    end

    test "text only, no tags" do
      pairs = token_pairs("just text")
      assert pairs == [{"TEXT", "just text"}]
    end

    test "whitespace between tags is consumed by skip pattern" do
      pairs = token_pairs("<a> <b> </b> </a>")
      texts = for {"TEXT", v} <- pairs, do: v
      assert texts == []
    end

    test "EOF is always the last token" do
      {:ok, tokens} = XmlLexer.tokenize("<root/>")
      assert List.last(tokens).type == "EOF"
    end

    test "position tracking: first token starts at line 1, column 1" do
      {:ok, tokens} = XmlLexer.tokenize("<p>hello</p>")
      [first | _] = tokens
      assert first.line == 1
      assert first.column == 1
    end
  end

  # ===========================================================================
  # Callback Function
  # ===========================================================================

  describe "xml_on_token/2 -- callback actions" do
    test "OPEN_TAG_START pushes tag group" do
      token = %CodingAdventures.Lexer.Token{type: "OPEN_TAG_START", value: "<"}
      assert XmlLexer.xml_on_token(token, nil) == [{:push_group, "tag"}]
    end

    test "CLOSE_TAG_START pushes tag group" do
      token = %CodingAdventures.Lexer.Token{type: "CLOSE_TAG_START", value: "</"}
      assert XmlLexer.xml_on_token(token, nil) == [{:push_group, "tag"}]
    end

    test "TAG_CLOSE pops group" do
      token = %CodingAdventures.Lexer.Token{type: "TAG_CLOSE", value: ">"}
      assert XmlLexer.xml_on_token(token, nil) == [:pop_group]
    end

    test "SELF_CLOSE pops group" do
      token = %CodingAdventures.Lexer.Token{type: "SELF_CLOSE", value: "/>"}
      assert XmlLexer.xml_on_token(token, nil) == [:pop_group]
    end

    test "COMMENT_START pushes comment group and disables skip" do
      token = %CodingAdventures.Lexer.Token{type: "COMMENT_START", value: "<!--"}
      assert XmlLexer.xml_on_token(token, nil) == [{:push_group, "comment"}, {:set_skip_enabled, false}]
    end

    test "COMMENT_END pops group and enables skip" do
      token = %CodingAdventures.Lexer.Token{type: "COMMENT_END", value: "-->"}
      assert XmlLexer.xml_on_token(token, nil) == [:pop_group, {:set_skip_enabled, true}]
    end

    test "CDATA_START pushes cdata group and disables skip" do
      token = %CodingAdventures.Lexer.Token{type: "CDATA_START", value: "<![CDATA["}
      assert XmlLexer.xml_on_token(token, nil) == [{:push_group, "cdata"}, {:set_skip_enabled, false}]
    end

    test "CDATA_END pops group and enables skip" do
      token = %CodingAdventures.Lexer.Token{type: "CDATA_END", value: "]]>"}
      assert XmlLexer.xml_on_token(token, nil) == [:pop_group, {:set_skip_enabled, true}]
    end

    test "PI_START pushes pi group and disables skip" do
      token = %CodingAdventures.Lexer.Token{type: "PI_START", value: "<?"}
      assert XmlLexer.xml_on_token(token, nil) == [{:push_group, "pi"}, {:set_skip_enabled, false}]
    end

    test "PI_END pops group and enables skip" do
      token = %CodingAdventures.Lexer.Token{type: "PI_END", value: "?>"}
      assert XmlLexer.xml_on_token(token, nil) == [:pop_group, {:set_skip_enabled, true}]
    end

    test "other token types return empty list" do
      token = %CodingAdventures.Lexer.Token{type: "TEXT", value: "hello"}
      assert XmlLexer.xml_on_token(token, nil) == []
    end
  end
end
