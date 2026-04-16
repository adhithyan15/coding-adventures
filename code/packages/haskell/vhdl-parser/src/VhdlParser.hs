module VhdlParser
    ( description
    , VhdlParserError(..)
    , parseVhdlTokens
    , tokenizeAndParseVhdl
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified VhdlLexer

description :: String
description = "Haskell vhdl-parser backed by compiled parser grammar data"

data VhdlParserError
    = VhdlParserLexerError LexerError
    | VhdlParserParseError ParseError
    deriving (Eq, Show)

parseVhdlTokens :: [Token] -> Either ParseError ASTNode
parseVhdlTokens = parseWithGrammar parserGrammarData

tokenizeAndParseVhdl :: String -> Either VhdlParserError ASTNode
tokenizeAndParseVhdl source =
    case VhdlLexer.tokenizeVhdl source of
        Left err -> Left (VhdlParserLexerError err)
        Right tokens ->
            case parseVhdlTokens tokens of
                Left err -> Left (VhdlParserParseError err)
                Right ast -> Right ast
