module JsonParser
    ( description
    , JsonParserError(..)
    , parseJsonTokens
    , tokenizeAndParseJson
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified JsonLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for json-parser built on the generic parser package"

data JsonParserError
    = JsonParserLexerError LexerError
    | JsonParserParseError ParseError
    deriving (Eq, Show)

parseJsonTokens :: [Token] -> Either ParseError ASTNode
parseJsonTokens = parseTokens

tokenizeAndParseJson :: String -> Either JsonParserError ASTNode
tokenizeAndParseJson source =
    case JsonLexer.tokenizeJson source of
        Left err -> Left (JsonParserLexerError err)
        Right tokens ->
            case parseJsonTokens tokens of
                Left err -> Left (JsonParserParseError err)
                Right ast -> Right ast
