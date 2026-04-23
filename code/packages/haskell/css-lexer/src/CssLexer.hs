module CssLexer
    ( description
    , cssLexerKeywords
    , tokenizeCss
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell css-lexer backed by compiled token grammar data"

cssLexerKeywords :: [String]
cssLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeCss :: String -> Either LexerError [Token]
tokenizeCss = tokenizeWithGrammar tokenGrammarData
