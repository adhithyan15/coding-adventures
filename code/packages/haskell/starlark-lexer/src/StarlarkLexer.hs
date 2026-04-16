module StarlarkLexer
    ( description
    , starlarkLexerKeywords
    , tokenizeStarlark
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for starlark-lexer built on the generic lexer package"

starlarkLexerKeywords :: [String]
starlarkLexerKeywords = []

tokenizeStarlark :: String -> Either LexerError [Token]
tokenizeStarlark = tokenize defaultLexerConfig {lexerKeywords = starlarkLexerKeywords}
