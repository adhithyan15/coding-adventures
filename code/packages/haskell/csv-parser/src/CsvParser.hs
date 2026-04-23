module CsvParser
    ( description
    , parseCsvTokens
    ) where

import Lexer (Token)
import Parser (ASTNode, ParseError, parseTokens)

-- Starter parser wrapper for grammars that do not yet have a sibling Haskell
-- lexer package in this first porting wave.
description :: String
description = "Haskell starter wrapper for csv-parser built on the generic parser package"

parseCsvTokens :: [Token] -> Either ParseError ASTNode
parseCsvTokens = parseTokens
