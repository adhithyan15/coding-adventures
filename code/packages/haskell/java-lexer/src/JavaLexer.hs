module JavaLexer
    ( description
    , javaLexerKeywords
    , tokenizeJava
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for java-lexer built on the generic lexer package"

javaLexerKeywords :: [String]
javaLexerKeywords = []

tokenizeJava :: String -> Either LexerError [Token]
tokenizeJava = tokenize defaultLexerConfig {lexerKeywords = javaLexerKeywords}
