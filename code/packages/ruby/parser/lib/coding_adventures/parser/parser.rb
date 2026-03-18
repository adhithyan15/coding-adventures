# frozen_string_literal: true

require "coding_adventures_lexer"

# ==========================================================================
# Parser -- Hand-Written Recursive Descent Parser
# ==========================================================================
#
# Each grammar rule becomes a method:
#
#   program         = statement*
#   statement       = assignment | expression_stmt
#   assignment      = NAME EQUALS expression NEWLINE
#   expression_stmt = expression NEWLINE
#   expression      = term ((PLUS | MINUS) term)*
#   term            = factor ((STAR | SLASH) factor)*
#   factor          = NUMBER | STRING | NAME | LPAREN expression RPAREN
#
# Operator precedence is encoded by the depth of the rules:
#   expression  -> handles + and -  (LOWEST precedence)
#   term        -> handles * and /  (HIGHER precedence)
#   factor      -> handles atoms    (HIGHEST precedence)
# ==========================================================================

module CodingAdventures
  module Parser
    class RecursiveDescentParser
      TT = CodingAdventures::Lexer::TokenType

      def initialize(tokens)
        @tokens = tokens
        @pos = 0
      end

      # Parse the token stream and return a complete AST.
      def parse
        parse_program
      end

      private

      # =====================================================================
      # Helper methods
      # =====================================================================

      def peek
        return @tokens[-1] if @pos >= @tokens.length
        @tokens[@pos]
      end

      def advance
        token = peek
        @pos += 1
        token
      end

      def expect(token_type)
        token = peek
        unless token.type == token_type
          raise ParseError.new(
            "Expected #{token_type}, got #{token.type} (#{token.value.inspect})",
            token
          )
        end
        advance
      end

      # Consume the current token if it matches any of the given types.
      def match(*token_types)
        token = peek
        if token_types.include?(token.type)
          advance
        end
      end

      def at_end?
        peek.type == TT::EOF
      end

      def skip_newlines
        advance while peek.type == TT::NEWLINE
      end

      # =====================================================================
      # Grammar methods
      # =====================================================================

      # program = statement*
      def parse_program
        statements = []
        skip_newlines
        until at_end?
          statements << parse_statement
          skip_newlines
        end
        Program.new(statements: statements)
      end

      # statement = assignment | expression_stmt
      def parse_statement
        if peek.type == TT::NAME &&
           @pos + 1 < @tokens.length &&
           @tokens[@pos + 1].type == TT::EQUALS
          parse_assignment
        else
          parse_expression_stmt
        end
      end

      # assignment = NAME EQUALS expression NEWLINE
      def parse_assignment
        name_token = expect(TT::NAME)
        target = Name.new(name: name_token.value)
        expect(TT::EQUALS)
        value = parse_expression
        expect(TT::NEWLINE) unless at_end?
        Assignment.new(target: target, value: value)
      end

      # expression_stmt = expression NEWLINE
      def parse_expression_stmt
        expression = parse_expression
        expect(TT::NEWLINE) unless at_end?
        expression
      end

      # expression = term ((PLUS | MINUS) term)*
      def parse_expression
        left = parse_term
        while (op_token = match(TT::PLUS, TT::MINUS))
          right = parse_term
          left = BinaryOp.new(left: left, op: op_token.value, right: right)
        end
        left
      end

      # term = factor ((STAR | SLASH) factor)*
      def parse_term
        left = parse_factor
        while (op_token = match(TT::STAR, TT::SLASH))
          right = parse_factor
          left = BinaryOp.new(left: left, op: op_token.value, right: right)
        end
        left
      end

      # factor = NUMBER | STRING | NAME | LPAREN expression RPAREN
      def parse_factor
        token = peek

        case token.type
        when TT::NUMBER
          advance
          NumberLiteral.new(value: token.value.to_i)
        when TT::STRING
          advance
          StringLiteral.new(value: token.value)
        when TT::NAME
          advance
          Name.new(name: token.value)
        when TT::LPAREN
          advance
          expression = parse_expression
          expect(TT::RPAREN)
          expression
        else
          raise ParseError.new(
            "Unexpected token #{token.type} (#{token.value.inspect})",
            token
          )
        end
      end
    end
  end
end
