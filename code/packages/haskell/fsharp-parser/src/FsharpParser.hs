module FsharpParser
    ( description
    , FsharpParserError(..)
    , parseFsharpTokens
    , tokenizeAndParseFsharp
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified FsharpLexer

description :: String
description = "Haskell fsharp-parser backed by compiled parser grammar data"

data FsharpParserError
    = FsharpParserLexerError LexerError
    | FsharpParserParseError ParseError
    deriving (Eq, Show)

parseFsharpTokens :: [Token] -> Either ParseError ASTNode
parseFsharpTokens = parseWithGrammar parserGrammarData

tokenizeAndParseFsharp :: String -> Either FsharpParserError ASTNode
tokenizeAndParseFsharp source =
    case FsharpLexer.tokenizeFsharp source of
        Left err -> Left (FsharpParserLexerError err)
        Right tokens ->
            case parseFsharpTokens tokens of
                Left err -> Left (FsharpParserParseError err)
                Right ast -> Right ast
