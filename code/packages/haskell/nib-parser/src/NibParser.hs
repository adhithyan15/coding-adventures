module NibParser
    ( description
    , NibParserError(..)
    , parseNibTokens
    , tokenizeAndParseNib
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified NibLexer

description :: String
description = "Haskell nib-parser backed by compiled parser grammar data"

data NibParserError
    = NibParserLexerError LexerError
    | NibParserParseError ParseError
    deriving (Eq, Show)

parseNibTokens :: [Token] -> Either ParseError ASTNode
parseNibTokens = parseWithGrammar parserGrammarData

tokenizeAndParseNib :: String -> Either NibParserError ASTNode
tokenizeAndParseNib source =
    case NibLexer.tokenizeNib source of
        Left err -> Left (NibParserLexerError err)
        Right tokens ->
            case parseNibTokens tokens of
                Left err -> Left (NibParserParseError err)
                Right ast -> Right ast
