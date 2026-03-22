# frozen_string_literal: true

# ================================================================
# XML Lexer -- Tokenizes XML Text Using Pattern Groups and Callbacks
# ================================================================
#
# This module is the first Ruby lexer wrapper that uses **pattern
# groups** and an **on-token callback** to handle context-sensitive
# lexical structure. Unlike the JSON lexer (which uses a flat pattern
# list), the XML lexer must distinguish between different contexts:
#
# Context-Sensitive Lexing
# ------------------------
#
# XML is context-sensitive at the lexical level. The same character
# has different meaning depending on where it appears:
#
#   - "=" is an attribute delimiter inside <tag attr="val">
#   - "=" is plain text content outside tags: 1 + 1 = 2
#
# A flat pattern list cannot distinguish these contexts. Pattern
# groups solve this by defining separate sets of patterns for each
# context, and a callback function switches between them at runtime.
#
# The xml.tokens Grammar
# ----------------------
#
# The grammar defines 5 pattern groups:
#
#   - **default** (implicit): Text content, entity refs, tag openers
#   - **tag**: Tag names, attributes, equals, quoted values, closers
#   - **comment**: Comment text and "-->" delimiter
#   - **cdata**: Raw text and "]]>" delimiter
#   - **pi**: Processing instruction target, text, and "?>" delimiter
#
# The Callback (xml_on_token)
# ---------------------------
#
# The callback fires after each token match and drives group
# transitions. It follows a simple state machine:
#
#   default --OPEN_TAG_START--> tag --TAG_CLOSE--> default
#           --CLOSE_TAG_START-> tag --SELF_CLOSE-> default
#           --COMMENT_START---> comment --COMMENT_END--> default
#           --CDATA_START-----> cdata --CDATA_END--> default
#           --PI_START--------> pi --PI_END--> default
#
# For comment, CDATA, and PI groups, the callback also disables
# skip patterns (so whitespace is preserved as content) and
# re-enables them when leaving the group.
#
# Usage:
#   tokens = CodingAdventures::XmlLexer.tokenize('<div class="main">Hello</div>')
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module XmlLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/xml_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       xml.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         xml_lexer/
    #           lib/
    #             coding_adventures/
    #               xml_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    XML_TOKENS_PATH = File.join(GRAMMAR_DIR, "xml.tokens")

    # ================================================================
    # XML On-Token Callback
    # ================================================================
    #
    # This proc drives the group transitions. It is a pure function
    # of the token type -- no external state is needed. The
    # LexerContext provides all the control we need (push/pop groups,
    # toggle skip).
    #
    # The pattern is simple:
    # - Opening delimiters push a group
    # - Closing delimiters pop the group
    # - Comment/CDATA/PI groups disable skip (whitespace is content)
    # ================================================================

    # Build the on-token callback for XML group switching.
    #
    # This callback fires after each token match. It examines the
    # token type and pushes/pops pattern groups accordingly:
    #
    # - OPEN_TAG_START (<) or CLOSE_TAG_START (</)
    #   Push the "tag" group so the lexer recognizes tag names,
    #   attributes, and tag closers.
    #
    # - TAG_CLOSE (>) or SELF_CLOSE (/>)
    #   Pop the "tag" group to return to default (text content).
    #
    # - COMMENT_START (<!--)
    #   Push "comment" group and disable skip (whitespace matters).
    #
    # - COMMENT_END (-->)
    #   Pop "comment" group and re-enable skip.
    #
    # - CDATA_START (<![CDATA[)
    #   Push "cdata" group and disable skip.
    #
    # - CDATA_END (]]>)
    #   Pop "cdata" group and re-enable skip.
    #
    # - PI_START (<?)
    #   Push "pi" group and disable skip.
    #
    # - PI_END (?>)
    #   Pop "pi" group and re-enable skip.
    #
    # @param token [CodingAdventures::Lexer::Token] the matched token
    # @param ctx [CodingAdventures::Lexer::LexerContext] the context
    XML_ON_TOKEN = proc { |token, ctx|
      # The token type may be a string or a TokenType enum value.
      # Normalize to a string for matching.
      token_type = token.type.is_a?(String) ? token.type : token.type.to_s

      case token_type
      # --- Tag boundaries ---
      when "OPEN_TAG_START", "CLOSE_TAG_START"
        ctx.push_group("tag")
      when "TAG_CLOSE", "SELF_CLOSE"
        ctx.pop_group

      # --- Comment boundaries ---
      when "COMMENT_START"
        ctx.push_group("comment")
        ctx.set_skip_enabled(false)
      when "COMMENT_END"
        ctx.pop_group
        ctx.set_skip_enabled(true)

      # --- CDATA boundaries ---
      when "CDATA_START"
        ctx.push_group("cdata")
        ctx.set_skip_enabled(false)
      when "CDATA_END"
        ctx.pop_group
        ctx.set_skip_enabled(true)

      # --- Processing instruction boundaries ---
      when "PI_START"
        ctx.push_group("pi")
        ctx.set_skip_enabled(false)
      when "PI_END"
        ctx.pop_group
        ctx.set_skip_enabled(true)
      end
    }

    # ================================================================
    # Public API
    # ================================================================

    # Create a GrammarLexer configured for XML text.
    #
    # This method reads the xml.tokens file, parses it into a
    # TokenGrammar, creates a GrammarLexer, and registers the XML
    # on-token callback for pattern group switching.
    #
    # @param source [String] the XML text to tokenize
    # @return [CodingAdventures::Lexer::GrammarLexer] configured lexer
    def self.create_xml_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(XML_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.set_on_token(XML_ON_TOKEN)
      lexer
    end

    # Tokenize XML text and return an array of Token objects.
    #
    # This is the main entry point for the XML lexer. Pass in a
    # string of XML text, and get back a flat array of Token objects.
    # The array always ends with an EOF token.
    #
    # Token types you will see:
    #
    # **Default group** (content between tags):
    #   - TEXT -- text content (e.g., "Hello world")
    #   - ENTITY_REF -- entity reference (e.g., "&amp;")
    #   - CHAR_REF -- character reference (e.g., "&#65;", "&#x41;")
    #   - OPEN_TAG_START -- "<"
    #   - CLOSE_TAG_START -- "</"
    #   - COMMENT_START -- "<!--"
    #   - CDATA_START -- "<![CDATA["
    #   - PI_START -- "<?"
    #
    # **Tag group** (inside tags):
    #   - TAG_NAME -- tag or attribute name (e.g., "div", "class")
    #   - ATTR_EQUALS -- "="
    #   - ATTR_VALUE -- quoted attribute value (e.g., '"main"')
    #   - TAG_CLOSE -- ">"
    #   - SELF_CLOSE -- "/>"
    #
    # **Comment group**:
    #   - COMMENT_TEXT -- comment content
    #   - COMMENT_END -- "-->"
    #
    # **CDATA group**:
    #   - CDATA_TEXT -- raw text content
    #   - CDATA_END -- "]]>"
    #
    # **Processing instruction group**:
    #   - PI_TARGET -- PI target name (e.g., "xml")
    #   - PI_TEXT -- PI content
    #   - PI_END -- "?>"
    #
    # **Always present**:
    #   - EOF -- end of input
    #
    # @param source [String] the XML text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      create_xml_lexer(source).tokenize
    end
  end
end
