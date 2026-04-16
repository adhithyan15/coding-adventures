module EcmascriptEs3Lexer
    ( description
    , ecmascriptEs3LexerKeywords
    , tokenizeEcmascriptEs3
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for ecmascript-es3-lexer built on the generic lexer package"

ecmascriptEs3LexerKeywords :: [String]
ecmascriptEs3LexerKeywords = []

tokenizeEcmascriptEs3 :: String -> Either LexerError [Token]
tokenizeEcmascriptEs3 = tokenize defaultLexerConfig {lexerKeywords = ecmascriptEs3LexerKeywords}
