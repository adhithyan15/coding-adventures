module JavaParser
    ( description
    , JavaParserError(..)
    , parseJavaTokens
    , tokenizeAndParseJava
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified JavaLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for java-parser built on the generic parser package"

data JavaParserError
    = JavaParserLexerError LexerError
    | JavaParserParseError ParseError
    deriving (Eq, Show)

parseJavaTokens :: [Token] -> Either ParseError ASTNode
parseJavaTokens = parseTokens

tokenizeAndParseJava :: String -> Either JavaParserError ASTNode
tokenizeAndParseJava source =
    case JavaLexer.tokenizeJava source of
        Left err -> Left (JavaParserLexerError err)
        Right tokens ->
            case parseJavaTokens tokens of
                Left err -> Left (JavaParserParseError err)
                Right ast -> Right ast
