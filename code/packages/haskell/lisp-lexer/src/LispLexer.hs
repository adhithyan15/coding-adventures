module LispLexer
    ( description
    , lispLexerKeywords
    , tokenizeLisp
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for lisp-lexer built on the generic lexer package"

lispLexerKeywords :: [String]
lispLexerKeywords = []

tokenizeLisp :: String -> Either LexerError [Token]
tokenizeLisp = tokenize defaultLexerConfig {lexerKeywords = lispLexerKeywords}
