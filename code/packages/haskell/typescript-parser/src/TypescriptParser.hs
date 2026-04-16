module TypescriptParser
    ( description
    , TypescriptParserError(..)
    , parseTypescriptTokens
    , tokenizeAndParseTypescript
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified TypescriptLexer

description :: String
description = "Haskell typescript-parser backed by compiled parser grammar data"

data TypescriptParserError
    = TypescriptParserLexerError LexerError
    | TypescriptParserParseError ParseError
    deriving (Eq, Show)

parseTypescriptTokens :: [Token] -> Either ParseError ASTNode
parseTypescriptTokens = parseWithGrammar parserGrammarData

tokenizeAndParseTypescript :: String -> Either TypescriptParserError ASTNode
tokenizeAndParseTypescript source =
    case TypescriptLexer.tokenizeTypescript source of
        Left err -> Left (TypescriptParserLexerError err)
        Right tokens ->
            case parseTypescriptTokens tokens of
                Left err -> Left (TypescriptParserParseError err)
                Right ast -> Right ast
