module GrammarTools.ParserGrammar
    ( ParserGrammarError(..)
    , GrammarElement(..)
    , GrammarRule(..)
    , ParserGrammar(..)
    , parseParserGrammar
    , validateParserGrammar
    , ruleNames
    , tokenReferences
    , ruleReferences
    ) where

import Data.Char (isAlpha, isAlphaNum, isLower, isUpper, toUpper)
import Data.List (intercalate, isPrefixOf, sort)
import qualified Data.Set as Set
import Data.Set (Set)
import Text.Read (readMaybe)

data ParserGrammarError = ParserGrammarError
    { parserGrammarErrorMessage :: String
    , parserGrammarErrorLineNumber :: Int
    }
    deriving (Eq)

instance Show ParserGrammarError where
    show err =
        "Line "
            ++ show (parserGrammarErrorLineNumber err)
            ++ ": "
            ++ parserGrammarErrorMessage err

data GrammarElement
    = RuleReference
        { ruleReferenceName :: String
        , ruleReferenceIsToken :: Bool
        }
    | Literal
        { literalValue :: String
        }
    | Sequence
        { sequenceElements :: [GrammarElement]
        }
    | Alternation
        { alternationChoices :: [GrammarElement]
        }
    | Repetition
        { repetitionElement :: GrammarElement
        }
    | Optional
        { optionalElement :: GrammarElement
        }
    | Group
        { groupElement :: GrammarElement
        }
    | PositiveLookahead
        { positiveLookaheadElement :: GrammarElement
        }
    | NegativeLookahead
        { negativeLookaheadElement :: GrammarElement
        }
    | OneOrMoreRepetition
        { oneOrMoreElement :: GrammarElement
        }
    | SeparatedRepetition
        { separatedElement :: GrammarElement
        , separatedSeparator :: GrammarElement
        , separatedAtLeastOne :: Bool
        }
    deriving (Eq, Show)

data GrammarRule = GrammarRule
    { grammarRuleName :: String
    , grammarRuleBody :: GrammarElement
    , grammarRuleLineNumber :: Int
    }
    deriving (Eq, Show)

data ParserGrammar = ParserGrammar
    { parserGrammarVersion :: Int
    , parserGrammarRules :: [GrammarRule]
    }
    deriving (Eq, Show)

data TokenKind
    = TkIdent
    | TkString
    | TkEquals
    | TkSemi
    | TkPipe
    | TkLBrace
    | TkRBrace
    | TkLBracket
    | TkRBracket
    | TkLParen
    | TkRParen
    | TkAmpersand
    | TkBang
    | TkPlus
    | TkDoubleSlash
    | TkEof
    deriving (Eq, Show)

data InternalToken = InternalToken
    { internalTokenKind :: TokenKind
    , internalTokenValue :: String
    , internalTokenLine :: Int
    }
    deriving (Eq, Show)

data ParserState = ParserState
    { parserStateTokens :: [InternalToken]
    , parserStatePosition :: Int
    }
    deriving (Eq, Show)

parseParserGrammar :: String -> Either ParserGrammarError ParserGrammar
parseParserGrammar source = do
    let grammarVersion = extractVersion source
    tokens <- tokenizeGrammar source
    rules <- parseRules (ParserState tokens 0)
    Right (ParserGrammar grammarVersion rules)

validateParserGrammar :: ParserGrammar -> Maybe (Set String) -> [String]
validateParserGrammar grammar maybeTokenNames =
    duplicateIssues ++ lowercaseIssues ++ undefinedRuleIssues ++ undefinedTokenIssues ++ unreachableIssues
  where
    definedRules = ruleNames grammar
    referencedRuleNames = ruleReferences grammar
    referencedTokenNames = tokenReferences grammar
    duplicateIssues =
        concatMap duplicateIssue (parserGrammarRules grammar)
      where
        firstDefinitions =
            foldl
                (\acc rule ->
                    if Set.member (grammarRuleName rule) (Set.fromList (map fst acc))
                        then acc
                        else acc ++ [(grammarRuleName rule, grammarRuleLineNumber rule)]
                )
                []
                (parserGrammarRules grammar)
        duplicateIssue rule =
            case lookup (grammarRuleName rule) firstDefinitions of
                Just firstLine
                    | firstLine /= grammarRuleLineNumber rule ->
                        [ "Line "
                            ++ show (grammarRuleLineNumber rule)
                            ++ ": Duplicate rule name '"
                            ++ grammarRuleName rule
                            ++ "' (first defined on line "
                            ++ show firstLine
                            ++ ")"
                        ]
                _ -> []
    lowercaseIssues =
        [ "Line "
            ++ show (grammarRuleLineNumber rule)
            ++ ": Rule name '"
            ++ grammarRuleName rule
            ++ "' should be lowercase"
        | rule <- parserGrammarRules grammar
        , grammarRuleName rule /= toLowerString (grammarRuleName rule)
        ]
    undefinedRuleIssues =
        [ "Undefined rule reference: '" ++ referenceName ++ "'"
        | referenceName <- sort (Set.toList referencedRuleNames)
        , referenceName `Set.notMember` definedRules
        ]
    syntheticTokens = Set.fromList ["NEWLINE", "INDENT", "DEDENT", "EOF"]
    undefinedTokenIssues =
        case maybeTokenNames of
            Nothing -> []
            Just validTokens ->
                [ "Undefined token reference: '" ++ referenceName ++ "'"
                | referenceName <- sort (Set.toList referencedTokenNames)
                , referenceName `Set.notMember` validTokens
                , referenceName `Set.notMember` syntheticTokens
                ]
    unreachableIssues =
        case parserGrammarRules grammar of
            [] -> []
            startRule : _ ->
                [ "Line "
                    ++ show (grammarRuleLineNumber rule)
                    ++ ": Rule '"
                    ++ grammarRuleName rule
                    ++ "' is defined but never referenced (unreachable)"
                | rule <- parserGrammarRules grammar
                , grammarRuleName rule /= grammarRuleName startRule
                , grammarRuleName rule `Set.notMember` referencedRuleNames
                ]

ruleNames :: ParserGrammar -> Set String
ruleNames grammar =
    Set.fromList [grammarRuleName rule | rule <- parserGrammarRules grammar]

tokenReferences :: ParserGrammar -> Set String
tokenReferences grammar =
    Set.unions [collectTokenReferences (grammarRuleBody rule) | rule <- parserGrammarRules grammar]

ruleReferences :: ParserGrammar -> Set String
ruleReferences grammar =
    Set.unions [collectRuleReferences (grammarRuleBody rule) | rule <- parserGrammarRules grammar]

collectTokenReferences :: GrammarElement -> Set String
collectTokenReferences element =
    case element of
        RuleReference name True -> Set.singleton name
        RuleReference _ False -> Set.empty
        Literal _ -> Set.empty
        Sequence elements -> Set.unions (map collectTokenReferences elements)
        Alternation choices -> Set.unions (map collectTokenReferences choices)
        Repetition child -> collectTokenReferences child
        Optional child -> collectTokenReferences child
        Group child -> collectTokenReferences child
        PositiveLookahead child -> collectTokenReferences child
        NegativeLookahead child -> collectTokenReferences child
        OneOrMoreRepetition child -> collectTokenReferences child
        SeparatedRepetition child separator _ ->
            Set.union (collectTokenReferences child) (collectTokenReferences separator)

collectRuleReferences :: GrammarElement -> Set String
collectRuleReferences element =
    case element of
        RuleReference name False -> Set.singleton name
        RuleReference _ True -> Set.empty
        Literal _ -> Set.empty
        Sequence elements -> Set.unions (map collectRuleReferences elements)
        Alternation choices -> Set.unions (map collectRuleReferences choices)
        Repetition child -> collectRuleReferences child
        Optional child -> collectRuleReferences child
        Group child -> collectRuleReferences child
        PositiveLookahead child -> collectRuleReferences child
        NegativeLookahead child -> collectRuleReferences child
        OneOrMoreRepetition child -> collectRuleReferences child
        SeparatedRepetition child separator _ ->
            Set.union (collectRuleReferences child) (collectRuleReferences separator)

extractVersion :: String -> Int
extractVersion source =
    foldl extract 0 (lines source)
  where
    extract currentVersion rawLine =
        let stripped = trim rawLine
         in case words (drop 1 stripped) of
                ("@version" : rawValue : _) | "#" `isPrefixOf` stripped ->
                    case readMaybe rawValue of
                        Just versionValue -> versionValue
                        Nothing -> currentVersion
                _ -> currentVersion

tokenizeGrammar :: String -> Either ParserGrammarError [InternalToken]
tokenizeGrammar source =
    fmap (++ [InternalToken TkEof "" eofLine]) (concat <$> mapM tokenizeLine numberedLines)
  where
    sourceLines = lines source
    eofLine = max 1 (length sourceLines)
    numberedLines = zip [1 ..] sourceLines

tokenizeLine :: (Int, String) -> Either ParserGrammarError [InternalToken]
tokenizeLine (lineNumber, rawLine)
    | null stripped = Right []
    | "#" `isPrefixOf` stripped = Right []
    | otherwise = go rawLine
  where
    stripped = trim rawLine
    go [] = Right []
    go (ch : rest)
        | ch == ' ' || ch == '\t' = go rest
        | ch == '#' = Right []
        | ch == '=' = prependToken TkEquals "=" rest
        | ch == ';' = prependToken TkSemi ";" rest
        | ch == '|' = prependToken TkPipe "|" rest
        | ch == '{' = prependToken TkLBrace "{" rest
        | ch == '}' = prependToken TkRBrace "}" rest
        | ch == '[' = prependToken TkLBracket "[" rest
        | ch == ']' = prependToken TkRBracket "]" rest
        | ch == '(' = prependToken TkLParen "(" rest
        | ch == ')' = prependToken TkRParen ")" rest
        | ch == '&' = prependToken TkAmpersand "&" rest
        | ch == '!' = prependToken TkBang "!" rest
        | ch == '+' = prependToken TkPlus "+" rest
        | ch == '/' =
            case rest of
                '/' : remaining -> prependToken TkDoubleSlash "//" remaining
                _ -> Left (ParserGrammarError ("Unexpected character: " ++ show ch) lineNumber)
        | ch == '"' =
            case readQuotedString rest [] of
                Left message -> Left (ParserGrammarError message lineNumber)
                Right (stringValue, remaining) ->
                    prependExistingToken (InternalToken TkString stringValue lineNumber) remaining
        | isAlpha ch || ch == '_' =
            let (identifier, remaining) = span (\c -> isAlphaNum c || c == '_') (ch : rest)
             in prependExistingToken (InternalToken TkIdent identifier lineNumber) remaining
        | otherwise =
            Left (ParserGrammarError ("Unexpected character: " ++ show ch) lineNumber)

    prependToken kind value remaining =
        prependExistingToken (InternalToken kind value lineNumber) remaining

    prependExistingToken token remaining = do
        more <- go remaining
        Right (token : more)

readQuotedString :: String -> String -> Either String (String, String)
readQuotedString [] _ = Left "Unterminated string literal"
readQuotedString (ch : rest) acc
    | ch == '"' = Right (reverse acc, rest)
    | ch == '\\' =
        case rest of
            [] -> Left "Unterminated string literal"
            escaped : remaining -> readQuotedString remaining (escaped : '\\' : acc)
    | otherwise =
        readQuotedString rest (ch : acc)

parseRules :: ParserState -> Either ParserGrammarError [GrammarRule]
parseRules state
    | internalTokenKind (peekToken state) == TkEof = Right []
    | otherwise = do
        (rule, nextState) <- parseRule state
        moreRules <- parseRules nextState
        Right (rule : moreRules)

parseRule :: ParserState -> Either ParserGrammarError (GrammarRule, ParserState)
parseRule state = do
    (nameToken, stateAfterName) <- expectToken TkIdent state
    (_, stateAfterEquals) <- expectToken TkEquals stateAfterName
    (body, stateAfterBody) <- parseBody stateAfterEquals
    (_, stateAfterSemi) <- expectToken TkSemi stateAfterBody
    Right
        ( GrammarRule
            { grammarRuleName = internalTokenValue nameToken
            , grammarRuleBody = body
            , grammarRuleLineNumber = internalTokenLine nameToken
            }
        , stateAfterSemi
        )

parseBody :: ParserState -> Either ParserGrammarError (GrammarElement, ParserState)
parseBody state = do
    (firstSequence, stateAfterFirst) <- parseSequence state
    collectAlternatives [firstSequence] stateAfterFirst
  where
    collectAlternatives alternatives currentState
        | internalTokenKind (peekToken currentState) == TkPipe = do
            (_, stateAfterPipe) <- expectToken TkPipe currentState
            (nextSequence, stateAfterNext) <- parseSequence stateAfterPipe
            collectAlternatives (alternatives ++ [nextSequence]) stateAfterNext
        | length alternatives == 1 =
            case alternatives of
                [single] -> Right (single, currentState)
                _ ->
                    Left
                        ( ParserGrammarError
                            "Expected at least one alternative"
                            (internalTokenLine (peekToken currentState))
                        )
        | otherwise =
            Right (Alternation alternatives, currentState)

parseSequence :: ParserState -> Either ParserGrammarError (GrammarElement, ParserState)
parseSequence state =
    collectElements [] state
  where
    collectElements elements currentState
        | internalTokenKind (peekToken currentState) `elem` [TkPipe, TkSemi, TkRBrace, TkRBracket, TkRParen, TkDoubleSlash, TkEof] =
            case elements of
                [] ->
                    Left
                        ( ParserGrammarError
                            "Expected at least one element in sequence"
                            (internalTokenLine (peekToken currentState))
                        )
                [single] -> Right (single, currentState)
                _ -> Right (Sequence elements, currentState)
        | otherwise = do
            (element, nextState) <- parseElement currentState
            collectElements (elements ++ [element]) nextState

parseElement :: ParserState -> Either ParserGrammarError (GrammarElement, ParserState)
parseElement state =
    case internalTokenKind (peekToken state) of
        TkAmpersand -> do
            (_, afterAmpersand) <- expectToken TkAmpersand state
            (child, afterChild) <- parseElement afterAmpersand
            Right (PositiveLookahead child, afterChild)
        TkBang -> do
            (_, afterBang) <- expectToken TkBang state
            (child, afterChild) <- parseElement afterBang
            Right (NegativeLookahead child, afterChild)
        TkIdent ->
            let (token, nextState) = advanceToken state
                name = internalTokenValue token
                isTokenReference = isUpperTokenReference name
             in Right (RuleReference name isTokenReference, nextState)
        TkString ->
            let (token, nextState) = advanceToken state
             in Right (Literal (internalTokenValue token), nextState)
        TkLBrace -> do
            (_, afterOpen) <- expectToken TkLBrace state
            (body, afterBody) <- parseBody afterOpen
            if internalTokenKind (peekToken afterBody) == TkDoubleSlash
                then do
                    (_, afterSlash) <- expectToken TkDoubleSlash afterBody
                    (separator, afterSeparator) <- parseBody afterSlash
                    (_, afterClose) <- expectToken TkRBrace afterSeparator
                    let (atLeastOne, nextState) =
                            if internalTokenKind (peekToken afterClose) == TkPlus
                                then (True, snd (advanceToken afterClose))
                                else (False, afterClose)
                    Right (SeparatedRepetition body separator atLeastOne, nextState)
                else do
                    (_, afterClose) <- expectToken TkRBrace afterBody
                    if internalTokenKind (peekToken afterClose) == TkPlus
                        then Right (OneOrMoreRepetition body, snd (advanceToken afterClose))
                        else Right (Repetition body, afterClose)
        TkLBracket -> do
            (_, afterOpen) <- expectToken TkLBracket state
            (body, afterBody) <- parseBody afterOpen
            (_, afterClose) <- expectToken TkRBracket afterBody
            Right (Optional body, afterClose)
        TkLParen -> do
            (_, afterOpen) <- expectToken TkLParen state
            (body, afterBody) <- parseBody afterOpen
            (_, afterClose) <- expectToken TkRParen afterBody
            Right (Group body, afterClose)
        _ ->
            Left
                ( ParserGrammarError
                    ("Unexpected token: " ++ show (internalTokenKind (peekToken state)) ++ " (" ++ show (internalTokenValue (peekToken state)) ++ ")")
                    (internalTokenLine (peekToken state))
                )

peekToken :: ParserState -> InternalToken
peekToken state =
    case drop (parserStatePosition state) (parserStateTokens state) of
        token : _ -> token
        [] -> InternalToken TkEof "" 1

advanceToken :: ParserState -> (InternalToken, ParserState)
advanceToken state =
    let token = peekToken state
     in (token, state {parserStatePosition = parserStatePosition state + 1})

expectToken :: TokenKind -> ParserState -> Either ParserGrammarError (InternalToken, ParserState)
expectToken expectedKind state =
    let token = peekToken state
     in if internalTokenKind token == expectedKind
            then Right (advanceToken state)
            else
                Left
                    ( ParserGrammarError
                        ("Expected " ++ show expectedKind ++ ", got " ++ show (internalTokenKind token) ++ " (" ++ show (internalTokenValue token) ++ ")")
                        (internalTokenLine token)
                    )

isUpperTokenReference :: String -> Bool
isUpperTokenReference [] = False
isUpperTokenReference name@(firstChar : _) =
    isAlpha firstChar && name == map toUpper name

toLowerString :: String -> String
toLowerString = map lowerChar
  where
    lowerChar ch
        | isUpper ch =
            toEnum (fromEnum ch + 32)
        | otherwise = ch

trim :: String -> String
trim = reverse . dropWhile isSpaceChar . reverse . dropWhile isSpaceChar
  where
    isSpaceChar ch = ch == ' ' || ch == '\t' || ch == '\r'
