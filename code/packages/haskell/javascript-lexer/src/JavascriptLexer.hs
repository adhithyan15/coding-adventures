module JavascriptLexer
    ( description
    , javascriptLexerKeywords
    , tokenizeJavascript
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell javascript-lexer backed by compiled token grammar data"

javascriptLexerKeywords :: [String]
javascriptLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeJavascript :: String -> Either LexerError [Token]
tokenizeJavascript = tokenizeWithGrammar tokenGrammarData
