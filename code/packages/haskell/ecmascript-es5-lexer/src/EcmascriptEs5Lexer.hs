module EcmascriptEs5Lexer
    ( description
    , ecmascriptEs5LexerKeywords
    , tokenizeEcmascriptEs5
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell ecmascript-es5-lexer backed by compiled token grammar data"

ecmascriptEs5LexerKeywords :: [String]
ecmascriptEs5LexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeEcmascriptEs5 :: String -> Either LexerError [Token]
tokenizeEcmascriptEs5 = tokenizeWithGrammar tokenGrammarData
