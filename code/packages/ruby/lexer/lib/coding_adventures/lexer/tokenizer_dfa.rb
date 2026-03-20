# frozen_string_literal: true

# ==========================================================================
# Tokenizer DFA -- Formal Model of the Hand-Written Lexer's Dispatch Logic
# ==========================================================================
#
# The hand-written tokenizer in tokenizer.rb has an *implicit* DFA in its
# main loop: it looks at the current character, classifies it, and dispatches
# to the appropriate sub-routine. This module makes that implicit DFA
# *explicit* by defining it as a formal DFA using the state-machine library.
#
# === States ===
#
# The tokenizer DFA has 10 states. Each state represents what the tokenizer
# is about to do:
#
#   State            Description
#   -----------      ---------------------------------------------------
#   start            Idle -- examining the next character to decide what
#                    kind of token starts here.
#   in_number        Reading a sequence of digits (e.g., "42").
#   in_name          Reading an identifier -- letters, digits, underscores
#                    (e.g., "my_var").
#   in_string        Reading a string literal between double quotes.
#   in_operator      Emitting a single-character operator or delimiter
#                    (e.g., "+", "(", ".").
#   in_equals        Handling "=" with lookahead for "==".
#   at_newline       Emitting a NEWLINE token.
#   at_whitespace    Skipping whitespace (spaces, tabs, carriage returns).
#   done             End of input reached -- the lexer is finished.
#   error            An unexpected character was encountered.
#
# === How the DFA is used ===
#
# The DFA does NOT replace the tokenizer's logic. The sub-routines like
# read_number and read_string still do the actual work. What the DFA
# provides is a formal, verifiable model of the dispatch decision:
# "given that I'm in the start state and I see a digit, I should go to
# in_number." The tokenizer consults the DFA for this decision, then calls
# the appropriate sub-routine.
#
# This means the tokenizer's behavior is now *data-driven* at the top level.
# If you want to verify that the tokenizer handles every character class,
# you can inspect the DFA's transition table. If you want to visualize the
# dispatch logic, you can call dfa.to_dot and render it as a graph.
#
# === Character classification ===
#
# The DFA operates on *character classes*, not raw characters. The
# classify_char method maps every possible character to a named class.
# This is the alphabet of the DFA.
#
# Character class table:
#
#   Class             Characters       What it triggers
#   ===============   ===========      =====================================
#   "eof"             nil (end)        Emit EOF token
#   "whitespace"      space/tab/CR     Skip whitespace
#   "newline"         \n               Emit NEWLINE token
#   "digit"           0-9              Read a number
#   "alpha"           a-zA-Z           Read a name/keyword
#   "underscore"      _                Read a name/keyword (starts identifier)
#   "quote"           "                Read a string literal
#   "equals"          =                Lookahead for = vs ==
#   "operator"        +-*/             Emit simple operator token
#   "open_paren"      (                Emit LPAREN
#   "close_paren"     )                Emit RPAREN
#   "comma"           ,                Emit COMMA
#   "colon"           :                Emit COLON
#   "semicolon"       ;                Emit SEMICOLON
#   "open_brace"      {                Emit LBRACE
#   "close_brace"     }                Emit RBRACE
#   "open_bracket"    [                Emit LBRACKET
#   "close_bracket"   ]                Emit RBRACKET
#   "dot"             .                Emit DOT
#   "bang"            !                Emit BANG
#   "other"           everything else  Raise error
# ==========================================================================

require "coding_adventures_state_machine"

module CodingAdventures
  module Lexer
    module TokenizerDFA
      # Classify a character into one of the DFA's alphabet symbols.
      #
      # The tokenizer's main loop dispatches on the *kind* of character it
      # sees, not on the exact character. For example, "a", "Z", and "_"
      # all belong to similar classes because they can all appear in an
      # identifier.
      #
      # This method makes that implicit classification explicit by mapping
      # every possible character to a named class. The DFA's transition table
      # uses these class names to decide what to do next.
      #
      # @param ch [String, nil] A single character, or nil if at end of input.
      # @return [String] The name of the character class.
      def self.classify_char(ch)
        return "eof" if ch.nil?
        return "whitespace" if ch == " " || ch == "\t" || ch == "\r"
        return "newline" if ch == "\n"
        return "digit" if ch.match?(/[0-9]/)
        return "alpha" if ch.match?(/[a-zA-Z]/)
        return "underscore" if ch == "_"
        return "quote" if ch == '"'
        return "equals" if ch == "="
        return "operator" if ch == "+" || ch == "-" || ch == "*" || ch == "/"
        return "open_paren" if ch == "("
        return "close_paren" if ch == ")"
        return "comma" if ch == ","
        return "colon" if ch == ":"
        return "semicolon" if ch == ";"
        return "open_brace" if ch == "{"
        return "close_brace" if ch == "}"
        return "open_bracket" if ch == "["
        return "close_bracket" if ch == "]"
        return "dot" if ch == "."
        return "bang" if ch == "!"

        "other"
      end

      # The set of DFA states.
      STATES = Set[
        "start", "in_number", "in_name", "in_string",
        "in_operator", "in_equals", "at_newline", "at_whitespace",
        "done", "error"
      ].freeze

      # The alphabet -- character classes that the DFA recognizes.
      ALPHABET = Set[
        "digit", "alpha", "underscore", "quote", "newline", "whitespace",
        "operator", "equals", "open_paren", "close_paren", "comma", "colon",
        "semicolon", "open_brace", "close_brace", "open_bracket",
        "close_bracket", "dot", "bang", "eof", "other"
      ].freeze

      # Map from character class to target state when in "start".
      #
      # This is the heart of the dispatch logic. When the tokenizer is in the
      # "start" state and sees a character of a given class, it transitions to
      # the corresponding handler state.
      START_DISPATCH = {
        "digit" => "in_number",
        "alpha" => "in_name",
        "underscore" => "in_name",
        "quote" => "in_string",
        "newline" => "at_newline",
        "whitespace" => "at_whitespace",
        "operator" => "in_operator",
        "equals" => "in_equals",
        "open_paren" => "in_operator",
        "close_paren" => "in_operator",
        "comma" => "in_operator",
        "colon" => "in_operator",
        "semicolon" => "in_operator",
        "open_brace" => "in_operator",
        "close_brace" => "in_operator",
        "open_bracket" => "in_operator",
        "close_bracket" => "in_operator",
        "dot" => "in_operator",
        "bang" => "in_operator",
        "eof" => "done",
        "other" => "error"
      }.freeze

      # Build the complete transition table for the tokenizer DFA.
      #
      # The transition table has three sections:
      #
      # 1. From "start", each character class goes to its handler state
      #    (defined by START_DISPATCH above).
      #
      # 2. All handler states (in_number, in_name, etc.) transition back
      #    to "start" on every symbol. This models the fact that after
      #    emitting a token, the lexer returns to the start state to
      #    examine the next character.
      #
      # 3. The terminal states "done" and "error" loop on themselves for
      #    every symbol. Once the lexer finishes or fails, it stays there.
      #
      # @return [Hash<Array(String, String), String>] The transition table.
      def self.build_transitions
        transitions = {}

        # From "start", dispatch based on character class.
        START_DISPATCH.each do |char_class, target|
          transitions[["start", char_class]] = target
        end

        # All handler states return to "start" on every symbol.
        handlers = %w[
          in_number in_name in_string in_operator
          in_equals at_newline at_whitespace
        ]
        handlers.each do |handler|
          ALPHABET.each do |symbol|
            transitions[[handler, symbol]] = "start"
          end
        end

        # "done" loops on itself for every symbol.
        ALPHABET.each do |symbol|
          transitions[["done", symbol]] = "done"
        end

        # "error" loops on itself for every symbol.
        ALPHABET.each do |symbol|
          transitions[["error", symbol]] = "error"
        end

        transitions.freeze
      end

      # Create a new tokenizer dispatch DFA.
      #
      # Each call returns a fresh DFA instance so callers can process
      # independently. The DFA models the top-level character classification
      # dispatch of the hand-written tokenizer.
      #
      # @return [CodingAdventures::StateMachine::DFA] A new DFA instance.
      #
      # @example
      #   dfa = TokenizerDFA.new_tokenizer_dfa
      #   char_class = TokenizerDFA.classify_char("5")  # => "digit"
      #   next_state = dfa.process(char_class)           # => "in_number"
      def self.new_tokenizer_dfa
        CodingAdventures::StateMachine::DFA.new(
          states: STATES.dup,
          alphabet: ALPHABET.dup,
          transitions: build_transitions,
          initial: "start",
          accepting: Set["done"]
        )
      end
    end
  end
end
