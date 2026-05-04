# frozen_string_literal: true

module CodingAdventures
  module Twig
    SymbolRef = Data.define(:name)

    class Parser
      def self.parse(source)
        new(tokenize(source)).parse_program
      end

      def self.tokenize(source)
        tokens = []
        i = 0
        while i < source.length
          char = source[i]
          if char =~ /\s/
            i += 1
          elsif char == ";"
            i += 1
            i += 1 while i < source.length && source[i] != "\n"
          elsif char == "(" || char == ")"
            tokens << char
            i += 1
          else
            start = i
            i += 1 while i < source.length && source[i] !~ /[\s()]/
            atom = source[start...i]
            tokens << atom
          end
        end
        tokens
      end

      def initialize(tokens)
        @tokens = tokens
        @pos = 0
      end

      def parse_program
        forms = []
        forms << parse_form until @pos >= @tokens.length
        forms
      end

      def parse_form
        tok = @tokens[@pos]
        @pos += 1
        case tok
        when "("
          list = []
          list << parse_form until @tokens[@pos] == ")"
          raise SyntaxError, "unclosed list" if @pos >= @tokens.length

          @pos += 1
          list
        when ")"
          raise SyntaxError, "unexpected ')'"
        when /^-?\d+$/
          tok.to_i
        when "#t"
          true
        when "#f"
          false
        when "nil"
          nil
        else
          SymbolRef.new(tok)
        end
      end
    end
  end
end
