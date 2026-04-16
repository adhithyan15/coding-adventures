module PythonLexer
    ( description
    , pythonLexerKeywords
    , tokenizePython
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell python-lexer backed by compiled token grammar data"

pythonLexerKeywords :: [String]
pythonLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizePython :: String -> Either LexerError [Token]
tokenizePython = tokenizeWithGrammar tokenGrammarData
