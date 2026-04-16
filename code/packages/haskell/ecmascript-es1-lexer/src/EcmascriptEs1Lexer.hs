module EcmascriptEs1Lexer
    ( description
    , ecmascriptEs1LexerKeywords
    , tokenizeEcmascriptEs1
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell ecmascript-es1-lexer backed by compiled token grammar data"

ecmascriptEs1LexerKeywords :: [String]
ecmascriptEs1LexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeEcmascriptEs1 :: String -> Either LexerError [Token]
tokenizeEcmascriptEs1 = tokenizeWithGrammar tokenGrammarData
