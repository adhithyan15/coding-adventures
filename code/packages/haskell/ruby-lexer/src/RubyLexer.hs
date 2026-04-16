module RubyLexer
    ( description
    , rubyLexerKeywords
    , tokenizeRuby
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for ruby-lexer built on the generic lexer package"

rubyLexerKeywords :: [String]
rubyLexerKeywords = []

tokenizeRuby :: String -> Either LexerError [Token]
tokenizeRuby = tokenize defaultLexerConfig {lexerKeywords = rubyLexerKeywords}
