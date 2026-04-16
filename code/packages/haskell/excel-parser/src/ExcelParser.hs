module ExcelParser
    ( description
    , ExcelParserError(..)
    , parseExcelTokens
    , tokenizeAndParseExcel
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified ExcelLexer

description :: String
description = "Haskell excel-parser backed by compiled parser grammar data"

data ExcelParserError
    = ExcelParserLexerError LexerError
    | ExcelParserParseError ParseError
    deriving (Eq, Show)

parseExcelTokens :: [Token] -> Either ParseError ASTNode
parseExcelTokens = parseWithGrammar parserGrammarData

tokenizeAndParseExcel :: String -> Either ExcelParserError ASTNode
tokenizeAndParseExcel source =
    case ExcelLexer.tokenizeExcel source of
        Left err -> Left (ExcelParserLexerError err)
        Right tokens ->
            case parseExcelTokens tokens of
                Left err -> Left (ExcelParserParseError err)
                Right ast -> Right ast
