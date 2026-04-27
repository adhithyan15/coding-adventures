module SqlParser
    ( description
    , SqlParserError(..)
    , parseSqlTokens
    , tokenizeAndParseSql
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified SqlLexer

description :: String
description = "Haskell sql-parser backed by compiled parser grammar data"

data SqlParserError
    = SqlParserLexerError LexerError
    | SqlParserParseError ParseError
    deriving (Eq, Show)

parseSqlTokens :: [Token] -> Either ParseError ASTNode
parseSqlTokens = parseWithGrammar parserGrammarData

tokenizeAndParseSql :: String -> Either SqlParserError ASTNode
tokenizeAndParseSql source =
    case SqlLexer.tokenizeSql source of
        Left err -> Left (SqlParserLexerError err)
        Right tokens ->
            case parseSqlTokens tokens of
                Left err -> Left (SqlParserParseError err)
                Right ast -> Right ast
