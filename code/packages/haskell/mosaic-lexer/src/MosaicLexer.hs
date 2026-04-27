module MosaicLexer
    ( description
    , mosaicLexerKeywords
    , tokenizeMosaic
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell mosaic-lexer backed by compiled token grammar data"

mosaicLexerKeywords :: [String]
mosaicLexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenizeMosaic :: String -> Either LexerError [Token]
tokenizeMosaic = tokenizeWithGrammar tokenGrammarData
