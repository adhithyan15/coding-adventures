module ExcelParser
    ( description
    , ExcelParserError(..)
    , parseExcelTokens
    , tokenizeAndParseExcel
    ) where

import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseTokens)
import qualified ExcelLexer

-- Starter parser wrapper that composes the shared parser engine with the
-- sibling lexer package for this language family.
description :: String
description = "Haskell starter wrapper for excel-parser built on the generic parser package"

data ExcelParserError
    = ExcelParserLexerError LexerError
    | ExcelParserParseError ParseError
    deriving (Eq, Show)

parseExcelTokens :: [Token] -> Either ParseError ASTNode
parseExcelTokens = parseTokens

tokenizeAndParseExcel :: String -> Either ExcelParserError ASTNode
tokenizeAndParseExcel source =
    case ExcelLexer.tokenizeExcel source of
        Left err -> Left (ExcelParserLexerError err)
        Right tokens ->
            case parseExcelTokens tokens of
                Left err -> Left (ExcelParserParseError err)
                Right ast -> Right ast
