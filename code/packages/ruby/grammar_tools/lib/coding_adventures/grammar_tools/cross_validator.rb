# frozen_string_literal: true

# ==========================================================================
# cross_validator.rb -- Cross-validates .tokens and .grammar Files
# ==========================================================================
#
# Each file can be valid on its own but broken when used together:
#
# - A grammar might reference SEMICOLON, but the .tokens file only defines
#   SEMI. Each file is fine individually, but the pair is broken.
# - A .tokens file might define TILDE = "~" that no grammar rule ever uses.
#   Not an error, but worth warning about.
#
# What we check
# -------------
#
# 1. Missing token references -- every UPPERCASE name in the grammar must
#    correspond to a token definition.
# 2. Unused tokens -- every token defined in .tokens should ideally be
#    referenced in the grammar. Reported as warnings, not errors.
# ==========================================================================

module CodingAdventures
  module GrammarTools
    # Cross-validate a token grammar and a parser grammar.
    #
    # Returns a list of error/warning strings. Errors describe broken
    # references; warnings describe unused definitions. An empty list means
    # the two grammars are fully consistent.
    def self.cross_validate(token_grammar, parser_grammar)
      issues = []

      defined_tokens = token_grammar.token_names
      referenced_tokens = parser_grammar.token_references

      # Missing token references (errors).
      referenced_tokens.sort.each do |ref|
        unless defined_tokens.include?(ref)
          issues << "Error: Grammar references token '#{ref}' which is not " \
                    "defined in the tokens file"
        end
      end

      # Unused tokens (warnings).
      token_grammar.definitions.each do |defn|
        unless referenced_tokens.include?(defn.name)
          issues << "Warning: Token '#{defn.name}' (line #{defn.line_number}) " \
                    "is defined but never used in the grammar"
        end
      end

      issues
    end
  end
end
