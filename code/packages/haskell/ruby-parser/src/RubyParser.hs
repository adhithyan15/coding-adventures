module RubyParser
    ( description
    , RubyParserError(..)
    , parseRubyTokens
    , tokenizeAndParseRuby
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified RubyLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for ruby-parser built on the generic parser package"

data RubyParserError
    = RubyParserLexerError LexerError
    | RubyParserParseError ParseError
    deriving (Eq, Show)

parseRubyTokens :: [Token] -> Either ParseError ASTNode
parseRubyTokens = parseTokens

tokenizeAndParseRuby :: String -> Either RubyParserError ASTNode
tokenizeAndParseRuby source =
    case RubyLexer.tokenizeRuby source of
        Left err -> Left (RubyParserLexerError err)
        Right tokens ->
            case parseRubyTokens tokens of
                Left err -> Left (RubyParserParseError err)
                Right ast -> Right ast
