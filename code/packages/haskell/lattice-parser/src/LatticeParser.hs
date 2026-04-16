module LatticeParser
    ( description
    , LatticeParserError(..)
    , parseLatticeTokens
    , tokenizeAndParseLattice
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified LatticeLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for lattice-parser built on the generic parser package"

data LatticeParserError
    = LatticeParserLexerError LexerError
    | LatticeParserParseError ParseError
    deriving (Eq, Show)

parseLatticeTokens :: [Token] -> Either ParseError ASTNode
parseLatticeTokens = parseTokens

tokenizeAndParseLattice :: String -> Either LatticeParserError ASTNode
tokenizeAndParseLattice source =
    case LatticeLexer.tokenizeLattice source of
        Left err -> Left (LatticeParserLexerError err)
        Right tokens ->
            case parseLatticeTokens tokens of
                Left err -> Left (LatticeParserParseError err)
                Right ast -> Right ast
