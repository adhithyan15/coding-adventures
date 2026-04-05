# frozen_string_literal: true

# ================================================================
# Mosaic Parser -- Parses Mosaic Source into an AST from Ruby
# ================================================================
#
# This module is the syntactic analysis stage for the Mosaic
# Component Description Language (CDL). It follows the same
# grammar-driven pattern as the JSON parser:
#
#   1. Tokenize using MosaicLexer (loads mosaic.tokens)
#   2. Load mosaic.grammar from the grammars directory
#   3. Parse using GrammarDrivenParser → AST
#
# Mosaic grammar rules (abbreviated):
#
#   file           = { import_decl } component_decl
#   component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE
#   slot_decl      = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
#   node_element   = NAME LBRACE { node_content } RBRACE
#   property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON
#   when_block     = KEYWORD slot_ref LBRACE { node_content } RBRACE
#   each_block     = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
#
# The AST produced here is consumed by MosaicAnalyzer which strips
# syntax noise and produces a typed MosaicIR.
#
# Usage:
#   ast = CodingAdventures::MosaicParser.parse(source)
#   # => ASTNode(rule_name: "file", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"

module CodingAdventures
  module MosaicParser
    # Path to mosaic.grammar, computed relative to this file.
    # Navigate up from lib/coding_adventures/mosaic_parser/ to code/grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    MOSAIC_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "mosaic.grammar")

    # Parse a string of Mosaic source into a generic AST.
    #
    # Pipeline:
    # 1. Tokenize using MosaicLexer (reads mosaic.tokens)
    # 2. Load mosaic.grammar and build a ParserGrammar
    # 3. Fix slot_type alternative ordering (list_type first)
    # 4. Feed tokens + grammar into GrammarDrivenParser
    # 5. Return the root ASTNode (rule_name: "file")
    #
    # Grammar Note: The mosaic.grammar file lists slot_type alternatives as:
    #   slot_type = KEYWORD | NAME | list_type
    #
    # For a first-come-first-served recursive descent parser, this ordering
    # causes `list<text>` to fail: the parser matches KEYWORD("list") as a
    # complete slot_type, then fails to find SEMICOLON because LANGLE follows.
    # The fix is to try `list_type` first (as the TypeScript generated grammar
    # does — see mosaic-parser/src/_grammar.ts, where list_type comes first).
    #
    # @param source [String] Mosaic source text to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)

      # Step 1: Tokenize using the Mosaic lexer.
      tokens = CodingAdventures::MosaicLexer.tokenize(source)

      # Step 2: Load and parse the Mosaic grammar.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(MOSAIC_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Reorder alternatives to avoid greedy-match failures.
      # Two issues in the grammar:
      #   1. slot_type: list_type must come before KEYWORD (list<text> fix)
      #   2. property_value: enum_value must come before NAME (heading.large fix)
      grammar = fix_slot_type_ordering(grammar)
      grammar = fix_property_value_ordering(grammar)

      # Step 4: Parse tokens using the grammar-driven parser.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end

    # Rebuild the grammar with list_type first in the slot_type alternation.
    #
    # The grammar objects are immutable Ruby Data types, so we must construct
    # new instances rather than mutating.
    def self.fix_slot_type_ordering(grammar)
      slot_type_rule = grammar.rules.find { |r| r.name == "slot_type" }
      return grammar unless slot_type_rule

      choices = slot_type_rule.body.choices
      list_type_choice = choices.find { |c| !c.is_token && c.name == "list_type" }
      return grammar unless list_type_choice

      other_choices = choices.reject { |c| !c.is_token && c.name == "list_type" }
      new_alternation = CodingAdventures::GrammarTools::Alternation.new(choices: [list_type_choice] + other_choices)
      new_rule = CodingAdventures::GrammarTools::GrammarRule.new(
        name: "slot_type",
        body: new_alternation,
        line_number: slot_type_rule.line_number
      )
      new_rules = grammar.rules.map { |r| r.name == "slot_type" ? new_rule : r }
      CodingAdventures::GrammarTools::ParserGrammar.new(version: grammar.version, rules: new_rules)
    end
    private_class_method :fix_slot_type_ordering

    # Rebuild the grammar with enum_value first in the property_value alternation.
    #
    # The grammar file has NAME before enum_value, but a greedy parser would
    # match "heading" as NAME alone, leaving ".large" unparsed. Since enum_value
    # = NAME DOT NAME is more specific (starts with NAME followed by DOT), it
    # must come before the bare NAME alternative.
    def self.fix_property_value_ordering(grammar)
      prop_val_rule = grammar.rules.find { |r| r.name == "property_value" }
      return grammar unless prop_val_rule

      choices = prop_val_rule.body.choices
      enum_choice = choices.find { |c| !c.is_token && c.name == "enum_value" }
      return grammar unless enum_choice

      # Remove enum_value from its current position and insert before NAME
      other_choices = choices.reject { |c| !c.is_token && c.name == "enum_value" }
      # Find where NAME appears and insert enum_value before it
      name_idx = other_choices.index { |c| c.is_token && c.name == "NAME" }
      reordered = if name_idx
                    other_choices[0...name_idx] + [enum_choice] + other_choices[name_idx..]
                  else
                    other_choices + [enum_choice]
                  end

      new_alternation = CodingAdventures::GrammarTools::Alternation.new(choices: reordered)
      new_rule = CodingAdventures::GrammarTools::GrammarRule.new(
        name: "property_value",
        body: new_alternation,
        line_number: prop_val_rule.line_number
      )
      new_rules = grammar.rules.map { |r| r.name == "property_value" ? new_rule : r }
      CodingAdventures::GrammarTools::ParserGrammar.new(version: grammar.version, rules: new_rules)
    end
    private_class_method :fix_property_value_ordering
  end
end
