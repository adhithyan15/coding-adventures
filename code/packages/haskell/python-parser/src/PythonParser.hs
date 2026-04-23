module PythonParser
    ( description
    , PythonParserError(..)
    , parsePythonTokens
    , tokenizeAndParsePython
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified PythonLexer

description :: String
description = "Haskell python-parser backed by compiled parser grammar data"

data PythonParserError
    = PythonParserLexerError LexerError
    | PythonParserParseError ParseError
    deriving (Eq, Show)

parsePythonTokens :: [Token] -> Either ParseError ASTNode
parsePythonTokens = parseWithGrammar parserGrammarData

tokenizeAndParsePython :: String -> Either PythonParserError ASTNode
tokenizeAndParsePython source =
    case PythonLexer.tokenizePython source of
        Left err -> Left (PythonParserLexerError err)
        Right tokens ->
            case parsePythonTokens tokens of
                Left err -> Left (PythonParserParseError err)
                Right ast -> Right ast
