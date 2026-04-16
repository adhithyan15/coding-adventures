module Parser.AST
    ( ASTNode(..)
    ) where

data ASTNode
    = NumberNode Double
    | StringNode String
    | NameNode String
    | BinaryOpNode ASTNode String ASTNode
    | AssignmentNode String ASTNode
    | ExpressionStmtNode ASTNode
    | ProgramNode [ASTNode]
    deriving (Eq, Show)
