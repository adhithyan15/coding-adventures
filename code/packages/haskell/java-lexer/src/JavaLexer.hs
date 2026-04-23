module JavaLexer
    ( description
    , javaLexerKeywords
    , tokenizeJava
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell java-lexer backed by compiled token grammar data"

javaLexerKeywords :: [String]
javaLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeJava :: String -> Either LexerError [Token]
tokenizeJava = tokenizeWithGrammar tokenGrammarData
