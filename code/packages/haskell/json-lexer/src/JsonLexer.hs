module JsonLexer
    ( description
    , jsonLexerKeywords
    , tokenizeJson
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell json-lexer backed by compiled token grammar data"

jsonLexerKeywords :: [String]
jsonLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeJson :: String -> Either LexerError [Token]
tokenizeJson = tokenizeWithGrammar tokenGrammarData
