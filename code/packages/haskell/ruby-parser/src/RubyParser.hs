module RubyParser
    ( description
    , RubyParserError(..)
    , parseRubyTokens
    , tokenizeAndParseRuby
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified RubyLexer

description :: String
description = "Haskell ruby-parser backed by compiled parser grammar data"

data RubyParserError
    = RubyParserLexerError LexerError
    | RubyParserParseError ParseError
    deriving (Eq, Show)

parseRubyTokens :: [Token] -> Either ParseError ASTNode
parseRubyTokens = parseWithGrammar parserGrammarData

tokenizeAndParseRuby :: String -> Either RubyParserError ASTNode
tokenizeAndParseRuby source =
    case RubyLexer.tokenizeRuby source of
        Left err -> Left (RubyParserLexerError err)
        Right tokens ->
            case parseRubyTokens tokens of
                Left err -> Left (RubyParserParseError err)
                Right ast -> Right ast
