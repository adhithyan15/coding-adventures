module AlgolLexer
    ( description
    , algolLexerKeywords
    , tokenizeAlgol
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for algol-lexer built on the generic lexer package"

algolLexerKeywords :: [String]
algolLexerKeywords = []

tokenizeAlgol :: String -> Either LexerError [Token]
tokenizeAlgol = tokenize defaultLexerConfig {lexerKeywords = algolLexerKeywords}
