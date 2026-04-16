module VerilogLexer
    ( description
    , verilogLexerKeywords
    , tokenizeVerilog
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for verilog-lexer built on the generic lexer package"

verilogLexerKeywords :: [String]
verilogLexerKeywords = []

tokenizeVerilog :: String -> Either LexerError [Token]
tokenizeVerilog = tokenize defaultLexerConfig {lexerKeywords = verilogLexerKeywords}
