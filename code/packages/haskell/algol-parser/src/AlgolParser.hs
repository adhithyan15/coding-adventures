module AlgolParser
    ( description
    , AlgolParserError(..)
    , parseAlgolTokens
    , tokenizeAndParseAlgol
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified AlgolLexer

description :: String
description = "Haskell algol-parser backed by compiled parser grammar data"

data AlgolParserError
    = AlgolParserLexerError LexerError
    | AlgolParserParseError ParseError
    deriving (Eq, Show)

parseAlgolTokens :: [Token] -> Either ParseError ASTNode
parseAlgolTokens = parseWithGrammar parserGrammarData

tokenizeAndParseAlgol :: String -> Either AlgolParserError ASTNode
tokenizeAndParseAlgol source =
    case AlgolLexer.tokenizeAlgol source of
        Left err -> Left (AlgolParserLexerError err)
        Right tokens ->
            case parseAlgolTokens tokens of
                Left err -> Left (AlgolParserParseError err)
                Right ast -> Right ast
