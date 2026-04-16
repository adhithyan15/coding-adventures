module JsonLexer
    ( description
    , jsonLexerKeywords
    , tokenizeJson
    ) where

import Data.Char (chr, isDigit)
import Numeric (readHex)
import Lexer (LexerError(..), Token, TokenType(..), makeToken, withTypeName)

data Scanner = Scanner
    { scannerRemaining :: String
    , scannerLine :: Int
    , scannerColumn :: Int
    }
    deriving (Eq, Show)

description :: String
description = "Haskell JSON lexer with dedicated number, string, and literal handling"

jsonLexerKeywords :: [String]
jsonLexerKeywords = ["true", "false", "null"]

tokenizeJson :: String -> Either LexerError [Token]
tokenizeJson source = scanTokens initialScanner []
  where
    initialScanner =
        Scanner
            { scannerRemaining = source
            , scannerLine = 1
            , scannerColumn = 1
            }

scanTokens :: Scanner -> [Token] -> Either LexerError [Token]
scanTokens scanner tokens =
    case skipWhitespace scanner of
        current
            | isAtEnd current ->
                Right (tokens ++ [makeToken TokenEof "" (scannerLine current) (scannerColumn current)])
            | otherwise ->
                case currentChar current of
                    Just '"' -> do
                        (token, nextScanner) <- readString current
                        scanTokens nextScanner (tokens ++ [token])
                    Just '-' -> do
                        (token, nextScanner) <- readNumber current
                        scanTokens nextScanner (tokens ++ [token])
                    Just ch
                        | isDigit ch -> do
                            (token, nextScanner) <- readNumber current
                            scanTokens nextScanner (tokens ++ [token])
                    Just '{' ->
                        continueWith (makeToken TokenLBrace "{" (scannerLine current) (scannerColumn current)) (advanceChar current) tokens
                    Just '}' ->
                        continueWith (makeToken TokenRBrace "}" (scannerLine current) (scannerColumn current)) (advanceChar current) tokens
                    Just '[' ->
                        continueWith (makeToken TokenLBracket "[" (scannerLine current) (scannerColumn current)) (advanceChar current) tokens
                    Just ']' ->
                        continueWith (makeToken TokenRBracket "]" (scannerLine current) (scannerColumn current)) (advanceChar current) tokens
                    Just ':' ->
                        continueWith (makeToken TokenColon ":" (scannerLine current) (scannerColumn current)) (advanceChar current) tokens
                    Just ',' ->
                        continueWith (makeToken TokenComma "," (scannerLine current) (scannerColumn current)) (advanceChar current) tokens
                    Just 't' -> readLiteral "true" "TRUE" current >>= advanceWith tokens
                    Just 'f' -> readLiteral "false" "FALSE" current >>= advanceWith tokens
                    Just 'n' -> readLiteral "null" "NULL" current >>= advanceWith tokens
                    _ ->
                        Left
                            LexerError
                                { lexerErrorMessage = "unexpected character in JSON input"
                                , lexerErrorLine = scannerLine current
                                , lexerErrorColumn = scannerColumn current
                                }

continueWith :: Token -> Scanner -> [Token] -> Either LexerError [Token]
continueWith token nextScanner tokens =
    scanTokens nextScanner (tokens ++ [token])

advanceWith :: [Token] -> (Token, Scanner) -> Either LexerError [Token]
advanceWith tokens (token, nextScanner) =
    scanTokens nextScanner (tokens ++ [token])

skipWhitespace :: Scanner -> Scanner
skipWhitespace scanner =
    case currentChar scanner of
        Just ch
            | ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' ->
                skipWhitespace (advanceChar scanner)
        _ -> scanner

readLiteral :: String -> String -> Scanner -> Either LexerError (Token, Scanner)
readLiteral expected customTypeName scanner =
    if take (length expected) (scannerRemaining scanner) == expected
        then
            Right
                ( withTypeName customTypeName (makeToken TokenKeyword expected (scannerLine scanner) (scannerColumn scanner))
                , advanceChars (length expected) scanner
                )
        else
            Left
                LexerError
                    { lexerErrorMessage = "invalid JSON literal"
                    , lexerErrorLine = scannerLine scanner
                    , lexerErrorColumn = scannerColumn scanner
                    }

readString :: Scanner -> Either LexerError (Token, Scanner)
readString scanner =
    go [] (advanceChar scanner)
  where
    startLine = scannerLine scanner
    startColumn = scannerColumn scanner

    go acc current =
        case currentChar current of
            Nothing ->
                Left
                    LexerError
                        { lexerErrorMessage = "unterminated JSON string"
                        , lexerErrorLine = startLine
                        , lexerErrorColumn = startColumn
                        }
            Just '"' ->
                Right (makeToken TokenString (reverse acc) startLine startColumn, advanceChar current)
            Just '\\' ->
                case parseEscape current of
                    Left err -> Left err
                    Right (escaped, nextScanner) -> go (reverse escaped ++ acc) nextScanner
            Just ch ->
                if ch < ' '
                    then
                        Left
                            LexerError
                                { lexerErrorMessage = "control character in JSON string"
                                , lexerErrorLine = scannerLine current
                                , lexerErrorColumn = scannerColumn current
                                }
                    else go ([ch] ++ acc) (advanceChar current)

parseEscape :: Scanner -> Either LexerError (String, Scanner)
parseEscape scanner =
    case currentChar (advanceChar scanner) of
        Nothing ->
            Left
                LexerError
                    { lexerErrorMessage = "unterminated JSON escape"
                    , lexerErrorLine = scannerLine scanner
                    , lexerErrorColumn = scannerColumn scanner
                    }
        Just escaped ->
            case escaped of
                '"' -> Right ("\"", advanceChar (advanceChar scanner))
                '\\' -> Right ("\\", advanceChar (advanceChar scanner))
                '/' -> Right ("/", advanceChar (advanceChar scanner))
                'b' -> Right ("\b", advanceChar (advanceChar scanner))
                'f' -> Right ("\f", advanceChar (advanceChar scanner))
                'n' -> Right ("\n", advanceChar (advanceChar scanner))
                'r' -> Right ("\r", advanceChar (advanceChar scanner))
                't' -> Right ("\t", advanceChar (advanceChar scanner))
                'u' ->
                    let unicodeStart = advanceChar (advanceChar scanner)
                        digits = take 4 (scannerRemaining unicodeStart)
                     in if length digits == 4 && all isHexDigit digits
                            then
                                case readHex digits of
                                    [(value, "")] ->
                                        Right ([chr value], advanceChars 4 unicodeStart)
                                    _ ->
                                        invalidEscape unicodeStart
                            else invalidEscape unicodeStart
                _ -> invalidEscape (advanceChar scanner)
  where
    invalidEscape current =
        Left
            LexerError
                { lexerErrorMessage = "invalid JSON escape"
                , lexerErrorLine = scannerLine current
                , lexerErrorColumn = scannerColumn current
                }

readNumber :: Scanner -> Either LexerError (Token, Scanner)
readNumber scanner = do
    let startLine = scannerLine scanner
        startColumn = scannerColumn scanner
        (signScanner, seenMinus) =
            case currentChar scanner of
                Just '-' -> (advanceChar scanner, True)
                _ -> (scanner, False)
    integerScanner <-
        case currentChar signScanner of
            Just '0' -> Right (advanceChar signScanner)
            Just ch
                | ch >= '1' && ch <= '9' ->
                    Right (advanceWhile isDigitChar signScanner)
            _ ->
                Left
                    LexerError
                        { lexerErrorMessage = if seenMinus then "expected digits after '-'" else "invalid JSON number"
                        , lexerErrorLine = startLine
                        , lexerErrorColumn = startColumn
                        }
    fractionScanner <-
        case currentChar integerScanner of
            Just '.' ->
                let afterDot = advanceChar integerScanner
                 in case currentChar afterDot of
                        Just digit
                            | isDigitChar digit -> Right (advanceWhile isDigitChar afterDot)
                        _ ->
                            Left
                                LexerError
                                    { lexerErrorMessage = "invalid JSON fraction"
                                    , lexerErrorLine = scannerLine integerScanner
                                    , lexerErrorColumn = scannerColumn integerScanner
                                    }
            _ -> Right integerScanner
    exponentScanner <-
        case currentChar fractionScanner of
            Just marker
                | marker == 'e' || marker == 'E' ->
                    let afterMarker = advanceChar fractionScanner
                        afterSign =
                            case currentChar afterMarker of
                                Just '+' -> advanceChar afterMarker
                                Just '-' -> advanceChar afterMarker
                                _ -> afterMarker
                     in case currentChar afterSign of
                            Just digit
                                | isDigitChar digit -> Right (advanceWhile isDigitChar afterSign)
                            _ ->
                                Left
                                    LexerError
                                        { lexerErrorMessage = "invalid JSON exponent"
                                        , lexerErrorLine = scannerLine afterMarker
                                        , lexerErrorColumn = scannerColumn afterMarker
                                        }
            _ -> Right fractionScanner
    let consumedLength = length (scannerRemaining scanner) - length (scannerRemaining exponentScanner)
        numberText = take consumedLength (scannerRemaining scanner)
    Right (makeToken TokenNumber numberText startLine startColumn, exponentScanner)

advanceWhile :: (Char -> Bool) -> Scanner -> Scanner
advanceWhile predicate scanner =
    case currentChar scanner of
        Just ch
            | predicate ch -> advanceWhile predicate (advanceChar scanner)
        _ -> scanner

advanceChars :: Int -> Scanner -> Scanner
advanceChars count scanner
    | count <= 0 = scanner
    | otherwise = advanceChars (count - 1) (advanceChar scanner)

advanceChar :: Scanner -> Scanner
advanceChar scanner =
    case scannerRemaining scanner of
        [] -> scanner
        ch : rest ->
            scanner
                { scannerRemaining = rest
                , scannerLine = if ch == '\n' then scannerLine scanner + 1 else scannerLine scanner
                , scannerColumn = if ch == '\n' then 1 else scannerColumn scanner + 1
                }

currentChar :: Scanner -> Maybe Char
currentChar scanner =
    case scannerRemaining scanner of
        [] -> Nothing
        ch : _ -> Just ch

isAtEnd :: Scanner -> Bool
isAtEnd scanner = null (scannerRemaining scanner)

isDigitChar :: Char -> Bool
isDigitChar ch = ch >= '0' && ch <= '9'

isHexDigit :: Char -> Bool
isHexDigit ch = isDigit ch || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
