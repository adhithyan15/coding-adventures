# frozen_string_literal: true

# ==========================================================================
# AST Node Classes
# ==========================================================================
#
# These are the building blocks of our Abstract Syntax Tree. Each class
# represents a different kind of syntactic construct. Think of them like
# LEGO bricks -- each type has a specific shape (its fields), and you snap
# them together to build the full tree.
#
# The tree structure naturally encodes operator precedence:
#
#   "1 + 2 * 3" becomes:
#
#     BinaryOp(
#       left: NumberLiteral(1),
#       op: "+",
#       right: BinaryOp(
#         left: NumberLiteral(2),
#         op: "*",
#         right: NumberLiteral(3)
#       )
#     )
#
# The multiplication is deeper, so it gets evaluated first.
# ==========================================================================

module CodingAdventures
  module Parser
    # A numeric literal like 42 or 7. A leaf node.
    NumberLiteral = Data.define(:value)

    # A string literal like "hello". A leaf node.
    StringLiteral = Data.define(:value)

    # A variable name (identifier) like x or total.
    Name = Data.define(:name)

    # A binary operation like 1 + 2 or x * y.
    # left and right are expressions; op is a string like "+", "-", "*", "/".
    BinaryOp = Data.define(:left, :op, :right)

    # A variable assignment like x = 1 + 2.
    # target is a Name; value is an expression.
    Assignment = Data.define(:target, :value)

    # The root node -- an entire program (sequence of statements).
    Program = Data.define(:statements)

    # An error raised when the parser encounters unexpected tokens.
    class ParseError < StandardError
      attr_reader :message, :token

      def initialize(message, token)
        @message = message
        @token = token
        super("#{message} at line #{token.line}, column #{token.column}")
      end
    end
  end
end
