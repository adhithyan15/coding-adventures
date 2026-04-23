module RubyLexer
    ( description
    , rubyLexerKeywords
    , tokenizeRuby
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell ruby-lexer backed by compiled token grammar data"

rubyLexerKeywords :: [String]
rubyLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeRuby :: String -> Either LexerError [Token]
tokenizeRuby = tokenizeWithGrammar tokenGrammarData
