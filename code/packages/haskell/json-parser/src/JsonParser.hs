module JsonParser
    ( description
    , JsonParserError(..)
    , parseJsonTokens
    , tokenizeAndParseJson
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified JsonLexer

description :: String
description = "Haskell json-parser backed by compiled parser grammar data"

data JsonParserError
    = JsonParserLexerError LexerError
    | JsonParserParseError ParseError
    deriving (Eq, Show)

parseJsonTokens :: [Token] -> Either ParseError ASTNode
parseJsonTokens = parseWithGrammar parserGrammarData

tokenizeAndParseJson :: String -> Either JsonParserError ASTNode
tokenizeAndParseJson source =
    case JsonLexer.tokenizeJson source of
        Left err -> Left (JsonParserLexerError err)
        Right tokens ->
            case parseJsonTokens tokens of
                Left err -> Left (JsonParserParseError err)
                Right ast -> Right ast
