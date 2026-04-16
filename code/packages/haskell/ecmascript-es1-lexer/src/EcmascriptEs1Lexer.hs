module EcmascriptEs1Lexer
    ( description
    , ecmascriptEs1LexerKeywords
    , tokenizeEcmascriptEs1
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for ecmascript-es1-lexer built on the generic lexer package"

ecmascriptEs1LexerKeywords :: [String]
ecmascriptEs1LexerKeywords = []

tokenizeEcmascriptEs1 :: String -> Either LexerError [Token]
tokenizeEcmascriptEs1 = tokenize defaultLexerConfig {lexerKeywords = ecmascriptEs1LexerKeywords}
