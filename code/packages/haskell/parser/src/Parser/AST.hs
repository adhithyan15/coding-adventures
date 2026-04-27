module Parser.AST
    ( ASTNode(..)
    ) where

import Lexer.Token

data ASTNode
    = NumberNode Double
    | StringNode String
    | NameNode String
    | BinaryOpNode ASTNode String ASTNode
    | AssignmentNode String ASTNode
    | ExpressionStmtNode ASTNode
    | ProgramNode [ASTNode]
    | RuleNode String [ASTNode]
    | TokenNode Token
    deriving (Eq, Show)
