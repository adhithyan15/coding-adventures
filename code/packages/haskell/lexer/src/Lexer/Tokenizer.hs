module Lexer.Tokenizer
    ( LexerConfig(..)
    , defaultLexerConfig
    , tokenize
    ) where

import qualified Data.Set as Set
import Data.Set (Set)
import Lexer.Token
import Lexer.TokenizerDFA
import StateMachine.DFA

data LexerConfig = LexerConfig
    { lexerKeywords :: [String]
    }
    deriving (Eq, Show)

defaultLexerConfig :: LexerConfig
defaultLexerConfig = LexerConfig []

data Scanner = Scanner
    { scannerRemaining :: String
    , scannerLine :: Int
    , scannerColumn :: Int
    , scannerPrecededByNewline :: Bool
    }
    deriving (Eq, Show)

tokenize :: LexerConfig -> String -> Either LexerError [Token]
tokenize config source = do
    dispatchDFA <- toLexerError 1 1 newTokenizerDFA
    scanTokens dispatchDFA keywordSet initialScanner []
  where
    keywordSet = Set.fromList (lexerKeywords config)
    initialScanner =
        Scanner
            { scannerRemaining = source
            , scannerLine = 1
            , scannerColumn = 1
            , scannerPrecededByNewline = False
            }

scanTokens :: DFA -> Set String -> Scanner -> [Token] -> Either LexerError [Token]
scanTokens dispatchDFA keywordSet scanner tokens = do
    nextStateMachine <- toLexerError (scannerLine scanner) (scannerColumn scanner) (processDFA (classifyChar (currentChar scanner)) dispatchDFA)
    case dfaCurrentState nextStateMachine of
        "at_whitespace" ->
            scanTokens dispatchDFA keywordSet (skipHorizontalWhitespace scanner) tokens
        "at_newline" ->
            let startLine = scannerLine scanner
                startColumn = scannerColumn scanner
                scannerAfterNewline = markPrecededByNewline (snd (advanceChar scanner))
             in scanTokens dispatchDFA keywordSet scannerAfterNewline (tokens ++ [makeToken TokenNewline "\\n" startLine startColumn])
        "in_number" -> do
            let (token, nextScanner) = readNumber scanner
            scanTokens dispatchDFA keywordSet nextScanner (tokens ++ [attachPendingFlags scanner token])
        "in_name" -> do
            let (token, nextScanner) = readName keywordSet scanner
            scanTokens dispatchDFA keywordSet nextScanner (tokens ++ [attachPendingFlags scanner token])
        "in_string" -> do
            (token, nextScanner) <- readStringLiteral scanner
            scanTokens dispatchDFA keywordSet nextScanner (tokens ++ [attachPendingFlags scanner token])
        "in_equals" ->
            let (token, nextScanner) = readEquals scanner
             in scanTokens dispatchDFA keywordSet nextScanner (tokens ++ [attachPendingFlags scanner token])
        "in_operator" ->
            case readSimpleOperator scanner of
                Left err -> Left err
                Right (token, nextScanner) ->
                    scanTokens dispatchDFA keywordSet nextScanner (tokens ++ [attachPendingFlags scanner token])
        "done" ->
            let eofToken =
                    makeToken TokenEof "" (scannerLine scanner) (scannerColumn scanner)
             in Right (tokens ++ [eofToken])
        _ ->
            Left
                LexerError
                    { lexerErrorMessage = "unexpected character " ++ show (currentChar scanner)
                    , lexerErrorLine = scannerLine scanner
                    , lexerErrorColumn = scannerColumn scanner
                    }

currentChar :: Scanner -> Maybe Char
currentChar scanner =
    case scannerRemaining scanner of
        [] -> Nothing
        ch : _ -> Just ch

peekChar :: Scanner -> Maybe Char
peekChar scanner =
    case scannerRemaining scanner of
        _ : ch : _ -> Just ch
        _ -> Nothing

advanceChar :: Scanner -> (Maybe Char, Scanner)
advanceChar scanner =
    case scannerRemaining scanner of
        [] -> (Nothing, scanner)
        ch : rest ->
            ( Just ch
            , scanner
                { scannerRemaining = rest
                , scannerLine = if ch == '\n' then scannerLine scanner + 1 else scannerLine scanner
                , scannerColumn = if ch == '\n' then 1 else scannerColumn scanner + 1
                }
            )

skipHorizontalWhitespace :: Scanner -> Scanner
skipHorizontalWhitespace scanner =
    case currentChar scanner of
        Just ch
            | ch == ' ' || ch == '\t' || ch == '\r' ->
                skipHorizontalWhitespace (snd (advanceChar scanner))
        _ -> scanner

readNumber :: Scanner -> (Token, Scanner)
readNumber scanner = go "" scanner
  where
    startLine = scannerLine scanner
    startColumn = scannerColumn scanner
    go acc currentScanner =
        case currentChar currentScanner of
            Just ch | ch >= '0' && ch <= '9' ->
                go (acc ++ [ch]) (snd (advanceChar currentScanner))
            _ -> (makeToken TokenNumber acc startLine startColumn, clearPendingFlags currentScanner)

readName :: Set String -> Scanner -> (Token, Scanner)
readName keywordSet scanner = go "" scanner
  where
    startLine = scannerLine scanner
    startColumn = scannerColumn scanner
    go acc currentScanner =
        case currentChar currentScanner of
            Just ch | isNameChar ch ->
                go (acc ++ [ch]) (snd (advanceChar currentScanner))
            _ ->
                let tokenTypeValue =
                        if acc `Set.member` keywordSet then TokenKeyword else TokenName
                 in (makeToken tokenTypeValue acc startLine startColumn, clearPendingFlags currentScanner)

readStringLiteral :: Scanner -> Either LexerError (Token, Scanner)
readStringLiteral scanner =
    case advanceChar scanner of
        (Nothing, _) ->
            Left
                LexerError
                    { lexerErrorMessage = "unterminated string literal"
                    , lexerErrorLine = scannerLine scanner
                    , lexerErrorColumn = scannerColumn scanner
                    }
        (Just _, scannerAfterQuote) -> go "" scannerAfterQuote
  where
    startLine = scannerLine scanner
    startColumn = scannerColumn scanner
    go acc currentScanner =
        case currentChar currentScanner of
            Nothing ->
                Left
                    LexerError
                        { lexerErrorMessage = "unterminated string literal"
                        , lexerErrorLine = startLine
                        , lexerErrorColumn = startColumn
                        }
            Just '"' ->
                let (_, scannerAfterClose) = advanceChar currentScanner
                 in Right (makeToken TokenString acc startLine startColumn, clearPendingFlags scannerAfterClose)
            Just '\\' ->
                case advanceChar currentScanner of
                    (_, scannerAfterSlash) ->
                        case currentChar scannerAfterSlash of
                            Nothing ->
                                Left
                                    LexerError
                                        { lexerErrorMessage = "unterminated string escape"
                                        , lexerErrorLine = startLine
                                        , lexerErrorColumn = startColumn
                                        }
                            Just escaped ->
                                let translated =
                                        case escaped of
                                            'n' -> '\n'
                                            't' -> '\t'
                                            '\\' -> '\\'
                                            '"' -> '"'
                                            other -> other
                                    (_, scannerAfterEscaped) = advanceChar scannerAfterSlash
                                 in go (acc ++ [translated]) scannerAfterEscaped
            Just ch ->
                go (acc ++ [ch]) (snd (advanceChar currentScanner))

readEquals :: Scanner -> (Token, Scanner)
readEquals scanner =
    let startLine = scannerLine scanner
        startColumn = scannerColumn scanner
        (_, scannerAfterEquals) = advanceChar scanner
     in case currentChar scannerAfterEquals of
            Just '=' ->
                ( makeToken TokenEqualsEquals "==" startLine startColumn
                , clearPendingFlags (snd (advanceChar scannerAfterEquals))
                )
            _ ->
                ( makeToken TokenEquals "=" startLine startColumn
                , clearPendingFlags scannerAfterEquals
                )

readSimpleOperator :: Scanner -> Either LexerError (Token, Scanner)
readSimpleOperator scanner =
    case currentChar scanner of
        Nothing ->
            Left
                LexerError
                    { lexerErrorMessage = "unexpected operator dispatch"
                    , lexerErrorLine = scannerLine scanner
                    , lexerErrorColumn = scannerColumn scanner
                    }
        Just ch ->
            case simpleTokenType ch of
                Nothing ->
                    Left
                        LexerError
                            { lexerErrorMessage = "unexpected operator dispatch"
                            , lexerErrorLine = scannerLine scanner
                            , lexerErrorColumn = scannerColumn scanner
                            }
                Just tokenTypeValue ->
                    let startLine = scannerLine scanner
                        startColumn = scannerColumn scanner
                     in Right
                            ( makeToken tokenTypeValue [ch] startLine startColumn
                            , clearPendingFlags (snd (advanceChar scanner))
                            )

attachPendingFlags :: Scanner -> Token -> Token
attachPendingFlags scanner token
    | scannerPrecededByNewline scanner = withFlags tokenPrecededByNewline token
    | otherwise = token

clearPendingFlags :: Scanner -> Scanner
clearPendingFlags scanner = scanner {scannerPrecededByNewline = False}

markPrecededByNewline :: Scanner -> Scanner
markPrecededByNewline scanner = scanner {scannerPrecededByNewline = True}

isNameChar :: Char -> Bool
isNameChar ch =
    (ch >= 'a' && ch <= 'z')
        || (ch >= 'A' && ch <= 'Z')
        || (ch >= '0' && ch <= '9')
        || ch == '_'

toLexerError :: Int -> Int -> Either String a -> Either LexerError a
toLexerError line column result =
    case result of
        Left message ->
            Left
                LexerError
                    { lexerErrorMessage = message
                    , lexerErrorLine = line
                    , lexerErrorColumn = column
                    }
        Right value -> Right value
