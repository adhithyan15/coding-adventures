module TypescriptLexer
    ( description
    , typescriptLexerKeywords
    , tokenizeTypescript
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for typescript-lexer built on the generic lexer package"

typescriptLexerKeywords :: [String]
typescriptLexerKeywords = []

tokenizeTypescript :: String -> Either LexerError [Token]
tokenizeTypescript = tokenize defaultLexerConfig {lexerKeywords = typescriptLexerKeywords}
