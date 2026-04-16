module CssParser
    ( description
    , CssParserError(..)
    , parseCssTokens
    , tokenizeAndParseCss
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified CssLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for css-parser built on the generic parser package"

data CssParserError
    = CssParserLexerError LexerError
    | CssParserParseError ParseError
    deriving (Eq, Show)

parseCssTokens :: [Token] -> Either ParseError ASTNode
parseCssTokens = parseTokens

tokenizeAndParseCss :: String -> Either CssParserError ASTNode
tokenizeAndParseCss source =
    case CssLexer.tokenizeCss source of
        Left err -> Left (CssParserLexerError err)
        Right tokens ->
            case parseCssTokens tokens of
                Left err -> Left (CssParserParseError err)
                Right ast -> Right ast
