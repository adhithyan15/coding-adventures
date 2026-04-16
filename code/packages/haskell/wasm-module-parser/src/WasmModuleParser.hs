module WasmModuleParser
    ( description
    , parseWasmModuleTokens
    ) where

import Lexer (Token)
import Parser (ASTNode, ParseError, parseTokens)

-- Starter parser wrapper for grammars that do not yet have a sibling Haskell
-- lexer package in this first porting wave.
description :: String
description = "Haskell starter wrapper for wasm-module-parser built on the generic parser package"

parseWasmModuleTokens :: [Token] -> Either ParseError ASTNode
parseWasmModuleTokens = parseTokens
