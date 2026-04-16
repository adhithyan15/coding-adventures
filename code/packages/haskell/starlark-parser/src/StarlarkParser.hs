module StarlarkParser
    ( description
    , StarlarkParserError(..)
    , parseStarlarkTokens
    , tokenizeAndParseStarlark
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified StarlarkLexer

description :: String
description = "Haskell starlark-parser backed by compiled parser grammar data"

data StarlarkParserError
    = StarlarkParserLexerError LexerError
    | StarlarkParserParseError ParseError
    deriving (Eq, Show)

parseStarlarkTokens :: [Token] -> Either ParseError ASTNode
parseStarlarkTokens = parseWithGrammar parserGrammarData

tokenizeAndParseStarlark :: String -> Either StarlarkParserError ASTNode
tokenizeAndParseStarlark source =
    case StarlarkLexer.tokenizeStarlark source of
        Left err -> Left (StarlarkParserLexerError err)
        Right tokens ->
            case parseStarlarkTokens tokens of
                Left err -> Left (StarlarkParserParseError err)
                Right ast -> Right ast
