module NibParser
    ( description
    , NibParserError(..)
    , parseNibTokens
    , tokenizeAndParseNib
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified NibLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for nib-parser built on the generic parser package"

data NibParserError
    = NibParserLexerError LexerError
    | NibParserParseError ParseError
    deriving (Eq, Show)

parseNibTokens :: [Token] -> Either ParseError ASTNode
parseNibTokens = parseTokens

tokenizeAndParseNib :: String -> Either NibParserError ASTNode
tokenizeAndParseNib source =
    case NibLexer.tokenizeNib source of
        Left err -> Left (NibParserLexerError err)
        Right tokens ->
            case parseNibTokens tokens of
                Left err -> Left (NibParserParseError err)
                Right ast -> Right ast
