module LatticeParser
    ( description
    , LatticeParserError(..)
    , parseLatticeTokens
    , tokenizeAndParseLattice
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified LatticeLexer

description :: String
description = "Haskell lattice-parser backed by compiled parser grammar data"

data LatticeParserError
    = LatticeParserLexerError LexerError
    | LatticeParserParseError ParseError
    deriving (Eq, Show)

parseLatticeTokens :: [Token] -> Either ParseError ASTNode
parseLatticeTokens = parseWithGrammar parserGrammarData

tokenizeAndParseLattice :: String -> Either LatticeParserError ASTNode
tokenizeAndParseLattice source =
    case LatticeLexer.tokenizeLattice source of
        Left err -> Left (LatticeParserLexerError err)
        Right tokens ->
            case parseLatticeTokens tokens of
                Left err -> Left (LatticeParserParseError err)
                Right ast -> Right ast
