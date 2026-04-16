module FsharpLexer
    ( description
    , fsharpLexerKeywords
    , tokenizeFsharp
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for fsharp-lexer built on the generic lexer package"

fsharpLexerKeywords :: [String]
fsharpLexerKeywords = []

tokenizeFsharp :: String -> Either LexerError [Token]
tokenizeFsharp = tokenize defaultLexerConfig {lexerKeywords = fsharpLexerKeywords}
