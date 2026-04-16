module SqlParser
    ( description
    , SqlParserError(..)
    , parseSqlTokens
    , tokenizeAndParseSql
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified SqlLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for sql-parser built on the generic parser package"

data SqlParserError
    = SqlParserLexerError LexerError
    | SqlParserParseError ParseError
    deriving (Eq, Show)

parseSqlTokens :: [Token] -> Either ParseError ASTNode
parseSqlTokens = parseTokens

tokenizeAndParseSql :: String -> Either SqlParserError ASTNode
tokenizeAndParseSql source =
    case SqlLexer.tokenizeSql source of
        Left err -> Left (SqlParserLexerError err)
        Right tokens ->
            case parseSqlTokens tokens of
                Left err -> Left (SqlParserParseError err)
                Right ast -> Right ast
