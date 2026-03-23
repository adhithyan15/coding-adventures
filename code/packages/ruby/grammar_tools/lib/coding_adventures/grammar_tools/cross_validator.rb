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
    #
    # Special handling for extended features:
    # - Indentation mode: INDENT, DEDENT, NEWLINE are implicitly available.
    # - Aliases: If STRING_DQ -> STRING, referencing STRING counts as used.
    # - EOF is always implicitly available.
    def self.cross_validate(token_grammar, parser_grammar)
      issues = []

      # Build the set of all token names the parser can reference.
      defined_tokens = token_grammar.token_names

      # Indentation mode synthesizes INDENT, DEDENT, and NEWLINE tokens.
      if token_grammar.mode == "indentation"
        defined_tokens.merge(%w[INDENT DEDENT NEWLINE])
      end

      # The NEWLINE token is also implicitly available whenever the skip
      # pattern does NOT consume newlines. In that case, the lexer emits
      # NEWLINE tokens at each bare '\n'. Rather than requiring grammars to
      # redundantly define NEWLINE in their .tokens file, we always treat it
      # as a valid synthetic token (like EOF).
      defined_tokens.add("NEWLINE")

      # EOF is always implicitly available.
      defined_tokens.add("EOF")

      referenced_tokens = parser_grammar.token_references

      # Missing token references (errors).
      referenced_tokens.sort.each do |ref|
        unless defined_tokens.include?(ref)
          issues << "Error: Grammar references token '#{ref}' which is not " \
                    "defined in the tokens file"
        end
      end

      # Unused tokens (warnings).
      # Build alias -> names mapping for "used via alias" detection.
      alias_to_names = {}
      token_grammar.definitions.each do |defn|
        if defn.alias_name
          (alias_to_names[defn.alias_name] ||= []) << defn.name
        end
      end

      token_grammar.definitions.each do |defn|
        # A definition is "used" if its name OR its alias is referenced.
        is_used = referenced_tokens.include?(defn.name)
        is_used = true if defn.alias_name && referenced_tokens.include?(defn.alias_name)

        unless is_used
          issues << "Warning: Token '#{defn.name}' (line #{defn.line_number}) " \
                    "is defined but never used in the grammar"
        end
      end

      issues
    end
  end
end
