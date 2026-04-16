module JavaParser
    ( description
    , JavaParserError(..)
    , parseJavaTokens
    , tokenizeAndParseJava
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified JavaLexer

description :: String
description = "Haskell java-parser backed by compiled parser grammar data"

data JavaParserError
    = JavaParserLexerError LexerError
    | JavaParserParseError ParseError
    deriving (Eq, Show)

parseJavaTokens :: [Token] -> Either ParseError ASTNode
parseJavaTokens = parseWithGrammar parserGrammarData

tokenizeAndParseJava :: String -> Either JavaParserError ASTNode
tokenizeAndParseJava source =
    case JavaLexer.tokenizeJava source of
        Left err -> Left (JavaParserLexerError err)
        Right tokens ->
            case parseJavaTokens tokens of
                Left err -> Left (JavaParserParseError err)
                Right ast -> Right ast
