module CommonmarkParser
    ( description
    , parseCommonmarkTokens
    ) where

import Lexer (Token)
import Parser (ASTNode, ParseError, parseTokens)

-- Starter parser wrapper for grammars that do not yet have a sibling Haskell
-- lexer package in this first porting wave.
description :: String
description = "Haskell starter wrapper for commonmark-parser built on the generic parser package"

parseCommonmarkTokens :: [Token] -> Either ParseError ASTNode
parseCommonmarkTokens = parseTokens
