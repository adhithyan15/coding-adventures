module MosaicParser
    ( description
    , MosaicParserError(..)
    , parseMosaicTokens
    , tokenizeAndParseMosaic
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified MosaicLexer

description :: String
description = "Haskell mosaic-parser backed by compiled parser grammar data"

data MosaicParserError
    = MosaicParserLexerError LexerError
    | MosaicParserParseError ParseError
    deriving (Eq, Show)

parseMosaicTokens :: [Token] -> Either ParseError ASTNode
parseMosaicTokens = parseWithGrammar parserGrammarData

tokenizeAndParseMosaic :: String -> Either MosaicParserError ASTNode
tokenizeAndParseMosaic source =
    case MosaicLexer.tokenizeMosaic source of
        Left err -> Left (MosaicParserLexerError err)
        Right tokens ->
            case parseMosaicTokens tokens of
                Left err -> Left (MosaicParserParseError err)
                Right ast -> Right ast
