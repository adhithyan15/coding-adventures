module CsharpParser
    ( description
    , CsharpParserError(..)
    , parseCsharpTokens
    , tokenizeAndParseCsharp
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified CsharpLexer

description :: String
description = "Haskell csharp-parser backed by compiled parser grammar data"

data CsharpParserError
    = CsharpParserLexerError LexerError
    | CsharpParserParseError ParseError
    deriving (Eq, Show)

parseCsharpTokens :: [Token] -> Either ParseError ASTNode
parseCsharpTokens = parseWithGrammar parserGrammarData

tokenizeAndParseCsharp :: String -> Either CsharpParserError ASTNode
tokenizeAndParseCsharp source =
    case CsharpLexer.tokenizeCsharp source of
        Left err -> Left (CsharpParserLexerError err)
        Right tokens ->
            case parseCsharpTokens tokens of
                Left err -> Left (CsharpParserParseError err)
                Right ast -> Right ast
