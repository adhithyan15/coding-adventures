module MosaicLexer
    ( description
    , mosaicLexerKeywords
    , tokenizeMosaic
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for mosaic-lexer built on the generic lexer package"

mosaicLexerKeywords :: [String]
mosaicLexerKeywords = []

tokenizeMosaic :: String -> Either LexerError [Token]
tokenizeMosaic = tokenize defaultLexerConfig {lexerKeywords = mosaicLexerKeywords}
