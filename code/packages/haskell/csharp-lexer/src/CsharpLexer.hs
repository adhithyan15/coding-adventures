module CsharpLexer
    ( description
    , csharpLexerKeywords
    , tokenizeCsharp
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell csharp-lexer backed by compiled token grammar data"

csharpLexerKeywords :: [String]
csharpLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeCsharp :: String -> Either LexerError [Token]
tokenizeCsharp = tokenizeWithGrammar tokenGrammarData
