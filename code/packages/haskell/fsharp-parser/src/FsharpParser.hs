module FsharpParser
    ( description
    , FsharpParserError(..)
    , parseFsharpTokens
    , tokenizeAndParseFsharp
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified FsharpLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for fsharp-parser built on the generic parser package"

data FsharpParserError
    = FsharpParserLexerError LexerError
    | FsharpParserParseError ParseError
    deriving (Eq, Show)

parseFsharpTokens :: [Token] -> Either ParseError ASTNode
parseFsharpTokens = parseTokens

tokenizeAndParseFsharp :: String -> Either FsharpParserError ASTNode
tokenizeAndParseFsharp source =
    case FsharpLexer.tokenizeFsharp source of
        Left err -> Left (FsharpParserLexerError err)
        Right tokens ->
            case parseFsharpTokens tokens of
                Left err -> Left (FsharpParserParseError err)
                Right ast -> Right ast
