module TomlLexer
    ( description
    , tomlLexerKeywords
    , tokenizeToml
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell toml-lexer backed by compiled token grammar data"

tomlLexerKeywords :: [String]
tomlLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeToml :: String -> Either LexerError [Token]
tokenizeToml = tokenizeWithGrammar tokenGrammarData
