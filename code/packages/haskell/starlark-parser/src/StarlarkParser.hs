module StarlarkParser
    ( description
    , StarlarkParserError(..)
    , parseStarlarkTokens
    , tokenizeAndParseStarlark
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified StarlarkLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for starlark-parser built on the generic parser package"

data StarlarkParserError
    = StarlarkParserLexerError LexerError
    | StarlarkParserParseError ParseError
    deriving (Eq, Show)

parseStarlarkTokens :: [Token] -> Either ParseError ASTNode
parseStarlarkTokens = parseTokens

tokenizeAndParseStarlark :: String -> Either StarlarkParserError ASTNode
tokenizeAndParseStarlark source =
    case StarlarkLexer.tokenizeStarlark source of
        Left err -> Left (StarlarkParserLexerError err)
        Right tokens ->
            case parseStarlarkTokens tokens of
                Left err -> Left (StarlarkParserParseError err)
                Right ast -> Right ast
