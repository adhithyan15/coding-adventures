module TomlParser
    ( description
    , TomlParserError(..)
    , parseTomlTokens
    , tokenizeAndParseToml
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified TomlLexer

description :: String
description = "Haskell toml-parser backed by compiled parser grammar data"

data TomlParserError
    = TomlParserLexerError LexerError
    | TomlParserParseError ParseError
    deriving (Eq, Show)

parseTomlTokens :: [Token] -> Either ParseError ASTNode
parseTomlTokens = parseWithGrammar parserGrammarData

tokenizeAndParseToml :: String -> Either TomlParserError ASTNode
tokenizeAndParseToml source =
    case TomlLexer.tokenizeToml source of
        Left err -> Left (TomlParserLexerError err)
        Right tokens ->
            case parseTomlTokens tokens of
                Left err -> Left (TomlParserParseError err)
                Right ast -> Right ast
