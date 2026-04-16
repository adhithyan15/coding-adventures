module VerilogParser
    ( description
    , VerilogParserError(..)
    , parseVerilogTokens
    , tokenizeAndParseVerilog
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified VerilogLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for verilog-parser built on the generic parser package"

data VerilogParserError
    = VerilogParserLexerError LexerError
    | VerilogParserParseError ParseError
    deriving (Eq, Show)

parseVerilogTokens :: [Token] -> Either ParseError ASTNode
parseVerilogTokens = parseTokens

tokenizeAndParseVerilog :: String -> Either VerilogParserError ASTNode
tokenizeAndParseVerilog source =
    case VerilogLexer.tokenizeVerilog source of
        Left err -> Left (VerilogParserLexerError err)
        Right tokens ->
            case parseVerilogTokens tokens of
                Left err -> Left (VerilogParserParseError err)
                Right ast -> Right ast
