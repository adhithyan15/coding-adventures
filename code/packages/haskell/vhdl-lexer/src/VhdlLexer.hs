module VhdlLexer
    ( description
    , vhdlLexerKeywords
    , tokenizeVhdl
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell vhdl-lexer backed by compiled token grammar data"

vhdlLexerKeywords :: [String]
vhdlLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeVhdl :: String -> Either LexerError [Token]
tokenizeVhdl = tokenizeWithGrammar tokenGrammarData
