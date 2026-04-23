module AlgolLexer
    ( description
    , algolLexerKeywords
    , tokenizeAlgol
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell algol-lexer backed by compiled token grammar data"

algolLexerKeywords :: [String]
algolLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeAlgol :: String -> Either LexerError [Token]
tokenizeAlgol = tokenizeWithGrammar tokenGrammarData
