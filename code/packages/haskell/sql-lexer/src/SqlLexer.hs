module SqlLexer
    ( description
    , sqlLexerKeywords
    , tokenizeSql
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell sql-lexer backed by compiled token grammar data"

sqlLexerKeywords :: [String]
sqlLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeSql :: String -> Either LexerError [Token]
tokenizeSql = tokenizeWithGrammar tokenGrammarData
