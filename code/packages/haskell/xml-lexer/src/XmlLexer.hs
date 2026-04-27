module XmlLexer
    ( description
    , xmlLexerKeywords
    , tokenizeXml
    ) where

import Lexer (LexerConfig(..), LexerError, Token, defaultLexerConfig, tokenize)

-- Thin wrapper around the shared lexer package so language-specific ports can
-- grow their own keyword tables and token helpers incrementally.
description :: String
description = "Haskell starter wrapper for xml-lexer built on the generic lexer package"

xmlLexerKeywords :: [String]
xmlLexerKeywords = []

tokenizeXml :: String -> Either LexerError [Token]
tokenizeXml = tokenize defaultLexerConfig {lexerKeywords = xmlLexerKeywords}
