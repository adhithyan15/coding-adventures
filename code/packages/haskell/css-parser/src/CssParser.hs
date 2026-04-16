module CssParser
    ( description
    , CssParserError(..)
    , parseCssTokens
    , tokenizeAndParseCss
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified CssLexer

description :: String
description = "Haskell css-parser backed by compiled parser grammar data"

data CssParserError
    = CssParserLexerError LexerError
    | CssParserParseError ParseError
    deriving (Eq, Show)

parseCssTokens :: [Token] -> Either ParseError ASTNode
parseCssTokens = parseWithGrammar parserGrammarData

tokenizeAndParseCss :: String -> Either CssParserError ASTNode
tokenizeAndParseCss source =
    case CssLexer.tokenizeCss source of
        Left err -> Left (CssParserLexerError err)
        Right tokens ->
            case parseCssTokens tokens of
                Left err -> Left (CssParserParseError err)
                Right ast -> Right ast
