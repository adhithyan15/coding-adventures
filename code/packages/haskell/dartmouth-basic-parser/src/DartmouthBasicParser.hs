module DartmouthBasicParser
    ( description
    , DartmouthBasicParserError(..)
    , parseDartmouthBasicTokens
    , tokenizeAndParseDartmouthBasic
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified DartmouthBasicLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for dartmouth-basic-parser built on the generic parser package"

data DartmouthBasicParserError
    = DartmouthBasicParserLexerError LexerError
    | DartmouthBasicParserParseError ParseError
    deriving (Eq, Show)

parseDartmouthBasicTokens :: [Token] -> Either ParseError ASTNode
parseDartmouthBasicTokens = parseTokens

tokenizeAndParseDartmouthBasic :: String -> Either DartmouthBasicParserError ASTNode
tokenizeAndParseDartmouthBasic source =
    case DartmouthBasicLexer.tokenizeDartmouthBasic source of
        Left err -> Left (DartmouthBasicParserLexerError err)
        Right tokens ->
            case parseDartmouthBasicTokens tokens of
                Left err -> Left (DartmouthBasicParserParseError err)
                Right ast -> Right ast
