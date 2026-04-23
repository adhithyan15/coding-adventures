module FsharpLexer
    ( description
    , fsharpLexerKeywords
    , tokenizeFsharp
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell fsharp-lexer backed by compiled token grammar data"

fsharpLexerKeywords :: [String]
fsharpLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeFsharp :: String -> Either LexerError [Token]
tokenizeFsharp = tokenizeWithGrammar tokenGrammarData
