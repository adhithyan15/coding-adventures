module CsharpParser
    ( description
    , CsharpParserError(..)
    , parseCsharpTokens
    , tokenizeAndParseCsharp
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified CsharpLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for csharp-parser built on the generic parser package"

data CsharpParserError
    = CsharpParserLexerError LexerError
    | CsharpParserParseError ParseError
    deriving (Eq, Show)

parseCsharpTokens :: [Token] -> Either ParseError ASTNode
parseCsharpTokens = parseTokens

tokenizeAndParseCsharp :: String -> Either CsharpParserError ASTNode
tokenizeAndParseCsharp source =
    case CsharpLexer.tokenizeCsharp source of
        Left err -> Left (CsharpParserLexerError err)
        Right tokens ->
            case parseCsharpTokens tokens of
                Left err -> Left (CsharpParserParseError err)
                Right ast -> Right ast
