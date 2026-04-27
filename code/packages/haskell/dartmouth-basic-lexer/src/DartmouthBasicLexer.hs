module DartmouthBasicLexer
    ( description
    , dartmouthBasicLexerKeywords
    , tokenizeDartmouthBasic
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell dartmouth-basic-lexer backed by compiled token grammar data"

dartmouthBasicLexerKeywords :: [String]
dartmouthBasicLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeDartmouthBasic :: String -> Either LexerError [Token]
tokenizeDartmouthBasic = tokenizeWithGrammar tokenGrammarData
