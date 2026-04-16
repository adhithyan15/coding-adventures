module LispParser
    ( description
    , LispParserError(..)
    , parseLispTokens
    , tokenizeAndParseLisp
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified LispLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for lisp-parser built on the generic parser package"

data LispParserError
    = LispParserLexerError LexerError
    | LispParserParseError ParseError
    deriving (Eq, Show)

parseLispTokens :: [Token] -> Either ParseError ASTNode
parseLispTokens = parseTokens

tokenizeAndParseLisp :: String -> Either LispParserError ASTNode
tokenizeAndParseLisp source =
    case LispLexer.tokenizeLisp source of
        Left err -> Left (LispParserLexerError err)
        Right tokens ->
            case parseLispTokens tokens of
                Left err -> Left (LispParserParseError err)
                Right ast -> Right ast
