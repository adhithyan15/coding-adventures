module GrammarTools.TokenGrammar
    ( TokenGrammarError(..)
    , TokenDefinition(..)
    , PatternGroup(..)
    , TokenGrammar(..)
    , parseTokenGrammar
    , validateTokenGrammar
    , tokenNames
    , effectiveTokenNames
    ) where

import Data.Char (isAlpha, isAlphaNum, isLower, isSpace, isUpper, toLower)
import Data.List (isPrefixOf)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)
import Text.Read (readMaybe)

data TokenGrammarError = TokenGrammarError
    { tokenGrammarErrorMessage :: String
    , tokenGrammarErrorLineNumber :: Int
    }
    deriving (Eq)

instance Show TokenGrammarError where
    show err =
        "Line "
            ++ show (tokenGrammarErrorLineNumber err)
            ++ ": "
            ++ tokenGrammarErrorMessage err

data TokenDefinition = TokenDefinition
    { tokenDefinitionName :: String
    , tokenDefinitionPattern :: String
    , tokenDefinitionIsRegex :: Bool
    , tokenDefinitionLineNumber :: Int
    , tokenDefinitionAlias :: Maybe String
    }
    deriving (Eq, Show)

data PatternGroup = PatternGroup
    { patternGroupName :: String
    , patternGroupDefinitions :: [TokenDefinition]
    }
    deriving (Eq, Show)

data TokenGrammar = TokenGrammar
    { tokenGrammarVersion :: Int
    , tokenGrammarCaseInsensitive :: Bool
    , tokenGrammarDefinitions :: [TokenDefinition]
    , tokenGrammarKeywords :: [String]
    , tokenGrammarMode :: Maybe String
    , tokenGrammarSkipDefinitions :: [TokenDefinition]
    , tokenGrammarReservedKeywords :: [String]
    , tokenGrammarEscapeMode :: Maybe String
    , tokenGrammarErrorDefinitions :: [TokenDefinition]
    , tokenGrammarGroups :: Map String PatternGroup
    , tokenGrammarCaseSensitive :: Bool
    , tokenGrammarContextKeywords :: [String]
    , tokenGrammarSoftKeywords :: [String]
    }
    deriving (Eq, Show)

data TokenSection
    = SectionKeywords
    | SectionReserved
    | SectionSkip
    | SectionErrors
    | SectionContextKeywords
    | SectionSoftKeywords
    | SectionGroup String
    deriving (Eq, Show)

emptyTokenGrammar :: TokenGrammar
emptyTokenGrammar =
    TokenGrammar
        { tokenGrammarVersion = 0
        , tokenGrammarCaseInsensitive = False
        , tokenGrammarDefinitions = []
        , tokenGrammarKeywords = []
        , tokenGrammarMode = Nothing
        , tokenGrammarSkipDefinitions = []
        , tokenGrammarReservedKeywords = []
        , tokenGrammarEscapeMode = Nothing
        , tokenGrammarErrorDefinitions = []
        , tokenGrammarGroups = Map.empty
        , tokenGrammarCaseSensitive = True
        , tokenGrammarContextKeywords = []
        , tokenGrammarSoftKeywords = []
        }

parseTokenGrammar :: String -> Either TokenGrammarError TokenGrammar
parseTokenGrammar source =
    fst <$> foldl step (Right (emptyTokenGrammar, Nothing)) numberedLines
  where
    numberedLines = zip [1 ..] (lines source)

    step accumulated (lineNumber, rawLine) =
        accumulated >>= \(grammar, currentSection) ->
            parseLine lineNumber rawLine grammar currentSection

parseLine :: Int -> String -> TokenGrammar -> Maybe TokenSection -> Either TokenGrammarError (TokenGrammar, Maybe TokenSection)
parseLine lineNumber rawLine grammar currentSection
    | null stripped = Right (grammar, currentSection)
    | "#" `isPrefixOf` stripped =
        Right (applyMagicComment stripped grammar, currentSection)
    | "mode:" `isPrefixOf` stripped = do
        modeValue <- requireDirectiveValue "mode:" stripped lineNumber
        Right (grammar {tokenGrammarMode = Just modeValue}, Nothing)
    | "escapes:" `isPrefixOf` stripped = do
        escapeValue <- requireDirectiveValue "escapes:" stripped lineNumber
        Right (grammar {tokenGrammarEscapeMode = Just escapeValue}, Nothing)
    | "case_sensitive:" `isPrefixOf` stripped =
        let rawValue = map toLower (trim (drop (length "case_sensitive:") stripped))
         in if rawValue `elem` ["true", "false"]
                then Right (grammar {tokenGrammarCaseSensitive = rawValue == "true"}, Nothing)
                else Left (TokenGrammarError ("Invalid value for 'case_sensitive:': " ++ show rawValue) lineNumber)
    | isGroupHeader stripped = do
        let groupName = trim (init (drop (length "group ") stripped))
        validateGroupName groupName lineNumber grammar
        let group = PatternGroup groupName []
        Right (grammar {tokenGrammarGroups = Map.insert groupName group (tokenGrammarGroups grammar)}, Just (SectionGroup groupName))
    | stripped `elem` ["keywords:", "keywords :"] = Right (grammar, Just SectionKeywords)
    | stripped `elem` ["reserved:", "reserved :"] = Right (grammar, Just SectionReserved)
    | stripped `elem` ["skip:", "skip :"] = Right (grammar, Just SectionSkip)
    | stripped `elem` ["errors:", "errors :"] = Right (grammar, Just SectionErrors)
    | stripped `elem` ["context_keywords:", "context_keywords :"] = Right (grammar, Just SectionContextKeywords)
    | stripped `elem` ["soft_keywords:", "soft_keywords :"] = Right (grammar, Just SectionSoftKeywords)
    | otherwise =
        case currentSection of
            Just section | isIndented rawLine ->
                parseSectionLine section stripped lineNumber grammar
            _ -> do
                definition <- parseTopLevelDefinition stripped lineNumber
                Right (grammar {tokenGrammarDefinitions = tokenGrammarDefinitions grammar ++ [definition]}, Nothing)
  where
    stripped = trim rawLine

parseSectionLine :: TokenSection -> String -> Int -> TokenGrammar -> Either TokenGrammarError (TokenGrammar, Maybe TokenSection)
parseSectionLine section stripped lineNumber grammar =
    case section of
        SectionKeywords ->
            Right
                ( grammar {tokenGrammarKeywords = tokenGrammarKeywords grammar ++ [stripped]}
                , Just section
                )
        SectionReserved ->
            Right
                ( grammar {tokenGrammarReservedKeywords = tokenGrammarReservedKeywords grammar ++ [stripped]}
                , Just section
                )
        SectionContextKeywords ->
            Right
                ( grammar {tokenGrammarContextKeywords = tokenGrammarContextKeywords grammar ++ [stripped]}
                , Just section
                )
        SectionSoftKeywords ->
            Right
                ( grammar {tokenGrammarSoftKeywords = tokenGrammarSoftKeywords grammar ++ [stripped]}
                , Just section
                )
        SectionSkip -> do
            definition <- parseNamedDefinition "skip pattern definition" stripped lineNumber
            Right
                ( grammar {tokenGrammarSkipDefinitions = tokenGrammarSkipDefinitions grammar ++ [definition]}
                , Just section
                )
        SectionErrors -> do
            definition <- parseNamedDefinition "error pattern definition" stripped lineNumber
            Right
                ( grammar {tokenGrammarErrorDefinitions = tokenGrammarErrorDefinitions grammar ++ [definition]}
                , Just section
                )
        SectionGroup groupName -> do
            definition <- parseNamedDefinition ("token definition in group " ++ show groupName) stripped lineNumber
            let updatedGroup =
                    case Map.lookup groupName (tokenGrammarGroups grammar) of
                        Nothing -> PatternGroup groupName [definition]
                        Just group ->
                            group {patternGroupDefinitions = patternGroupDefinitions group ++ [definition]}
            Right
                ( grammar {tokenGrammarGroups = Map.insert groupName updatedGroup (tokenGrammarGroups grammar)}
                , Just section
                )

parseTopLevelDefinition :: String -> Int -> Either TokenGrammarError TokenDefinition
parseTopLevelDefinition stripped lineNumber =
    parseNamedDefinition "token definition" stripped lineNumber

parseNamedDefinition :: String -> String -> Int -> Either TokenGrammarError TokenDefinition
parseNamedDefinition label stripped lineNumber =
    case break (== '=') stripped of
        (_, []) ->
            Left
                ( TokenGrammarError
                    ("Expected " ++ label ++ " (NAME = pattern), got: " ++ show stripped)
                    lineNumber
                )
        (namePart, _ : rest) -> do
            let name = trim namePart
                patternPart = trim rest
            if null name
                then Left (TokenGrammarError "Missing token name before '='" lineNumber)
                else if not (isIdentifier name)
                    then Left (TokenGrammarError ("Invalid token name: " ++ show name) lineNumber)
                    else if null patternPart
                        then Left (TokenGrammarError ("Missing pattern after '=' for token " ++ show name) lineNumber)
                        else parseDefinition patternPart name lineNumber

parseDefinition :: String -> String -> Int -> Either TokenGrammarError TokenDefinition
parseDefinition patternPart namePart lineNumber
    | "/" `isPrefixOf` patternPart =
        let lastSlash = findClosingSlash patternPart
         in if lastSlash < 0
                then Left (TokenGrammarError ("Unclosed regex pattern for token " ++ show namePart) lineNumber)
                else
                    let regexBody = take (lastSlash - 1) (drop 1 patternPart)
                        remainder = trim (drop (lastSlash + 1) patternPart)
                     in if null regexBody
                            then Left (TokenGrammarError ("Empty regex pattern for token " ++ show namePart) lineNumber)
                            else do
                                alias <- parseAlias remainder namePart lineNumber
                                Right (TokenDefinition namePart regexBody True lineNumber alias)
    | "\"" `isPrefixOf` patternPart =
        let closeQuote = findClosingQuote patternPart
         in if closeQuote < 0
                then Left (TokenGrammarError ("Unclosed literal pattern for token " ++ show namePart) lineNumber)
                else
                    let literalBody = take (closeQuote - 1) (drop 1 patternPart)
                        remainder = trim (drop (closeQuote + 1) patternPart)
                     in if null literalBody
                            then Left (TokenGrammarError ("Empty literal pattern for token " ++ show namePart) lineNumber)
                            else do
                                alias <- parseAlias remainder namePart lineNumber
                                Right (TokenDefinition namePart literalBody False lineNumber alias)
    | otherwise =
        Left
            ( TokenGrammarError
                ("Pattern for token " ++ show namePart ++ " must be /regex/ or \"literal\"")
                lineNumber
            )

validateTokenGrammar :: TokenGrammar -> [String]
validateTokenGrammar grammar =
    concat
        [ validateDefinitions (tokenGrammarDefinitions grammar) "token"
        , validateDefinitions (tokenGrammarSkipDefinitions grammar) "skip pattern"
        , validateDefinitions (tokenGrammarErrorDefinitions grammar) "error pattern"
        , validateGroupIssues
        , validateDirectiveIssues
        ]
  where
    validateGroupIssues =
        concat
            [ let issues = validateDefinitions (patternGroupDefinitions group) "group token"
               in if null (patternGroupDefinitions group)
                    then ("Group '" ++ groupName ++ "' has no definitions") : issues
                    else issues
            | (groupName, group) <- Map.toList (tokenGrammarGroups grammar)
            ]
    validateDirectiveIssues =
        concat
            [ case tokenGrammarMode grammar of
                Just modeValue
                    | modeValue /= "indentation" ->
                        ["Unknown lexer mode '" ++ modeValue ++ "' (only 'indentation' is supported)"]
                _ -> []
            , case tokenGrammarEscapeMode grammar of
                Just escapeValue
                    | escapeValue /= "none" ->
                        ["Unknown escape mode '" ++ escapeValue ++ "' (only 'none' is supported)"]
                _ -> []
            ]

validateDefinitions :: [TokenDefinition] -> String -> [String]
validateDefinitions definitions label =
    concatMap definitionIssues definitions
  where
    firstDefinitions =
        Map.fromListWith min
            [ (tokenDefinitionName definition, tokenDefinitionLineNumber definition)
            | definition <- definitions
            ]

    definitionIssues definition =
        duplicateIssues definition ++ conventionIssues definition

    duplicateIssues definition =
        case Map.lookup (tokenDefinitionName definition) firstDefinitions of
            Just firstLine
                | firstLine /= tokenDefinitionLineNumber definition ->
                    [ "Line "
                        ++ show (tokenDefinitionLineNumber definition)
                        ++ ": Duplicate "
                        ++ label
                        ++ " name '"
                        ++ tokenDefinitionName definition
                        ++ "' (first defined on line "
                        ++ show firstLine
                        ++ ")"
                    ]
            _ -> []

    conventionIssues definition =
        concat
            [ if isUpperCaseName (tokenDefinitionName definition)
                then []
                else
                    [ "Line "
                        ++ show (tokenDefinitionLineNumber definition)
                        ++ ": Token name '"
                        ++ tokenDefinitionName definition
                        ++ "' should be UPPER_CASE"
                    ]
            , case tokenDefinitionAlias definition of
                Just aliasName
                    | not (isUpperCaseName aliasName) ->
                        [ "Line "
                            ++ show (tokenDefinitionLineNumber definition)
                            ++ ": Alias '"
                            ++ aliasName
                            ++ "' for token '"
                            ++ tokenDefinitionName definition
                            ++ "' should be UPPER_CASE"
                        ]
                _ -> []
            ]

tokenNames :: TokenGrammar -> Set String
tokenNames grammar =
    Set.fromList
        [ name
        | definition <- allDefinitions grammar
        , name <- tokenDefinitionName definition : maybeToList (tokenDefinitionAlias definition)
        ]

effectiveTokenNames :: TokenGrammar -> Set String
effectiveTokenNames grammar =
    Set.fromList
        [ maybe (tokenDefinitionName definition) id (tokenDefinitionAlias definition)
        | definition <- allDefinitions grammar
        ]

allDefinitions :: TokenGrammar -> [TokenDefinition]
allDefinitions grammar =
    tokenGrammarDefinitions grammar
        ++ concatMap patternGroupDefinitions (Map.elems (tokenGrammarGroups grammar))

applyMagicComment :: String -> TokenGrammar -> TokenGrammar
applyMagicComment stripped grammar =
    case words (drop 1 stripped) of
        ("@version" : rawValue : _) ->
            case readMaybe rawValue of
                Just versionValue -> grammar {tokenGrammarVersion = versionValue}
                Nothing -> grammar
        ("@case_insensitive" : rawValue : _) ->
            grammar {tokenGrammarCaseInsensitive = map toLower rawValue == "true"}
        _ -> grammar

requireDirectiveValue :: String -> String -> Int -> Either TokenGrammarError String
requireDirectiveValue prefix stripped lineNumber =
    let value = trim (drop (length prefix) stripped)
     in if null value
            then Left (TokenGrammarError ("Missing value after '" ++ prefix ++ "'") lineNumber)
            else Right value

validateGroupName :: String -> Int -> TokenGrammar -> Either TokenGrammarError ()
validateGroupName groupName lineNumber grammar
    | null groupName =
        Left (TokenGrammarError "Missing group name after 'group'" lineNumber)
    | not (isLowerIdentifier groupName) =
        Left (TokenGrammarError ("Invalid group name: " ++ show groupName) lineNumber)
    | groupName `elem` ["default", "skip", "keywords", "reserved", "errors"] =
        Left (TokenGrammarError ("Reserved group name: " ++ show groupName) lineNumber)
    | Map.member groupName (tokenGrammarGroups grammar) =
        Left (TokenGrammarError ("Duplicate group name: " ++ show groupName) lineNumber)
    | otherwise =
        Right ()

parseAlias :: String -> String -> Int -> Either TokenGrammarError (Maybe String)
parseAlias remainder namePart lineNumber
    | null remainder = Right Nothing
    | "->" `isPrefixOf` remainder =
        let aliasName = trim (drop 2 remainder)
         in if null aliasName
                then Left (TokenGrammarError ("Missing alias after '->' for token " ++ show namePart) lineNumber)
                else Right (Just aliasName)
    | otherwise =
        Left (TokenGrammarError ("Unexpected text after pattern for token " ++ show namePart ++ ": " ++ show remainder) lineNumber)

findClosingSlash :: String -> Int
findClosingSlash source = go 1 False
  where
    sourceLength = length source
    go index inBracket
        | index >= sourceLength =
            let lastSlash = lastSlashIndex source
             in if lastSlash > 0 then lastSlash else -1
        | otherwise =
            let ch = source !! index
             in if ch == '\\'
                    then go (index + 2) inBracket
                    else
                        case ch of
                            '[' | not inBracket -> go (index + 1) True
                            ']' | inBracket -> go (index + 1) False
                            '/' | not inBracket -> index
                            _ -> go (index + 1) inBracket

findClosingQuote :: String -> Int
findClosingQuote source = go 1
  where
    sourceLength = length source
    go index
        | index >= sourceLength = -1
        | source !! index == '"' = index
        | source !! index == '\\' = go (index + 2)
        | otherwise = go (index + 1)

lastSlashIndex :: String -> Int
lastSlashIndex source =
    case [index | (index, ch) <- zip [0 ..] source, ch == '/'] of
        [] -> -1
        xs -> last xs

isIdentifier :: String -> Bool
isIdentifier [] = False
isIdentifier (firstChar : rest) =
    (isAlpha firstChar || firstChar == '_')
        && all (\ch -> isAlphaNum ch || ch == '_') rest

isLowerIdentifier :: String -> Bool
isLowerIdentifier [] = False
isLowerIdentifier (firstChar : rest) =
    (isLower firstChar || firstChar == '_')
        && all (\ch -> isLower ch || isAlphaNum ch || ch == '_') rest

isUpperCaseName :: String -> Bool
isUpperCaseName [] = False
isUpperCaseName name = all validChar name && any isUpper name
  where
    validChar ch = isUpper ch || isDigitChar ch || ch == '_'
    isDigitChar ch = ch >= '0' && ch <= '9'

isGroupHeader :: String -> Bool
isGroupHeader stripped =
    "group " `isPrefixOf` stripped && ":" `isSuffixOf` stripped
  where
    isSuffixOf suffix value =
        let suffixLength = length suffix
            valueLength = length value
         in valueLength >= suffixLength
                && drop (valueLength - suffixLength) value == suffix

isIndented :: String -> Bool
isIndented [] = False
isIndented (ch : _) = ch == ' ' || ch == '\t'

trim :: String -> String
trim = trimRight . trimLeft

trimLeft :: String -> String
trimLeft = dropWhile isSpace

trimRight :: String -> String
trimRight = reverse . dropWhile isSpace . reverse

maybeToList :: Maybe a -> [a]
maybeToList maybeValue =
    case maybeValue of
        Nothing -> []
        Just value -> [value]
