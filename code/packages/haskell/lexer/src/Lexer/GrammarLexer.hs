module Lexer.GrammarLexer
    ( tokenizeWithGrammar
    ) where

import Data.Array ((!))
import Data.Char (chr, isDigit, toLower)
import qualified Data.Map.Strict as Map
import Data.Maybe (fromMaybe)
import qualified Data.Set as Set
import GrammarTools.TokenGrammar
import Lexer.Token
import Numeric (readHex)
import Text.Regex.Base.RegexLike (matchOnceText)
import Text.Regex.TDFA (Regex, defaultCompOpt, defaultExecOpt, makeRegexOpts)

data CompiledPattern = CompiledPattern
    { compiledPatternName :: String
    , compiledPatternRegex :: Regex
    , compiledPatternAlias :: Maybe String
    }

data GrammarLexerState = GrammarLexerState
    { stateOriginalSource :: String
    , stateWorkingSource :: String
    , statePosition :: Int
    , stateLine :: Int
    , stateColumn :: Int
    , stateIndentStack :: [Int]
    , stateBracketDepth :: Int
    }
    deriving (Eq, Show)

data GrammarContext = GrammarContext
    { contextGrammar :: TokenGrammar
    , contextCaseInsensitive :: Bool
    , contextKeywordMap :: Map.Map String String
    , contextReservedKeywords :: Set.Set String
    , contextContextKeywords :: Set.Set String
    , contextSoftKeywords :: Set.Set String
    , contextTokenPatterns :: [CompiledPattern]
    , contextSkipPatterns :: [CompiledPattern]
    }

tokenizeWithGrammar :: TokenGrammar -> String -> Either LexerError [Token]
tokenizeWithGrammar grammar source = do
    context <- compileContext grammar
    let initialState =
            GrammarLexerState
                { stateOriginalSource = source
                , stateWorkingSource = if contextCaseInsensitive context then map toLower source else source
                , statePosition = 0
                , stateLine = 1
                , stateColumn = 1
                , stateIndentStack = [0]
                , stateBracketDepth = 0
                }
    if tokenGrammarMode grammar == Just "indentation"
        then tokenizeIndentation context initialState []
        else tokenizeStandard context initialState []

compileContext :: TokenGrammar -> Either LexerError GrammarContext
compileContext grammar = do
    tokenPatterns <- mapM (compilePattern grammar) (tokenGrammarDefinitions grammar)
    skipPatterns <- mapM (compilePattern grammar) (tokenGrammarSkipDefinitions grammar)
    let caseInsensitive = tokenGrammarCaseInsensitive grammar || not (tokenGrammarCaseSensitive grammar)
        keywordMap =
            Map.fromList
                [ (normalizeCase caseInsensitive keyword, keyword)
                | keyword <- tokenGrammarKeywords grammar
                ]
        reservedKeywords =
            Set.fromList
                [ normalizeCase caseInsensitive keyword
                | keyword <- tokenGrammarReservedKeywords grammar
                ]
        contextKeywords =
            Set.fromList
                [ normalizeCase caseInsensitive keyword
                | keyword <- tokenGrammarContextKeywords grammar
                ]
        softKeywords =
            Set.fromList
                [ normalizeCase caseInsensitive keyword
                | keyword <- tokenGrammarSoftKeywords grammar
                ]
    Right
        GrammarContext
            { contextGrammar = grammar
            , contextCaseInsensitive = caseInsensitive
            , contextKeywordMap = keywordMap
            , contextReservedKeywords = reservedKeywords
            , contextContextKeywords = contextKeywords
            , contextSoftKeywords = softKeywords
            , contextTokenPatterns = tokenPatterns
            , contextSkipPatterns = skipPatterns
            }

compilePattern :: TokenGrammar -> TokenDefinition -> Either LexerError CompiledPattern
compilePattern grammar definition =
    Right
        CompiledPattern
            { compiledPatternName = tokenDefinitionName definition
            , compiledPatternRegex =
                makeRegexOpts defaultCompOpt defaultExecOpt (compilePatternSource grammar definition)
            , compiledPatternAlias = tokenDefinitionAlias definition
            }

compilePatternSource :: TokenGrammar -> TokenDefinition -> String
compilePatternSource grammar definition =
    "^" ++ withCaseModifier (effectivePattern definition)
  where
    withCaseModifier patternSource
        | tokenGrammarCaseInsensitive grammar || not (tokenGrammarCaseSensitive grammar) =
            "(?i:" ++ patternSource ++ ")"
        | otherwise = patternSource

effectivePattern :: TokenDefinition -> String
effectivePattern definition
    | tokenDefinitionIsRegex definition = normalizeRegexEscapes (tokenDefinitionPattern definition)
    | otherwise = escapeRegexLiteral (tokenDefinitionPattern definition)

normalizeRegexEscapes :: String -> String
normalizeRegexEscapes [] = []
normalizeRegexEscapes ['\\'] = ['\\']
normalizeRegexEscapes ('\\' : 't' : rest) = '\t' : normalizeRegexEscapes rest
normalizeRegexEscapes ('\\' : 'r' : rest) = '\r' : normalizeRegexEscapes rest
normalizeRegexEscapes ('\\' : 'n' : rest) = '\n' : normalizeRegexEscapes rest
normalizeRegexEscapes ('\\' : 'b' : rest) = '\b' : normalizeRegexEscapes rest
normalizeRegexEscapes ('\\' : 'f' : rest) = '\f' : normalizeRegexEscapes rest
normalizeRegexEscapes ('\\' : 'x' : a : b : rest)
    | all isHexDigit [a, b] =
        case readHex [a, b] of
            [(value, "")] -> chr value : normalizeRegexEscapes rest
            _ -> '\\' : 'x' : a : b : normalizeRegexEscapes rest
normalizeRegexEscapes ('\\' : ch : rest) = '\\' : ch : normalizeRegexEscapes rest
normalizeRegexEscapes (ch : rest) = ch : normalizeRegexEscapes rest

escapeRegexLiteral :: String -> String
escapeRegexLiteral = concatMap escapeChar
  where
    escapeChar ch
        | ch `elem` regexSpecials = ['\\', ch]
        | otherwise = [ch]
    regexSpecials = "\\.^$|?*+()[]{}"

tokenizeStandard :: GrammarContext -> GrammarLexerState -> [Token] -> Either LexerError [Token]
tokenizeStandard context state tokens
    | atEnd state =
        Right
            ( tokens
                ++ [ makeToken TokenEof "" (stateLine state) (stateColumn state)
                   ]
            )
    | otherwise =
        case tryConsumeSkip context state of
            Just nextState ->
                tokenizeStandard context nextState tokens
            Nothing ->
                case tryMatchToken context state of
                    Right (Just (token, nextState)) ->
                        tokenizeStandard context nextState (tokens ++ [token])
                    Right Nothing ->
                        Left
                            LexerError
                                { lexerErrorMessage = "unexpected character " ++ show (currentChar state)
                                , lexerErrorLine = stateLine state
                                , lexerErrorColumn = stateColumn state
                                }
                    Left err -> Left err

tokenizeIndentation :: GrammarContext -> GrammarLexerState -> [Token] -> Either LexerError [Token]
tokenizeIndentation context initialState initialTokens =
    loop True initialState initialTokens
  where
    loop atLineStart state tokens
        | atEnd state =
            let line = stateLine state
                column = stateColumn state
                withTerminalNewline =
                    if null tokens || canonicalTokenName (last tokens) /= "NEWLINE"
                        then tokens ++ [withTypeName "NEWLINE" (makeToken TokenNewline "\\n" line column)]
                        else tokens
                dedentTokens =
                    replicate (max 0 (length (stateIndentStack state) - 1)) (withTypeName "DEDENT" (makeToken TokenDedent "" line 1))
             in Right (withTerminalNewline ++ dedentTokens ++ [makeToken TokenEof "" line column])
        | atLineStart && stateBracketDepth state == 0 = do
            (indentTokens, nextState, skipLine) <- processLineStart context state
            if skipLine
                then loop True nextState tokens
                else loop False nextState (tokens ++ indentTokens)
        | otherwise =
            case currentChar state of
                Just '\n' ->
                    let newlineToken = withTypeName "NEWLINE" (makeToken TokenNewline "\\n" (stateLine state) (stateColumn state))
                        nextState = advanceStateByString "\n" state
                        nextTokens =
                            if stateBracketDepth state == 0
                                then tokens ++ [newlineToken]
                                else tokens
                     in loop True nextState nextTokens
                Just ch
                    | stateBracketDepth state > 0 && (ch == ' ' || ch == '\t' || ch == '\r') ->
                        loop False (advanceStateByString [ch] state) tokens
                _ ->
                    case tryConsumeSkip context state of
                        Just nextState ->
                            loop False nextState tokens
                        Nothing ->
                            case tryMatchToken context state of
                                Right (Just (token, nextState)) ->
                                    loop False nextState (tokens ++ [token])
                                Right Nothing ->
                                    Left
                                        LexerError
                                            { lexerErrorMessage = "unexpected character " ++ show (currentChar state)
                                            , lexerErrorLine = stateLine state
                                            , lexerErrorColumn = stateColumn state
                                            }
                                Left err -> Left err

processLineStart ::
       GrammarContext
    -> GrammarLexerState
    -> Either LexerError ([Token], GrammarLexerState, Bool)
processLineStart context state = do
    let (indentCount, afterIndent, sawTab) = readIndentation state
    if sawTab
        then
            Left
                LexerError
                    { lexerErrorMessage = "tab character in indentation"
                    , lexerErrorLine = stateLine state
                    , lexerErrorColumn = stateColumn state
                    }
        else
            case currentChar afterIndent of
                Nothing -> Right ([], afterIndent, True)
                Just '\n' -> Right ([], advanceStateByString "\n" afterIndent, True)
                _ ->
                    case commentOnlyRemainder context afterIndent of
                        Just nextState -> Right ([], nextState, True)
                        Nothing ->
                            let currentIndent = last (stateIndentStack state)
                             in if indentCount > currentIndent
                                    then
                                        let nextState = afterIndent {stateIndentStack = stateIndentStack state ++ [indentCount]}
                                            indentToken = withTypeName "INDENT" (makeToken TokenIndent "" (stateLine afterIndent) 1)
                                         in Right ([indentToken], nextState, False)
                                    else
                                        if indentCount < currentIndent
                                            then
                                                emitDedents indentCount afterIndent
                                            else Right ([], afterIndent, False)

emitDedents ::
       Int
    -> GrammarLexerState
    -> Either LexerError ([Token], GrammarLexerState, Bool)
emitDedents targetIndent state =
    let popOne stack = take (length stack - 1) stack
        go stack acc
            | length stack <= 1 = (stack, acc)
            | last stack > targetIndent =
                go (popOne stack) (acc ++ [withTypeName "DEDENT" (makeToken TokenDedent "" (stateLine state) 1)])
            | otherwise = (stack, acc)
        (nextStack, tokens) = go (stateIndentStack state) []
     in if last nextStack /= targetIndent
            then
                Left
                    LexerError
                        { lexerErrorMessage = "inconsistent dedent"
                        , lexerErrorLine = stateLine state
                        , lexerErrorColumn = stateColumn state
                        }
            else Right (tokens, state {stateIndentStack = nextStack}, False)

readIndentation :: GrammarLexerState -> (Int, GrammarLexerState, Bool)
readIndentation state = go 0 state False
  where
    go count current sawTab =
        case currentChar current of
            Just ' ' -> go (count + 1) (advanceStateByString " " current) sawTab
            Just '\t' -> go count (advanceStateByString "\t" current) True
            _ -> (count, current, sawTab)

commentOnlyRemainder :: GrammarContext -> GrammarLexerState -> Maybe GrammarLexerState
commentOnlyRemainder context state =
    case firstSkipMatch context state of
        Just (_, matchText)
            | '\n' `notElem` matchText ->
                let afterComment = advanceStateByString matchText state
                 in case currentChar afterComment of
                        Just '\n' -> Just (advanceStateByString "\n" afterComment)
                        Nothing -> Just afterComment
                        _ -> Nothing
        _ -> Nothing

tryConsumeSkip :: GrammarContext -> GrammarLexerState -> Maybe GrammarLexerState
tryConsumeSkip context state =
    case firstSkipMatch context state of
        Just (_, matchText) -> Just (advanceStateByString matchText state)
        Nothing ->
            case currentChar state of
                Just ch
                    | null (contextSkipPatterns context) && (ch == ' ' || ch == '\t' || ch == '\r') ->
                        Just (advanceStateByString [ch] state)
                _ -> Nothing

firstSkipMatch :: GrammarContext -> GrammarLexerState -> Maybe (CompiledPattern, String)
firstSkipMatch context state =
    firstPatternMatch (contextSkipPatterns context) (remainingWorkingSource state)

tryMatchToken :: GrammarContext -> GrammarLexerState -> Either LexerError (Maybe (Token, GrammarLexerState))
tryMatchToken context state =
    case firstPatternMatch (contextTokenPatterns context) (remainingWorkingSource state) of
        Nothing -> Right Nothing
        Just (patternInfo, matchedText) ->
            let startLine = stateLine state
                startColumn = stateColumn state
                matchedLength = length matchedText
                originalSlice =
                    take matchedLength
                        (drop (statePosition state) (stateOriginalSource state))
             in do
                    (tokenTypeValue, typeName, renderedValue, flags) <-
                        resolveToken context patternInfo originalSlice startLine startColumn
                    let token =
                            withOptionalTypeName typeName $
                            withFlagsIf flags $
                            makeToken tokenTypeValue renderedValue startLine startColumn
                        nextState = updateBracketDepth renderedValue (advanceStateByString matchedText state)
                    Right (Just (token, nextState))

firstPatternMatch :: [CompiledPattern] -> String -> Maybe (CompiledPattern, String)
firstPatternMatch [] _ = Nothing
firstPatternMatch (patternInfo : rest) input =
    case matchPattern patternInfo input of
        Just matched -> Just (patternInfo, matched)
        Nothing -> firstPatternMatch rest input

matchPattern :: CompiledPattern -> String -> Maybe String
matchPattern patternInfo input =
    case matchOnceText (compiledPatternRegex patternInfo) input of
        Just (_, matchedText, _) -> Just (fst (matchedText ! 0))
        Nothing -> Nothing

resolveToken ::
       GrammarContext
    -> CompiledPattern
    -> String
    -> Int
    -> Int
    -> Either LexerError (TokenType, Maybe String, String, Int)
resolveToken context patternInfo originalValue line column =
    let effectiveName = fromMaybe (compiledPatternName patternInfo) (compiledPatternAlias patternInfo)
        normalizedValue = normalizeCase (contextCaseInsensitive context) originalValue
        keywordValue = Map.lookup normalizedValue (contextKeywordMap context)
        contextKeyword =
            effectiveName == "NAME"
                && Set.member normalizedValue (contextContextKeywords context)
        softKeyword =
            effectiveName == "NAME"
                && Set.member normalizedValue (contextSoftKeywords context)
        flags =
            if contextKeyword then tokenContextKeyword else 0
     in if effectiveName == "NAME" && Set.member normalizedValue (contextReservedKeywords context)
            then
                Left
                    LexerError
                        { lexerErrorMessage = "reserved keyword cannot be used as an identifier"
                        , lexerErrorLine = line
                        , lexerErrorColumn = column
                        }
            else
                case keywordValue of
                    Just canonicalKeyword ->
                        Right (TokenKeyword, Nothing, canonicalKeyword, flags)
                    Nothing ->
                        let (baseType, maybeTypeName) = tokenTypeForName effectiveName
                            renderedValue =
                                if isStringLike effectiveName
                                    then decodeStringValue (tokenGrammarEscapeMode (contextGrammar context)) originalValue
                                    else originalValue
                            typeNameOverride =
                                if softKeyword then Just effectiveName else maybeTypeName
                         in Right (baseType, typeNameOverride, renderedValue, flags)

tokenTypeForName :: String -> (TokenType, Maybe String)
tokenTypeForName tokenName =
    case tokenName of
        "NAME" -> (TokenName, Nothing)
        "NUMBER" -> (TokenNumber, Nothing)
        "STRING" -> (TokenString, Nothing)
        "PLUS" -> (TokenPlus, Nothing)
        "MINUS" -> (TokenMinus, Nothing)
        "STAR" -> (TokenStar, Nothing)
        "SLASH" -> (TokenSlash, Nothing)
        "EQUALS" -> (TokenEquals, Nothing)
        "EQUALS_EQUALS" -> (TokenEqualsEquals, Nothing)
        "LPAREN" -> (TokenLParen, Nothing)
        "RPAREN" -> (TokenRParen, Nothing)
        "COMMA" -> (TokenComma, Nothing)
        "COLON" -> (TokenColon, Nothing)
        "SEMICOLON" -> (TokenSemicolon, Nothing)
        "LBRACE" -> (TokenLBrace, Nothing)
        "RBRACE" -> (TokenRBrace, Nothing)
        "LBRACKET" -> (TokenLBracket, Nothing)
        "RBRACKET" -> (TokenRBracket, Nothing)
        "DOT" -> (TokenDot, Nothing)
        "BANG" -> (TokenBang, Nothing)
        "NEWLINE" -> (TokenNewline, Nothing)
        "INDENT" -> (TokenIndent, Nothing)
        "DEDENT" -> (TokenDedent, Nothing)
        "EOF" -> (TokenEof, Nothing)
        other -> (TokenName, Just other)

decodeStringValue :: Maybe String -> String -> String
decodeStringValue escapeMode rawValue =
    case stripQuotes rawValue of
        Nothing -> rawValue
        Just inner
            | escapeMode == Just "none" -> inner
            | otherwise -> processEscapes inner

stripQuotes :: String -> Maybe String
stripQuotes rawValue =
    case rawValue of
        quoteA : quoteB : quoteC : rest
            | quoteA == quoteB
                && quoteB == quoteC
                && quoteA `elem` ['"', '\'']
                && length rawValue >= 6
                && drop (length rawValue - 3) rawValue == replicate 3 quoteA ->
                    Just (take (length rawValue - 6) (drop 3 rawValue))
        quote : rest
            | length rawValue >= 2
                && quote `elem` ['"', '\'']
                && last rawValue == quote ->
                    Just (take (length rawValue - 2) rest)
        _ -> Nothing

processEscapes :: String -> String
processEscapes [] = []
processEscapes ['\\'] = ['\\']
processEscapes ('\\' : 'n' : rest) = '\n' : processEscapes rest
processEscapes ('\\' : 'r' : rest) = '\r' : processEscapes rest
processEscapes ('\\' : 't' : rest) = '\t' : processEscapes rest
processEscapes ('\\' : 'b' : rest) = '\b' : processEscapes rest
processEscapes ('\\' : 'f' : rest) = '\f' : processEscapes rest
processEscapes ('\\' : '\\' : rest) = '\\' : processEscapes rest
processEscapes ('\\' : '"' : rest) = '"' : processEscapes rest
processEscapes ('\\' : '\'' : rest) = '\'' : processEscapes rest
processEscapes ('\\' : '/' : rest) = '/' : processEscapes rest
processEscapes ('\\' : 'u' : a : b : c : d : rest)
    | all isHexDigit [a, b, c, d] =
        case readHex [a, b, c, d] of
            [(value, "")] -> chr value : processEscapes rest
            _ -> '\\' : 'u' : a : b : c : d : processEscapes rest
processEscapes ('\\' : ch : rest) = ch : processEscapes rest
processEscapes (ch : rest) = ch : processEscapes rest

isStringLike :: String -> Bool
isStringLike tokenName = "STRING" `contains` tokenName

contains :: String -> String -> Bool
contains needle haystack =
    any (needle `prefixOf`) (suffixes haystack)

prefixOf :: String -> String -> Bool
prefixOf [] _ = True
prefixOf _ [] = False
prefixOf (left : leftRest) (right : rightRest) =
    left == right && prefixOf leftRest rightRest

suffixes :: String -> [String]
suffixes [] = [[]]
suffixes value@(_ : rest) = value : suffixes rest

isHexDigit :: Char -> Bool
isHexDigit ch = isDigit ch || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')

remainingWorkingSource :: GrammarLexerState -> String
remainingWorkingSource state = drop (statePosition state) (stateWorkingSource state)

currentChar :: GrammarLexerState -> Maybe Char
currentChar state =
    case remainingWorkingSource state of
        [] -> Nothing
        ch : _ -> Just ch

advanceStateByString :: String -> GrammarLexerState -> GrammarLexerState
advanceStateByString consumed state = foldl advanceStateChar state consumed

advanceStateChar :: GrammarLexerState -> Char -> GrammarLexerState
advanceStateChar state ch =
    state
        { statePosition = statePosition state + 1
        , stateLine = if ch == '\n' then stateLine state + 1 else stateLine state
        , stateColumn = if ch == '\n' then 1 else stateColumn state + 1
        }

updateBracketDepth :: String -> GrammarLexerState -> GrammarLexerState
updateBracketDepth renderedValue state =
    case renderedValue of
        "(" -> state {stateBracketDepth = stateBracketDepth state + 1}
        "[" -> state {stateBracketDepth = stateBracketDepth state + 1}
        "{" -> state {stateBracketDepth = stateBracketDepth state + 1}
        ")" -> state {stateBracketDepth = max 0 (stateBracketDepth state - 1)}
        "]" -> state {stateBracketDepth = max 0 (stateBracketDepth state - 1)}
        "}" -> state {stateBracketDepth = max 0 (stateBracketDepth state - 1)}
        _ -> state

atEnd :: GrammarLexerState -> Bool
atEnd state = statePosition state >= length (stateWorkingSource state)

normalizeCase :: Bool -> String -> String
normalizeCase shouldNormalize value
    | shouldNormalize = map toLower value
    | otherwise = value

withOptionalTypeName :: Maybe String -> Token -> Token
withOptionalTypeName maybeTypeName token =
    case maybeTypeName of
        Nothing -> token
        Just typeName -> withTypeName typeName token

withFlagsIf :: Int -> Token -> Token
withFlagsIf flags token
    | flags == 0 = token
    | otherwise = withFlags flags token
