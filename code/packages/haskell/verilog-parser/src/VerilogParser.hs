module VerilogParser
    ( description
    , VerilogParserError(..)
    , parseVerilogTokens
    , tokenizeAndParseVerilog
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified VerilogLexer

description :: String
description = "Haskell verilog-parser backed by compiled parser grammar data"

data VerilogParserError
    = VerilogParserLexerError LexerError
    | VerilogParserParseError ParseError
    deriving (Eq, Show)

parseVerilogTokens :: [Token] -> Either ParseError ASTNode
parseVerilogTokens = parseWithGrammar parserGrammarData

tokenizeAndParseVerilog :: String -> Either VerilogParserError ASTNode
tokenizeAndParseVerilog source =
    case VerilogLexer.tokenizeVerilog source of
        Left err -> Left (VerilogParserLexerError err)
        Right tokens ->
            case parseVerilogTokens tokens of
                Left err -> Left (VerilogParserParseError err)
                Right ast -> Right ast
