module StarlarkLexer
    ( description
    , starlarkLexerKeywords
    , tokenizeStarlark
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell starlark-lexer backed by compiled token grammar data"

starlarkLexerKeywords :: [String]
starlarkLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeStarlark :: String -> Either LexerError [Token]
tokenizeStarlark = tokenizeWithGrammar tokenGrammarData
