module TypescriptParser
    ( description
    , TypescriptParserError(..)
    , parseTypescriptTokens
    , tokenizeAndParseTypescript
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified TypescriptLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for typescript-parser built on the generic parser package"

data TypescriptParserError
    = TypescriptParserLexerError LexerError
    | TypescriptParserParseError ParseError
    deriving (Eq, Show)

parseTypescriptTokens :: [Token] -> Either ParseError ASTNode
parseTypescriptTokens = parseTokens

tokenizeAndParseTypescript :: String -> Either TypescriptParserError ASTNode
tokenizeAndParseTypescript source =
    case TypescriptLexer.tokenizeTypescript source of
        Left err -> Left (TypescriptParserLexerError err)
        Right tokens ->
            case parseTypescriptTokens tokens of
                Left err -> Left (TypescriptParserParseError err)
                Right ast -> Right ast
