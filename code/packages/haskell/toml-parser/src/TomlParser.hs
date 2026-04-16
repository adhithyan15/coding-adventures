module TomlParser
    ( description
    , TomlParserError(..)
    , parseTomlTokens
    , tokenizeAndParseToml
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified TomlLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for toml-parser built on the generic parser package"

data TomlParserError
    = TomlParserLexerError LexerError
    | TomlParserParseError ParseError
    deriving (Eq, Show)

parseTomlTokens :: [Token] -> Either ParseError ASTNode
parseTomlTokens = parseTokens

tokenizeAndParseToml :: String -> Either TomlParserError ASTNode
tokenizeAndParseToml source =
    case TomlLexer.tokenizeToml source of
        Left err -> Left (TomlParserLexerError err)
        Right tokens ->
            case parseTomlTokens tokens of
                Left err -> Left (TomlParserParseError err)
                Right ast -> Right ast
