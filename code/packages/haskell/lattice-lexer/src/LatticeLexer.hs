module LatticeLexer
    ( description
    , latticeLexerKeywords
    , tokenizeLattice
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell lattice-lexer backed by compiled token grammar data"

latticeLexerKeywords :: [String]
latticeLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeLattice :: String -> Either LexerError [Token]
tokenizeLattice = tokenizeWithGrammar tokenGrammarData
