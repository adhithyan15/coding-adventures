module DartmouthBasicLexer
    ( description
    , dartmouthBasicLexerKeywords
    , tokenizeDartmouthBasic
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for dartmouth-basic-lexer built on the generic lexer package"

dartmouthBasicLexerKeywords :: [String]
dartmouthBasicLexerKeywords = []

tokenizeDartmouthBasic :: String -> Either LexerError [Token]
tokenizeDartmouthBasic = tokenize defaultLexerConfig {lexerKeywords = dartmouthBasicLexerKeywords}
