module CsharpLexer
    ( description
    , csharpLexerKeywords
    , tokenizeCsharp
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for csharp-lexer built on the generic lexer package"

csharpLexerKeywords :: [String]
csharpLexerKeywords = []

tokenizeCsharp :: String -> Either LexerError [Token]
tokenizeCsharp = tokenize defaultLexerConfig {lexerKeywords = csharpLexerKeywords}
