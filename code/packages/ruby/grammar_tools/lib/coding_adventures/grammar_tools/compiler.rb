# frozen_string_literal: true

require "coding_adventures_grammar_tools"

module CodingAdventures
  module GrammarTools
    module Compiler
      # Compiles a TokenGrammar object into a Ruby source file.
      def self.compile_tokens_to_ruby(grammar, module_name)
        lines = []
        lines << "# frozen_string_literal: true"
        lines << "# AUTO-GENERATED FILE - DO NOT EDIT"
        lines << "require \"coding_adventures_grammar_tools\""
        lines << ""
        lines << "module CodingAdventures"
        lines << "  module #{module_name}"
        lines << "    def self.grammar"
        lines << "      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new("
        
        # definitions
        lines << "        definitions: ["
        grammar.definitions.each do |d|
          lines << "          #{compile_token_def_ruby(d)},"
        end
        lines << "        ],"
        
        # keywords
        lines << "        keywords: #{grammar.keywords.inspect},"
        
        # mode
        lines << "        mode: #{grammar.mode.inspect},"
        
        # skip_definitions
        lines << "        skip_definitions: ["
        grammar.skip_definitions.each do |d|
          lines << "          #{compile_token_def_ruby(d)},"
        end
        lines << "        ],"
        
        # error_definitions
        lines << "        error_definitions: ["
        grammar.error_definitions.each do |d|
          lines << "          #{compile_token_def_ruby(d)},"
        end
        lines << "        ],"

        # reserved_keywords
        lines << "        reserved_keywords: #{grammar.reserved_keywords.inspect},"
        
        # escape_mode
        lines << "        escape_mode: #{grammar.escape_mode.inspect},"
        
        # groups
        lines << "        groups: {"
        grammar.groups.each do |gname, group|
          lines << "          #{gname.inspect} => CodingAdventures::GrammarTools::PatternGroup.new("
          lines << "            name: #{group.name.inspect},"
          lines << "            definitions: ["
          group.definitions.each do |d|
            lines << "              #{compile_token_def_ruby(d)},"
          end
          lines << "            ]"
          lines << "          ),"
        end
        lines << "        },"
        
        # boolean/scalar properties
        lines << "        case_sensitive: #{grammar.case_sensitive.inspect},"
        lines << "        version: #{grammar.version.inspect},"
        lines << "        case_insensitive: #{grammar.case_insensitive.inspect}"
        lines << "      )"
        lines << "    end"
        lines << "  end"
        lines << "end"
        
        lines.join("\n") + "\n"
      end

      # Compiles a ParserGrammar object into a Ruby source file.
      def self.compile_parser_to_ruby(grammar, module_name)
        lines = []
        lines << "# frozen_string_literal: true"
        lines << "# AUTO-GENERATED FILE - DO NOT EDIT"
        lines << "require \"coding_adventures_grammar_tools\""
        lines << ""
        lines << "module CodingAdventures"
        lines << "  module #{module_name}"
        lines << "    def self.grammar"
        lines << "      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new("
        lines << "        version: #{grammar.version.inspect},"
        lines << "        rules: ["
        grammar.rules.each do |rule|
          lines << "          CodingAdventures::GrammarTools::GrammarRule.new("
          lines << "            name: #{rule.name.inspect},"
          lines << "            line_number: #{rule.line_number.inspect},"
          lines << "            body: #{compile_ebnf_node_ruby(rule.body)}"
          lines << "          ),"
        end
        lines << "        ]"
        lines << "      )"
        lines << "    end"
        lines << "  end"
        lines << "end"

        lines.join("\n") + "\n"
      end

      private

      def self.compile_token_def_ruby(d)
        "CodingAdventures::GrammarTools::TokenDefinition.new(name: #{d.name.inspect}, pattern: #{d.pattern.inspect}, is_regex: #{d.is_regex.inspect}, line_number: #{d.line_number.inspect}, alias_name: #{d.alias_name.inspect})"
      end

      def self.compile_ebnf_node_ruby(node)
        case node
        when CodingAdventures::GrammarTools::RuleReference
          "CodingAdventures::GrammarTools::RuleReference.new(name: #{node.name.inspect}, is_token: #{node.is_token.inspect})"
        when CodingAdventures::GrammarTools::Literal
          "CodingAdventures::GrammarTools::Literal.new(value: #{node.value.inspect})"
        when CodingAdventures::GrammarTools::Sequence
          elements = node.elements.map { |e| compile_ebnf_node_ruby(e) }.join(", ")
          "CodingAdventures::GrammarTools::Sequence.new(elements: [#{elements}])"
        when CodingAdventures::GrammarTools::Alternation
          choices = node.choices.map { |c| compile_ebnf_node_ruby(c) }.join(", ")
          "CodingAdventures::GrammarTools::Alternation.new(choices: [#{choices}])"
        when CodingAdventures::GrammarTools::Repetition
          "CodingAdventures::GrammarTools::Repetition.new(element: #{compile_ebnf_node_ruby(node.element)})"
        when CodingAdventures::GrammarTools::OptionalElement
          "CodingAdventures::GrammarTools::OptionalElement.new(element: #{compile_ebnf_node_ruby(node.element)})"
        when CodingAdventures::GrammarTools::Group
          "CodingAdventures::GrammarTools::Group.new(element: #{compile_ebnf_node_ruby(node.element)})"
        else
          raise "Unknown node type: #{node.class}"
        end
      end
    end
  end
end
