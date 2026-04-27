module EcmascriptEs3Lexer
    ( description
    , ecmascriptEs3LexerKeywords
    , tokenizeEcmascriptEs3
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell ecmascript-es3-lexer backed by compiled token grammar data"

ecmascriptEs3LexerKeywords :: [String]
ecmascriptEs3LexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeEcmascriptEs3 :: String -> Either LexerError [Token]
tokenizeEcmascriptEs3 = tokenizeWithGrammar tokenGrammarData
