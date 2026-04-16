module PythonParser
    ( description
    , PythonParserError(..)
    , parsePythonTokens
    , tokenizeAndParsePython
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified PythonLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for python-parser built on the generic parser package"

data PythonParserError
    = PythonParserLexerError LexerError
    | PythonParserParseError ParseError
    deriving (Eq, Show)

parsePythonTokens :: [Token] -> Either ParseError ASTNode
parsePythonTokens = parseTokens

tokenizeAndParsePython :: String -> Either PythonParserError ASTNode
tokenizeAndParsePython source =
    case PythonLexer.tokenizePython source of
        Left err -> Left (PythonParserLexerError err)
        Right tokens ->
            case parsePythonTokens tokens of
                Left err -> Left (PythonParserParseError err)
                Right ast -> Right ast
