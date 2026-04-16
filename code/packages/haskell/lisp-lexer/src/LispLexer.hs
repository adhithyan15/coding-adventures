module LispLexer
    ( description
    , lispLexerKeywords
    , tokenizeLisp
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell lisp-lexer backed by compiled token grammar data"

lispLexerKeywords :: [String]
lispLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeLisp :: String -> Either LexerError [Token]
tokenizeLisp = tokenizeWithGrammar tokenGrammarData
