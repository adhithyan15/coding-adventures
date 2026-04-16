module ExcelLexer
    ( description
    , excelLexerKeywords
    , tokenizeExcel
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for excel-lexer built on the generic lexer package"

excelLexerKeywords :: [String]
excelLexerKeywords = []

tokenizeExcel :: String -> Either LexerError [Token]
tokenizeExcel = tokenize defaultLexerConfig {lexerKeywords = excelLexerKeywords}
