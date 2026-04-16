module JavascriptLexer
    ( description
    , javascriptLexerKeywords
    , tokenizeJavascript
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for javascript-lexer built on the generic lexer package"

javascriptLexerKeywords :: [String]
javascriptLexerKeywords = []

tokenizeJavascript :: String -> Either LexerError [Token]
tokenizeJavascript = tokenize defaultLexerConfig {lexerKeywords = javascriptLexerKeywords}
