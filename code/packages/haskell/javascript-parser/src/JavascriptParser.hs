module JavascriptParser
    ( description
    , JavascriptParserError(..)
    , parseJavascriptTokens
    , tokenizeAndParseJavascript
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified JavascriptLexer

description :: String
description = "Haskell javascript-parser backed by compiled parser grammar data"

data JavascriptParserError
    = JavascriptParserLexerError LexerError
    | JavascriptParserParseError ParseError
    deriving (Eq, Show)

parseJavascriptTokens :: [Token] -> Either ParseError ASTNode
parseJavascriptTokens = parseWithGrammar parserGrammarData

tokenizeAndParseJavascript :: String -> Either JavascriptParserError ASTNode
tokenizeAndParseJavascript source =
    case JavascriptLexer.tokenizeJavascript source of
        Left err -> Left (JavascriptParserLexerError err)
        Right tokens ->
            case parseJavascriptTokens tokens of
                Left err -> Left (JavascriptParserParseError err)
                Right ast -> Right ast
