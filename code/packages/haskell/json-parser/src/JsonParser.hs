module JsonParser
    ( description
    , JsonParserError(..)
    , parseJsonTokens
    , tokenizeAndParseJson
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import GrammarTools (parseParserGrammar)
import qualified JsonLexer

description :: String
description = "Haskell JSON parser backed by the shared grammar-driven parser runtime"

data JsonParserError
    = JsonParserLexerError LexerError
    | JsonParserParseError ParseError
    deriving (Eq, Show)

parseJsonTokens :: [Token] -> Either ParseError ASTNode
parseJsonTokens = parseWithGrammar jsonParserGrammar

tokenizeAndParseJson :: String -> Either JsonParserError ASTNode
tokenizeAndParseJson source =
    case JsonLexer.tokenizeJson source of
        Left err -> Left (JsonParserLexerError err)
        Right tokens ->
            case parseJsonTokens tokens of
                Left err -> Left (JsonParserParseError err)
                Right ast -> Right ast

jsonParserGrammarSource :: String
jsonParserGrammarSource =
    unlines
        [ "value = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;"
        , "object = LBRACE [ pair { COMMA pair } ] RBRACE ;"
        , "pair = STRING COLON value ;"
        , "array = LBRACKET [ value { COMMA value } ] RBRACKET ;"
        ]

jsonParserGrammar = 
    case parseParserGrammar jsonParserGrammarSource of
        Left err -> error (show err)
        Right grammar -> grammar
