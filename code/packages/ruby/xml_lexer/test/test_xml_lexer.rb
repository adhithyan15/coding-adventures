# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the XML Lexer
# ================================================================
#
# These tests verify that the grammar-driven lexer, when loaded
# with xml.tokens and the xml_on_token callback, correctly
# tokenizes XML documents using pattern groups for context-sensitive
# lexing.
#
# XML is the first language that requires pattern groups and
# callbacks because the same character has different meaning in
# different contexts (e.g., "=" is an attribute delimiter inside
# tags but plain text content outside tags).
#
# We test:
#   1. Basic elements -- open/close tags, text content
#   2. Attributes -- single and double quoted values
#   3. Self-closing tags -- <br/>
#   4. Comments -- <!-- ... -->
#   5. CDATA sections -- <![CDATA[ ... ]]>
#   6. Processing instructions -- <?xml ... ?>
#   7. Entity references -- &amp;, &#65;, &#x41;
#   8. Nested structures -- tags within tags
#   9. Mixed content -- text interspersed with elements
#   10. Edge cases -- empty elements, whitespace handling
# ================================================================

class TestXmlLexer < Minitest::Test
  # ------------------------------------------------------------------
  # Helper: tokenize source and provide convenient accessors
  # ------------------------------------------------------------------

  # Tokenize XML and return (type_name, value) pairs without EOF.
  #
  # The token type may be a string or a TokenType enum value,
  # so we normalize to a string for consistent comparisons.
  def token_pairs(source)
    tokens = CodingAdventures::XmlLexer.tokenize(source)
    tokens.filter_map { |t|
      name = t.type.is_a?(String) ? t.type : t.type.to_s
      next if name == "EOF"
      [name, t.value]
    }
  end

  # Tokenize XML and return just the type names without EOF.
  def token_types(source)
    token_pairs(source).map(&:first)
  end

  # Extract values for a given token type from pairs.
  # This avoids the select { |t, _| } pattern that standardrb flags.
  def values_for_type(pairs, type_name)
    pairs.filter_map { |pair| pair[1] if pair[0] == type_name }
  end

  # Count occurrences of a given token type in pairs.
  def count_type(pairs, type_name)
    pairs.count { |pair| pair[0] == type_name }
  end

  # ------------------------------------------------------------------
  # Basic Tags
  # ------------------------------------------------------------------
  # These tests verify the fundamental open/close tag structure.
  # A simple XML element like <p>text</p> produces:
  #   OPEN_TAG_START, TAG_NAME, TAG_CLOSE, TEXT,
  #   CLOSE_TAG_START, TAG_NAME, TAG_CLOSE

  def test_simple_element
    pairs = token_pairs("<p>text</p>")
    assert_equal [
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "p"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "text"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "p"],
      ["TAG_CLOSE", ">"]
    ], pairs
  end

  def test_element_with_namespace
    # Tags with XML namespace prefixes: <ns:tag>
    # The colon is part of the TAG_NAME pattern.
    types = token_types("<ns:tag>content</ns:tag>")
    assert_equal [
      "OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE",
      "TEXT",
      "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE"
    ], types
    pairs = token_pairs("<ns:tag>content</ns:tag>")
    assert_equal ["TAG_NAME", "ns:tag"], pairs[1]
  end

  def test_empty_element_explicit
    # An explicitly empty element: <div></div>
    pairs = token_pairs("<div></div>")
    assert_equal [
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "div"],
      ["TAG_CLOSE", ">"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "div"],
      ["TAG_CLOSE", ">"]
    ], pairs
  end

  def test_self_closing_tag
    # Self-closing tag: <br/>
    pairs = token_pairs("<br/>")
    assert_equal [
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "br"],
      ["SELF_CLOSE", "/>"]
    ], pairs
  end

  def test_self_closing_with_space
    # Self-closing with space: <br />
    pairs = token_pairs("<br />")
    assert_equal [
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "br"],
      ["SELF_CLOSE", "/>"]
    ], pairs
  end

  # ------------------------------------------------------------------
  # Attributes
  # ------------------------------------------------------------------
  # Inside a tag, the lexer recognizes attribute names (same regex
  # as TAG_NAME), equals signs, and quoted attribute values.

  def test_double_quoted_attribute
    pairs = token_pairs('<div class="main">')
    assert_equal [
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "div"],
      ["TAG_NAME", "class"],
      ["ATTR_EQUALS", "="],
      ["ATTR_VALUE", '"main"'],
      ["TAG_CLOSE", ">"]
    ], pairs
  end

  def test_single_quoted_attribute
    pairs = token_pairs("<div class='main'>")
    assert_equal [
      ["OPEN_TAG_START", "<"],
      ["TAG_NAME", "div"],
      ["TAG_NAME", "class"],
      ["ATTR_EQUALS", "="],
      ["ATTR_VALUE", "'main'"],
      ["TAG_CLOSE", ">"]
    ], pairs
  end

  def test_multiple_attributes
    pairs = token_pairs('<a href="url" target="_blank">')
    tag_names = values_for_type(pairs, "TAG_NAME")
    assert_equal ["a", "href", "target"], tag_names
    attr_values = values_for_type(pairs, "ATTR_VALUE")
    assert_equal ['"url"', '"_blank"'], attr_values
  end

  def test_attribute_on_self_closing
    types = token_types('<img src="photo.jpg"/>')
    assert_includes types, "SELF_CLOSE"
    assert_includes types, "ATTR_VALUE"
  end

  # ------------------------------------------------------------------
  # Comments
  # ------------------------------------------------------------------
  # XML comments (<!-- ... -->) switch to the "comment" group.
  # Skip patterns are disabled so whitespace is preserved.

  def test_simple_comment
    pairs = token_pairs("<!-- hello -->")
    assert_equal [
      ["COMMENT_START", "<!--"],
      ["COMMENT_TEXT", " hello "],
      ["COMMENT_END", "-->"]
    ], pairs
  end

  def test_comment_preserves_whitespace
    # Whitespace inside comments is preserved (skip disabled).
    pairs = token_pairs("<!--  spaces  and\ttabs  -->")
    text = values_for_type(pairs, "COMMENT_TEXT")
    assert_equal ["  spaces  and\ttabs  "], text
  end

  def test_comment_with_dashes
    # Comments can contain single dashes (but not --).
    pairs = token_pairs("<!-- a-b-c -->")
    text = values_for_type(pairs, "COMMENT_TEXT")
    assert_equal [" a-b-c "], text
  end

  def test_comment_between_elements
    types = token_types("<a/><!-- mid --><b/>")
    assert_includes types, "COMMENT_START"
    assert_includes types, "COMMENT_END"
  end

  # ------------------------------------------------------------------
  # CDATA Sections
  # ------------------------------------------------------------------
  # CDATA sections (<![CDATA[ ... ]]>) contain raw text.
  # No entity processing, no tag recognition.

  def test_simple_cdata
    pairs = token_pairs("<![CDATA[raw text]]>")
    assert_equal [
      ["CDATA_START", "<![CDATA["],
      ["CDATA_TEXT", "raw text"],
      ["CDATA_END", "]]>"]
    ], pairs
  end

  def test_cdata_with_angle_brackets
    # CDATA can contain < and > which are normally special.
    pairs = token_pairs("<![CDATA[<not a tag>]]>")
    text = values_for_type(pairs, "CDATA_TEXT")
    assert_equal ["<not a tag>"], text
  end

  def test_cdata_preserves_whitespace
    pairs = token_pairs("<![CDATA[  hello\n  world  ]]>")
    text = values_for_type(pairs, "CDATA_TEXT")
    assert_equal ["  hello\n  world  "], text
  end

  def test_cdata_with_single_bracket
    # CDATA can contain ] without ending (needs ]]>).
    pairs = token_pairs("<![CDATA[a]b]]>")
    text = values_for_type(pairs, "CDATA_TEXT")
    assert_equal ["a]b"], text
  end

  # ------------------------------------------------------------------
  # Processing Instructions
  # ------------------------------------------------------------------
  # Processing instructions (<?target content?>) switch to the
  # "pi" group. The target name and text content are separate tokens.

  def test_xml_declaration
    pairs = token_pairs('<?xml version="1.0"?>')
    assert_equal [
      ["PI_START", "<?"],
      ["PI_TARGET", "xml"],
      ["PI_TEXT", ' version="1.0"'],
      ["PI_END", "?>"]
    ], pairs
  end

  def test_stylesheet_pi
    types = token_types('<?xml-stylesheet type="text/xsl"?>')
    assert_equal "PI_START", types[0]
    assert_equal "PI_TARGET", types[1]
    assert_equal "PI_END", types[-1]
  end

  # ------------------------------------------------------------------
  # Entity and Character References
  # ------------------------------------------------------------------
  # Entity references (&name;) and character references (&#NNN; or
  # &#xHHH;) are recognized in the default group (text content).

  def test_named_entity
    pairs = token_pairs("a&amp;b")
    assert_equal [
      ["TEXT", "a"],
      ["ENTITY_REF", "&amp;"],
      ["TEXT", "b"]
    ], pairs
  end

  def test_decimal_char_ref
    pairs = token_pairs("&#65;")
    assert_equal [["CHAR_REF", "&#65;"]], pairs
  end

  def test_hex_char_ref
    pairs = token_pairs("&#x41;")
    assert_equal [["CHAR_REF", "&#x41;"]], pairs
  end

  def test_multiple_entities
    types = token_types("&lt;hello&gt;")
    assert_equal ["ENTITY_REF", "TEXT", "ENTITY_REF"], types
  end

  # ------------------------------------------------------------------
  # Nested and Mixed Content
  # ------------------------------------------------------------------
  # Verify that group transitions work correctly across nesting
  # levels and mixed text/element content.

  def test_nested_elements
    types = token_types("<a><b>text</b></a>")
    # Should have two OPEN_TAG_START and two CLOSE_TAG_START
    assert_equal 2, types.count("OPEN_TAG_START")
    assert_equal 2, types.count("CLOSE_TAG_START")
  end

  def test_mixed_content
    # Text mixed with child elements.
    pairs = token_pairs("<p>Hello <b>world</b>!</p>")
    texts = values_for_type(pairs, "TEXT")
    assert_equal ["Hello ", "world", "!"], texts
  end

  def test_full_document
    # A small but complete XML document with PI, comment, tags,
    # attributes, and entity references.
    source = '<?xml version="1.0"?>' \
             "<!-- A greeting -->" \
             '<root lang="en">' \
             "<greeting>Hello &amp; welcome</greeting>" \
             "</root>"
    tokens = CodingAdventures::XmlLexer.tokenize(source)
    types = tokens.map { |t| t.type.is_a?(String) ? t.type : t.type.to_s }

    # PI present
    assert_includes types, "PI_START"
    assert_includes types, "PI_END"

    # Comment present
    assert_includes types, "COMMENT_START"
    assert_includes types, "COMMENT_END"

    # Tags present
    assert_equal 2, types.count("OPEN_TAG_START") # root + greeting
    assert_equal 2, types.count("CLOSE_TAG_START")

    # Entity ref present
    assert_includes types, "ENTITY_REF"

    # Ends with EOF
    assert_equal "EOF", types[-1]
  end

  def test_cdata_inside_element
    source = "<script><![CDATA[x < y]]></script>"
    types = token_types(source)
    assert_includes types, "CDATA_START"
    assert_includes types, "CDATA_TEXT"
    assert_includes types, "CDATA_END"
  end

  # ------------------------------------------------------------------
  # Edge Cases
  # ------------------------------------------------------------------

  def test_empty_string
    # Empty input produces only EOF.
    tokens = CodingAdventures::XmlLexer.tokenize("")
    assert_equal 1, tokens.length
    name = tokens[0].type.is_a?(String) ? tokens[0].type : tokens[0].type.to_s
    assert_equal "EOF", name
  end

  def test_text_only
    # Plain text with no tags.
    pairs = token_pairs("just text")
    assert_equal [["TEXT", "just text"]], pairs
  end

  def test_whitespace_between_tags_skipped
    # Whitespace between tags is consumed by skip patterns.
    # The XML grammar has a skip pattern for whitespace, so spaces
    # between tags are silently consumed.
    pairs = token_pairs("<a> <b> </b> </a>")
    texts = values_for_type(pairs, "TEXT")
    assert_equal [], texts # whitespace consumed by skip pattern
  end

  def test_eof_always_last
    # The last token is always EOF.
    tokens = CodingAdventures::XmlLexer.tokenize("<root/>")
    name = tokens[-1].type.is_a?(String) ? tokens[-1].type : tokens[-1].type.to_s
    assert_equal "EOF", name
  end

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::XmlLexer::XML_TOKENS_PATH),
      "xml.tokens file should exist at #{CodingAdventures::XmlLexer::XML_TOKENS_PATH}"
  end

  # ------------------------------------------------------------------
  # create_xml_lexer API
  # ------------------------------------------------------------------
  # Test the factory method that returns a configured GrammarLexer.

  def test_create_xml_lexer
    lexer = CodingAdventures::XmlLexer.create_xml_lexer("<p/>")
    tokens = lexer.tokenize
    types = tokens.map { |t| t.type.is_a?(String) ? t.type : t.type.to_s }
    assert_includes types, "OPEN_TAG_START"
    assert_includes types, "SELF_CLOSE"
    assert_includes types, "EOF"
  end

  # ------------------------------------------------------------------
  # Deeply nested elements
  # ------------------------------------------------------------------

  def test_deeply_nested
    source = "<a><b><c>deep</c></b></a>"
    types = token_types(source)
    assert_equal 3, types.count("OPEN_TAG_START")
    assert_equal 3, types.count("CLOSE_TAG_START")
    texts = values_for_type(token_pairs(source), "TEXT")
    assert_equal ["deep"], texts
  end

  # ------------------------------------------------------------------
  # Multiple comments
  # ------------------------------------------------------------------

  def test_multiple_comments
    source = "<!-- one --><!-- two -->"
    types = token_types(source)
    assert_equal 2, types.count("COMMENT_START")
    assert_equal 2, types.count("COMMENT_END")
  end

  # ------------------------------------------------------------------
  # PI followed by content
  # ------------------------------------------------------------------

  def test_pi_followed_by_content
    source = "<?xml version=\"1.0\"?><root/>"
    types = token_types(source)
    assert_equal "PI_START", types[0]
    assert_includes types, "OPEN_TAG_START"
    assert_includes types, "SELF_CLOSE"
  end

  # ------------------------------------------------------------------
  # Tag with many attributes
  # ------------------------------------------------------------------

  def test_tag_with_many_attributes
    source = '<div id="x" class="y" data-val="z">'
    pairs = token_pairs(source)
    attr_eq_count = count_type(pairs, "ATTR_EQUALS")
    assert_equal 3, attr_eq_count
    attr_vals = values_for_type(pairs, "ATTR_VALUE")
    assert_equal ['"x"', '"y"', '"z"'], attr_vals
  end

  # ------------------------------------------------------------------
  # Entity ref between elements
  # ------------------------------------------------------------------

  def test_entity_between_elements
    source = "<a/>&amp;<b/>"
    types = token_types(source)
    assert_includes types, "ENTITY_REF"
  end
end
