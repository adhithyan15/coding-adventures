module NibLexer
    ( description
    , nibLexerKeywords
    , tokenizeNib
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell nib-lexer backed by compiled token grammar data"

nibLexerKeywords :: [String]
nibLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeNib :: String -> Either LexerError [Token]
tokenizeNib = tokenizeWithGrammar tokenGrammarData
