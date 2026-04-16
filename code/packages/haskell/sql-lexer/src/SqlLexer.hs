module SqlLexer
    ( description
    , sqlLexerKeywords
    , tokenizeSql
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for sql-lexer built on the generic lexer package"

sqlLexerKeywords :: [String]
sqlLexerKeywords = []

tokenizeSql :: String -> Either LexerError [Token]
tokenizeSql = tokenize defaultLexerConfig {lexerKeywords = sqlLexerKeywords}
