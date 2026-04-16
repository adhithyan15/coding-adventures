module TomlLexer
    ( description
    , tomlLexerKeywords
    , tokenizeToml
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for toml-lexer built on the generic lexer package"

tomlLexerKeywords :: [String]
tomlLexerKeywords = []

tokenizeToml :: String -> Either LexerError [Token]
tokenizeToml = tokenize defaultLexerConfig {lexerKeywords = tomlLexerKeywords}
