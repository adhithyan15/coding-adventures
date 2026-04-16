module VhdlParser
    ( description
    , VhdlParserError(..)
    , parseVhdlTokens
    , tokenizeAndParseVhdl
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified VhdlLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for vhdl-parser built on the generic parser package"

data VhdlParserError
    = VhdlParserLexerError LexerError
    | VhdlParserParseError ParseError
    deriving (Eq, Show)

parseVhdlTokens :: [Token] -> Either ParseError ASTNode
parseVhdlTokens = parseTokens

tokenizeAndParseVhdl :: String -> Either VhdlParserError ASTNode
tokenizeAndParseVhdl source =
    case VhdlLexer.tokenizeVhdl source of
        Left err -> Left (VhdlParserLexerError err)
        Right tokens ->
            case parseVhdlTokens tokens of
                Left err -> Left (VhdlParserParseError err)
                Right ast -> Right ast
