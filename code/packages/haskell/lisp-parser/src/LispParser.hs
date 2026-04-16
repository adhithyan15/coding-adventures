module LispParser
    ( description
    , LispParserError(..)
    , parseLispTokens
    , tokenizeAndParseLisp
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified LispLexer

description :: String
description = "Haskell lisp-parser backed by compiled parser grammar data"

data LispParserError
    = LispParserLexerError LexerError
    | LispParserParseError ParseError
    deriving (Eq, Show)

parseLispTokens :: [Token] -> Either ParseError ASTNode
parseLispTokens = parseWithGrammar parserGrammarData

tokenizeAndParseLisp :: String -> Either LispParserError ASTNode
tokenizeAndParseLisp source =
    case LispLexer.tokenizeLisp source of
        Left err -> Left (LispParserLexerError err)
        Right tokens ->
            case parseLispTokens tokens of
                Left err -> Left (LispParserParseError err)
                Right ast -> Right ast
