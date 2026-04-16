module AlgolParser
    ( description
    , AlgolParserError(..)
    , parseAlgolTokens
    , tokenizeAndParseAlgol
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified AlgolLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for algol-parser built on the generic parser package"

data AlgolParserError
    = AlgolParserLexerError LexerError
    | AlgolParserParseError ParseError
    deriving (Eq, Show)

parseAlgolTokens :: [Token] -> Either ParseError ASTNode
parseAlgolTokens = parseTokens

tokenizeAndParseAlgol :: String -> Either AlgolParserError ASTNode
tokenizeAndParseAlgol source =
    case AlgolLexer.tokenizeAlgol source of
        Left err -> Left (AlgolParserLexerError err)
        Right tokens ->
            case parseAlgolTokens tokens of
                Left err -> Left (AlgolParserParseError err)
                Right ast -> Right ast
