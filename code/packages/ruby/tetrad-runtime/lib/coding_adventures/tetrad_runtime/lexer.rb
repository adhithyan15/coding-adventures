# frozen_string_literal: true

module CodingAdventures
  module TetradRuntime
    Token = Data.define(:type, :value)

    module Lexer
      KEYWORDS = %w[fn let return].freeze

      def self.tokenize(source)
        tokens = []
        i = 0
        while i < source.length
          char = source[i]
          if char =~ /\s/
            i += 1
          elsif char == "#"
            i += 1
            i += 1 while i < source.length && source[i] != "\n"
          elsif source[i, 2] == ":="
            tokens << Token.new(:op, ":=")
            i += 2
          elsif "+-*/%(),{}=;".include?(char)
            tokens << Token.new(char.to_sym, char)
            i += 1
          elsif char =~ /\d/
            start = i
            i += 1 while i < source.length && source[i] =~ /\d/
            tokens << Token.new(:number, source[start...i])
          elsif char =~ /[A-Za-z_]/
            start = i
            i += 1 while i < source.length && source[i] =~ /[A-Za-z0-9_]/
            word = source[start...i]
            tokens << Token.new(KEYWORDS.include?(word) ? word.to_sym : :name, word)
          else
            raise SyntaxError, "unexpected character #{char.inspect}"
          end
        end
        tokens << Token.new(:eof, "")
      end
    end
  end
end
