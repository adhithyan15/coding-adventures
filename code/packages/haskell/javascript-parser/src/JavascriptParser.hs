module JavascriptParser
    ( description
    , JavascriptParserError(..)
    , parseJavascriptTokens
    , tokenizeAndParseJavascript
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified JavascriptLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for javascript-parser built on the generic parser package"

data JavascriptParserError
    = JavascriptParserLexerError LexerError
    | JavascriptParserParseError ParseError
    deriving (Eq, Show)

parseJavascriptTokens :: [Token] -> Either ParseError ASTNode
parseJavascriptTokens = parseTokens

tokenizeAndParseJavascript :: String -> Either JavascriptParserError ASTNode
tokenizeAndParseJavascript source =
    case JavascriptLexer.tokenizeJavascript source of
        Left err -> Left (JavascriptParserLexerError err)
        Right tokens ->
            case parseJavascriptTokens tokens of
                Left err -> Left (JavascriptParserParseError err)
                Right ast -> Right ast
