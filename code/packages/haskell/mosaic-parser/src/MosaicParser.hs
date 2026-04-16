module MosaicParser
    ( description
    , MosaicParserError(..)
    , parseMosaicTokens
    , tokenizeAndParseMosaic
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified MosaicLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for mosaic-parser built on the generic parser package"

data MosaicParserError
    = MosaicParserLexerError LexerError
    | MosaicParserParseError ParseError
    deriving (Eq, Show)

parseMosaicTokens :: [Token] -> Either ParseError ASTNode
parseMosaicTokens = parseTokens

tokenizeAndParseMosaic :: String -> Either MosaicParserError ASTNode
tokenizeAndParseMosaic source =
    case MosaicLexer.tokenizeMosaic source of
        Left err -> Left (MosaicParserLexerError err)
        Right tokens ->
            case parseMosaicTokens tokens of
                Left err -> Left (MosaicParserParseError err)
                Right ast -> Right ast
