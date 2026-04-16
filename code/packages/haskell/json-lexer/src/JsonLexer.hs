module JsonLexer
    ( description
    , jsonLexerKeywords
    , tokenizeJson
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for json-lexer built on the generic lexer package"

jsonLexerKeywords :: [String]
jsonLexerKeywords = []

tokenizeJson :: String -> Either LexerError [Token]
tokenizeJson = tokenize defaultLexerConfig {lexerKeywords = jsonLexerKeywords}
