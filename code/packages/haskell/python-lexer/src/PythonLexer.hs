module PythonLexer
    ( description
    , pythonLexerKeywords
    , tokenizePython
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for python-lexer built on the generic lexer package"

pythonLexerKeywords :: [String]
pythonLexerKeywords = []

tokenizePython :: String -> Either LexerError [Token]
tokenizePython = tokenize defaultLexerConfig {lexerKeywords = pythonLexerKeywords}
