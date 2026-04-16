module VhdlLexer
    ( description
    , vhdlLexerKeywords
    , tokenizeVhdl
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for vhdl-lexer built on the generic lexer package"

vhdlLexerKeywords :: [String]
vhdlLexerKeywords = []

tokenizeVhdl :: String -> Either LexerError [Token]
tokenizeVhdl = tokenize defaultLexerConfig {lexerKeywords = vhdlLexerKeywords}
