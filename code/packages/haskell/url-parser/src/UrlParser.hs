module UrlParser
    ( description
    , parseUrlTokens
    ) where

import Lexer (Token)
import Parser (ASTNode, ParseError, parseTokens)

-- Starter parser wrapper for grammars that do not yet have a sibling Haskell
-- lexer package in this first porting wave.
description :: String
description = "Haskell starter wrapper for url-parser built on the generic parser package"

parseUrlTokens :: [Token] -> Either ParseError ASTNode
parseUrlTokens = parseTokens
