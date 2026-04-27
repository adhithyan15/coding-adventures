module DartmouthBasicParser
    ( description
    , DartmouthBasicParserError(..)
    , parseDartmouthBasicTokens
    , tokenizeAndParseDartmouthBasic
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified DartmouthBasicLexer

description :: String
description = "Haskell dartmouth-basic-parser backed by compiled parser grammar data"

data DartmouthBasicParserError
    = DartmouthBasicParserLexerError LexerError
    | DartmouthBasicParserParseError ParseError
    deriving (Eq, Show)

parseDartmouthBasicTokens :: [Token] -> Either ParseError ASTNode
parseDartmouthBasicTokens = parseWithGrammar parserGrammarData

tokenizeAndParseDartmouthBasic :: String -> Either DartmouthBasicParserError ASTNode
tokenizeAndParseDartmouthBasic source =
    case DartmouthBasicLexer.tokenizeDartmouthBasic source of
        Left err -> Left (DartmouthBasicParserLexerError err)
        Right tokens ->
            case parseDartmouthBasicTokens tokens of
                Left err -> Left (DartmouthBasicParserParseError err)
                Right ast -> Right ast
