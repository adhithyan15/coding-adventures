module VerilogLexer
    ( description
    , verilogLexerKeywords
    , tokenizeVerilog
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell verilog-lexer backed by compiled token grammar data"

verilogLexerKeywords :: [String]
verilogLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeVerilog :: String -> Either LexerError [Token]
tokenizeVerilog = tokenizeWithGrammar tokenGrammarData
