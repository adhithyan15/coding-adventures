module ExcelLexer
    ( description
    , excelLexerKeywords
    , tokenizeExcel
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell excel-lexer backed by compiled token grammar data"

excelLexerKeywords :: [String]
excelLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeExcel :: String -> Either LexerError [Token]
tokenizeExcel = tokenizeWithGrammar tokenGrammarData
