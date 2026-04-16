module EcmascriptEs5Lexer
    ( description
    , ecmascriptEs5LexerKeywords
    , tokenizeEcmascriptEs5
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for ecmascript-es5-lexer built on the generic lexer package"

ecmascriptEs5LexerKeywords :: [String]
ecmascriptEs5LexerKeywords = []

tokenizeEcmascriptEs5 :: String -> Either LexerError [Token]
tokenizeEcmascriptEs5 = tokenize defaultLexerConfig {lexerKeywords = ecmascriptEs5LexerKeywords}
