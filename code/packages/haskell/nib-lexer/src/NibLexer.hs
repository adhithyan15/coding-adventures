module NibLexer
    ( description
    , nibLexerKeywords
    , tokenizeNib
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for nib-lexer built on the generic lexer package"

nibLexerKeywords :: [String]
nibLexerKeywords = []

tokenizeNib :: String -> Either LexerError [Token]
tokenizeNib = tokenize defaultLexerConfig {lexerKeywords = nibLexerKeywords}
