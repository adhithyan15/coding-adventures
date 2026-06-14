# frozen_string_literal: true

require_relative "ast"
require_relative "lexer"

module CodingAdventures
  module TetradRuntime
    class Parser
      def self.parse(source)
        new(Lexer.tokenize(source)).parse
      end

      def initialize(tokens)
        @tokens = tokens
        @pos = 0
      end

      def parse
        forms = []
        forms << parse_form until peek.type == :eof
        Program.new(forms)
      end

      private

      def parse_form
        return parse_function if match?(:fn)
        stmt = parse_statement
        consume(:";") if peek.type == :";"
        stmt
      end

      def parse_function
        name = consume(:name).value
        consume(:"(")
        params = []
        unless peek.type == :")"
          loop do
            params << consume(:name).value
            break unless match?(:",")
          end
        end
        consume(:")")
        consume(:"{")
        body = []
        until peek.type == :"}"
          body << parse_statement
          consume(:";") if peek.type == :";"
        end
        consume(:"}")
        FunctionDef.new(name, params, body)
      end

      def parse_statement
        if match?(:let)
          name = consume(:name).value
          consume_assignment
          LetStmt.new(name, parse_expression)
        elsif match?(:return)
          ReturnStmt.new(parse_expression)
        elsif peek.type == :name && %i[= op].include?(@tokens[@pos + 1].type)
          name = consume(:name).value
          consume_assignment
          LetStmt.new(name, parse_expression)
        else
          ExprStmt.new(parse_expression)
        end
      end

      def parse_expression
        parse_add
      end

      def parse_add
        left = parse_mul
        while %i[+ -].include?(peek.type)
          op = advance.value
          left = Binary.new(left, op, parse_mul)
        end
        left
      end

      def parse_mul
        left = parse_primary
        while %i[* / %].include?(peek.type)
          op = advance.value
          left = Binary.new(left, op, parse_primary)
        end
        left
      end

      def parse_primary
        tok = advance
        case tok.type
        when :number
          NumberLit.new(tok.value.to_i)
        when :name
          if match?(:"(")
            args = []
            unless peek.type == :")"
              loop do
                args << parse_expression
                break unless match?(:",")
              end
            end
            consume(:")")
            Call.new(tok.value, args)
          else
            VarRef.new(tok.value)
          end
        when :"("
          expr = parse_expression
          consume(:")")
          expr
        else
          raise SyntaxError, "unexpected token #{tok.type.inspect}"
        end
      end

      def consume_assignment
        if peek.type == :op && peek.value == ":="
          advance
        else
          consume(:"=")
        end
      end

      def peek
        @tokens[@pos]
      end

      def advance
        tok = peek
        @pos += 1
        tok
      end

      def match?(type)
        return false unless peek.type == type

        advance
        true
      end

      def consume(type)
        tok = advance
        raise SyntaxError, "expected #{type.inspect}, got #{tok.type.inspect}" unless tok.type == type

        tok
      end
    end
  end
end
