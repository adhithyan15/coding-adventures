# frozen_string_literal: true

# ==========================================================================
# compiler.rb -- Compile TokenGrammar and ParserGrammar into Ruby source code
# ==========================================================================
#
# The grammar-tools library parses .tokens and .grammar files into in-memory
# data structures. This module adds the *compile* step: given a parsed grammar
# object, generate Ruby source code that embeds the grammar as native Ruby
# data structures.
#
# Why compile grammars?
# ---------------------
#
# The default workflow reads .tokens and .grammar files at runtime. This has
# three costs that compilation eliminates:
#
#   1. File I/O at startup -- every process must find and open the files.
#      Packages walk up the directory tree to find code/grammars/, which
#      couples them to the repo layout.
#
#   2. Parse overhead at startup -- the grammar is re-parsed every run.
#
#   3. Deployment coupling -- .tokens and .grammar files must ship with the
#      program alongside the compiled binary.
#
# The generated Ruby file directly instantiates TokenGrammar or ParserGrammar
# with literal data and can be required like any other Ruby file.
#
# Generated output shape (json.tokens => json_tokens.rb):
#
#   # AUTO-GENERATED FILE -- DO NOT EDIT
#   # Source: json.tokens
#
#   require "coding_adventures_grammar_tools"
#   GT = CodingAdventures::GrammarTools unless defined?(GT)
#
#   TOKEN_GRAMMAR = GT::TokenGrammar.new(
#     version: 1,
#     case_sensitive: true,
#     definitions: [
#       GT::TokenDefinition.new(name: "STRING", ...),
#     ],
#     ...
#   )
#
# Design note: helper methods are defined as module_function so they can be
# called both as Compiler.method_name(...) and as module functions within
# the module body. We do NOT use `private` here because private module methods
# cannot be called from other module_function methods in Ruby.
# ==========================================================================

module CodingAdventures
  module GrammarTools
    # Compiler generates Ruby source code from parsed grammar objects.
    module Compiler
      module_function

      # Generate Ruby source code embedding a TokenGrammar as native data.
      #
      # Parameters:
      #   grammar     -- a TokenGrammar object to compile
      #   source_file -- the original .tokens filename for the header comment
      #                  (pass "" to omit)
      #
      # Returns a String of valid Ruby source code. Write it to a .rb file.
      def compile_token_grammar(grammar, source_file = "")
        # Strip newlines so a crafted filename cannot break out of the comment
        # line and inject arbitrary code into the generated file.
        source_file = source_file.gsub(/[\r\n]/, "_")
        source_line = source_file.empty? ? "" : "# Source: #{source_file}\n"

        defs_src   = token_def_list_src(grammar.definitions, "    ")
        skip_src   = token_def_list_src(grammar.skip_definitions, "    ")
        err_src    = token_def_list_src(grammar.error_definitions, "    ")
        groups_src = build_groups_src(grammar.groups, "    ")

        <<~RUBY
          # frozen_string_literal: true
          # AUTO-GENERATED FILE \u2014 DO NOT EDIT
          #{source_line}# Regenerate with: grammar-tools compile-tokens #{source_file}
          #
          # This file embeds a TokenGrammar as native Ruby data structures.
          # Downstream packages require this file directly instead of reading
          # and parsing the .tokens file at runtime.

          require "coding_adventures_grammar_tools"

          GT = CodingAdventures::GrammarTools unless defined?(GT)

          TOKEN_GRAMMAR = GT::TokenGrammar.new(
            version: #{grammar.version.inspect},
            case_insensitive: #{grammar.case_insensitive.inspect},
            case_sensitive: #{grammar.case_sensitive.inspect},
            definitions: #{defs_src},
            keywords: #{grammar.keywords.inspect},
            mode: #{grammar.mode.inspect},
            escape_mode: #{grammar.escape_mode.inspect},
            skip_definitions: #{skip_src},
            reserved_keywords: #{grammar.reserved_keywords.inspect},
            error_definitions: #{err_src},
            groups: #{groups_src},
            layout_keywords: #{grammar.layout_keywords.inspect},
            context_keywords: #{grammar.context_keywords.inspect},
            soft_keywords: #{grammar.soft_keywords.inspect},
          )
        RUBY
      end

      # Generate Ruby source code embedding a ParserGrammar as native data.
      #
      # Parameters:
      #   grammar     -- a ParserGrammar object to compile
      #   source_file -- the original .grammar filename for the header comment
      #
      # Returns a String of valid Ruby source code.
      def compile_parser_grammar(grammar, source_file = "")
        # Strip newlines so a crafted filename cannot break out of the comment line.
        source_file = source_file.gsub(/[\r\n]/, "_")
        source_line = source_file.empty? ? "" : "# Source: #{source_file}\n"

        if grammar.rules.empty?
          rules_src = "[]"
        else
          rule_lines = grammar.rules.map { |r| grammar_rule_src(r, "    ") }
          rules_src  = "[\n#{rule_lines.join(",\n")},\n  ]"
        end

        <<~RUBY
          # frozen_string_literal: true
          # AUTO-GENERATED FILE \u2014 DO NOT EDIT
          #{source_line}# Regenerate with: grammar-tools compile-grammar #{source_file}
          #
          # This file embeds a ParserGrammar as native Ruby data structures.
          # Downstream packages require this file directly instead of reading
          # and parsing the .grammar file at runtime.

          require "coding_adventures_grammar_tools"

          GT = CodingAdventures::GrammarTools unless defined?(GT)

          PARSER_GRAMMAR = GT::ParserGrammar.new(
            version: #{grammar.version.inspect},
            rules: #{rules_src},
          )
        RUBY
      end

      # Render one TokenDefinition as a constructor call string.
      def token_def_src(defn, indent)
        i = indent + "  "
        [
          "#{indent}GT::TokenDefinition.new(",
          "#{i}name: #{defn.name.inspect},",
          "#{i}pattern: #{defn.pattern.inspect},",
          "#{i}is_regex: #{defn.is_regex.inspect},",
          "#{i}line_number: #{defn.line_number.inspect},",
          "#{i}alias_name: #{defn.alias_name.inspect},",
          "#{indent})"
        ].join("\n")
      end

      # Render a list of TokenDefinitions. Returns "[]" for empty lists.
      def token_def_list_src(defs, indent)
        return "[]" if defs.empty?

        inner = indent + "  "
        items = defs.map { |d| token_def_src(d, inner) }.join(",\n")
        "[\n#{items},\n#{indent}]"
      end

      # Render the groups hash. Returns "{}" for empty.
      def build_groups_src(groups, indent)
        return "{}" if groups.empty?

        inner = indent + "  "
        entries = groups.map do |name, group|
          defs_lit = token_def_list_src(group.definitions, inner + "  ")
          [
            "#{inner}#{name.inspect} => GT::PatternGroup.new(",
            "#{inner}  name: #{group.name.inspect},",
            "#{inner}  definitions: #{defs_lit},",
            "#{inner})"
          ].join("\n")
        end
        "{\n#{entries.join(",\n")},\n#{indent}}"
      end

      # Render a GrammarRule as a constructor call string.
      def grammar_rule_src(rule, indent)
        i = indent + "  "
        body_src = element_src(rule.body, i)
        [
          "#{indent}GT::GrammarRule.new(",
          "#{i}name: #{rule.name.inspect},",
          "#{i}body: #{body_src},",
          "#{i}line_number: #{rule.line_number.inspect},",
          "#{indent})"
        ].join("\n")
      end

      # Recursively render a GrammarElement as a constructor expression.
      def element_src(element, indent)
        i = indent + "  "
        case element
        when RuleReference
          "GT::RuleReference.new(name: #{element.name.inspect}, is_token: #{element.is_token.inspect})"
        when Literal
          "GT::Literal.new(value: #{element.value.inspect})"
        when Sequence
          items = element.elements.map { |e| "#{i}#{element_src(e, i)}" }.join(",\n")
          "GT::Sequence.new(elements: [\n#{items},\n#{indent}])"
        when Alternation
          items = element.choices.map { |c| "#{i}#{element_src(c, i)}" }.join(",\n")
          "GT::Alternation.new(choices: [\n#{items},\n#{indent}])"
        when Repetition
          child = element_src(element.element, i)
          "GT::Repetition.new(element: #{child})"
        when OptionalElement
          child = element_src(element.element, i)
          "GT::OptionalElement.new(element: #{child})"
        when Group
          child = element_src(element.element, i)
          "GT::Group.new(element: #{child})"
        when PositiveLookahead
          child = element_src(element.element, i)
          "GT::PositiveLookahead.new(element: #{child})"
        when NegativeLookahead
          child = element_src(element.element, i)
          "GT::NegativeLookahead.new(element: #{child})"
        when OneOrMoreRepetition
          child = element_src(element.element, i)
          "GT::OneOrMoreRepetition.new(element: #{child})"
        when SeparatedRepetition
          elem_child = element_src(element.element, i)
          sep_child = element_src(element.separator, i)
          "GT::SeparatedRepetition.new(element: #{elem_child}, separator: #{sep_child}, at_least_one: #{element.at_least_one.inspect})"
        else
          raise TypeError, "Unknown grammar element: #{element.class}"
        end
      end
    end

    # Expose compiler functions directly on the GrammarTools module for
    # convenience:
    #
    #   CodingAdventures::GrammarTools.compile_token_grammar(grammar)
    module_function

    def compile_token_grammar(grammar, source_file = "")
      Compiler.compile_token_grammar(grammar, source_file)
    end

    def compile_parser_grammar(grammar, source_file = "")
      Compiler.compile_parser_grammar(grammar, source_file)
    end
  end
end
