module LatticeLexer
    ( description
    , latticeLexerKeywords
    , tokenizeLattice
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for lattice-lexer built on the generic lexer package"

latticeLexerKeywords :: [String]
latticeLexerKeywords = []

tokenizeLattice :: String -> Either LexerError [Token]
tokenizeLattice = tokenize defaultLexerConfig {lexerKeywords = latticeLexerKeywords}
