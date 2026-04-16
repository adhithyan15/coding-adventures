module TypescriptLexer
    ( description
    , typescriptLexerKeywords
    , tokenizeTypescript
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell typescript-lexer backed by compiled token grammar data"

typescriptLexerKeywords :: [String]
typescriptLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeTypescript :: String -> Either LexerError [Token]
tokenizeTypescript = tokenizeWithGrammar tokenGrammarData
