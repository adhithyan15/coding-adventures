module CssLexer
    ( description
    , cssLexerKeywords
    , tokenizeCss
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for css-lexer built on the generic lexer package"

cssLexerKeywords :: [String]
cssLexerKeywords = []

tokenizeCss :: String -> Either LexerError [Token]
tokenizeCss = tokenize defaultLexerConfig {lexerKeywords = cssLexerKeywords}
