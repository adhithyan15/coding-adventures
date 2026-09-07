{-# LANGUAGE OverloadedStrings #-}

module LanguageSourceInputRegistry
    ( LanguageRegistry(..)
    , LanguageRule(..)
    , PackageExactInput(..)
    , SelectedSourceFile(..)
    , SourceCandidate(..)
    , SourceCandidateKind(..)
    , SourceCollectionMode(..)
    , SourceCollectionRequest(..)
    , ScopedInput(..)
    , UniversalInputs(..)
    , generatedDirectoryComponents
    , languageSourceInputRegistry
    , languageSourceInputRegistryDigest
    , languageSourceInputRegistryValue
    , selectLanguageSourceInput
    , selectSourceCandidates
    ) where

import Data.Aeson
    ( FromJSON(..)
    , Value
    , eitherDecodeStrict'
    , withObject
    , (.:)
    , (.:?)
    )
import Control.Monad (foldM)
import Data.Char
    ( GeneralCategory(Control, Format, LineSeparator, ParagraphSeparator)
    , generalCategory
    , isAlpha
    , ord
    )
import Data.List (find, isPrefixOf, isSuffixOf)
import qualified Data.Map.Strict as Map
import qualified Data.ByteString as BS
import qualified Data.Set as Set
import qualified Data.Text as Text
import qualified Data.Text.Encoding as TextEncoding
import Sha256 (sha256FinalizeHex, sha256Init, sha256Update)
import System.FilePath.Posix (takeFileName)

import LanguageSourceInputRegistryData (languageSourceInputRegistryJSON)
import qualified TrackedArtifactUnicode17 as TrackedUnicode

data UniversalInputs = UniversalInputs
    { universalBuildFilenames :: [String]
    , universalGeneratedDirectoryComponents :: [String]
    , universalRootExactBasenames :: [String]
    }
    deriving (Eq, Show)

instance FromJSON UniversalInputs where
    parseJSON = withObject "universal source inputs" $ \object ->
        UniversalInputs
            <$> object .: "build_filenames"
            <*> object .: "generated_directory_components"
            <*> object .: "root_exact_basenames"

data PackageExactInput = PackageExactInput
    { packageExactId :: String
    , packageExactRoot :: String
    , packageExactPaths :: [String]
    , packageExactReason :: String
    , packageExactOwner :: String
    }
    deriving (Eq, Show)

instance FromJSON PackageExactInput where
    parseJSON = withObject "package exact source input" $ \object ->
        PackageExactInput
            <$> object .: "id"
            <*> object .: "package_root"
            <*> object .: "paths"
            <*> object .: "reason"
            <*> object .: "owner"

data ScopedInput = ScopedInput
    { scopedId :: String
    , scopedRole :: String
    , scopedDecision :: String
    , scopedScope :: String
    , scopedPathPrefix :: Maybe String
    , scopedSuffixes :: [String]
    , scopedExactBasenames :: [String]
    , scopedReason :: String
    , scopedOwner :: String
    }
    deriving (Eq, Show)

instance FromJSON ScopedInput where
    parseJSON = withObject "scoped source input" $ \object ->
        ScopedInput
            <$> object .: "id"
            <*> object .: "role"
            <*> object .: "decision"
            <*> object .: "scope"
            <*> object .:? "path_prefix"
            <*> object .: "suffixes"
            <*> object .: "exact_basenames"
            <*> object .: "reason"
            <*> object .: "owner"

data LanguageRule = LanguageRule
    { ruleLanguage :: String
    , ruleRecursiveSuffixes :: [String]
    , ruleRecursiveExactBasenames :: [String]
    , ruleRootExactBasenames :: [String]
    , ruleRootVariableSuffixes :: [String]
    , ruleRootExactRelativePaths :: [String]
    , rulePackageExactInputs :: [PackageExactInput]
    , ruleCaseAliasGroups :: [[String]]
    , ruleScopedInputs :: [ScopedInput]
    }
    deriving (Eq, Show)

instance FromJSON LanguageRule where
    parseJSON = withObject "language source rule" $ \object ->
        LanguageRule
            <$> object .: "language"
            <*> object .: "recursive_suffixes"
            <*> object .: "recursive_exact_basenames"
            <*> object .: "root_exact_basenames"
            <*> object .: "root_variable_suffixes"
            <*> object .: "root_exact_relative_paths"
            <*> object .: "package_exact_inputs"
            <*> object .: "case_alias_groups"
            <*> object .: "scoped_inputs"

data LanguageRegistry = LanguageRegistry
    { registrySchemaVersion :: Int
    , registryUniversalInputs :: UniversalInputs
    , registryLanguages :: [LanguageRule]
    }
    deriving (Eq, Show)

data SourceCollectionMode
    = ExtensionMode
    | DeclaredSourcesMode
    deriving (Eq, Show)

data SourceCandidateKind
    = RegularCandidate
    | SymlinkCandidate
    | ReparseCandidate
    deriving (Eq, Show)

data SourceCandidate = SourceCandidate
    { sourceCandidatePath :: FilePath
    , sourceCandidateKind :: SourceCandidateKind
    , sourceCandidateContent :: BS.ByteString
    }
    deriving (Eq, Show)

data SourceCollectionRequest = SourceCollectionRequest
    { sourceCollectionLanguage :: String
    , sourceCollectionPackageRoot :: FilePath
    , sourceCollectionMode :: SourceCollectionMode
    , sourceCollectionRegistryDigest :: String
    , sourceCollectionDeclaredSources :: [String]
    , sourceCollectionCandidates :: [SourceCandidate]
    }
    deriving (Eq, Show)

data SelectedSourceFile = SelectedSourceFile
    { selectedSourcePath :: FilePath
    , selectedSourceDigest :: String
    }
    deriving (Eq, Show)

instance FromJSON LanguageRegistry where
    parseJSON = withObject "language source-input registry" $ \object ->
        LanguageRegistry
            <$> object .: "schema_version"
            <*> object .: "universal_inputs"
            <*> object .: "languages"

registryBytes :: Text.Text
registryBytes = languageSourceInputRegistryJSON

languageSourceInputRegistryValue :: Value
languageSourceInputRegistryValue = decodeEmbedded "value"

languageSourceInputRegistry :: LanguageRegistry
languageSourceInputRegistry = decodeEmbedded "typed projection"

decodeEmbedded :: FromJSON a => String -> a
decodeEmbedded label =
    case eitherDecodeStrict' (TextEncoding.encodeUtf8 registryBytes) of
        Left message -> error ("invalid embedded language source-input registry " ++ label ++ ": " ++ message)
        Right value -> value

languageSourceInputRegistryDigest :: String
languageSourceInputRegistryDigest =
    "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70"

generatedDirectoryComponents :: [String]
generatedDirectoryComponents =
    universalGeneratedDirectoryComponents (registryUniversalInputs languageSourceInputRegistry)

selectLanguageSourceInput :: String -> String -> FilePath -> Either String Bool
selectLanguageSourceInput language packageRoot relative =
    selectLanguageSourceInputForMode True language packageRoot relative

selectLanguageSourceInputForMode :: Bool -> String -> String -> FilePath -> Either String Bool
selectLanguageSourceInputForMode includeRecursive language packageRoot relative = do
    rule <-
        maybe
            (Left "SOURCE_HASH_LANGUAGE_UNKNOWN")
            Right
            (find ((== language) . ruleLanguage) (registryLanguages languageSourceInputRegistry))
    let universal = registryUniversalInputs languageSourceInputRegistry
        basename = takeFileName relative
        rootOnly = '/' `notElem` relative
        matchesSuffix suffix = suffix `isSuffixOf` basename
        universalMatch =
            basename `elem` universalBuildFilenames universal
                || (rootOnly && basename `elem` universalRootExactBasenames universal)
        recursiveMatch =
            any matchesSuffix (ruleRecursiveSuffixes rule)
                || basename `elem` ruleRecursiveExactBasenames rule
        rootMatch =
            rootOnly
                && ( basename `elem` ruleRootExactBasenames rule
                        || any matchesSuffix (ruleRootVariableSuffixes rule)
                   )
        fixedMatch = relative `elem` ruleRootExactRelativePaths rule
        packageMatch = any (matchesPackageExact packageRoot relative) (rulePackageExactInputs rule)
        scopedMatch = any (matchesScoped relative basename rootOnly) (ruleScopedInputs rule)
    pure
        ( universalMatch
            || rootMatch
            || fixedMatch
            || packageMatch
            || (includeRecursive && (recursiveMatch || scopedMatch))
        )

selectSourceCandidates :: SourceCollectionRequest -> Either String [SelectedSourceFile]
selectSourceCandidates request = do
    if sourceCollectionRegistryDigest request == languageSourceInputRegistryDigest
        then Right ()
        else Left "SOURCE_HASH_REGISTRY_MISMATCH"
    _ <-
        selectLanguageSourceInputForMode
            (sourceCollectionMode request == ExtensionMode)
            (sourceCollectionLanguage request)
            (sourceCollectionPackageRoot request)
            "__registry_probe__"
    validatePackageRoot (sourceCollectionLanguage request) (sourceCollectionPackageRoot request)
    if length candidates <= maximumCandidateCount
        then Right ()
        else Left "SOURCE_HASH_LIMIT_EXCEEDED"
    validateDeclaredSources (sourceCollectionDeclaredSources request)
    validated <- validateCandidates candidates
    validateCandidateTopology validated
    let inertPrefixes =
            [ sourceCandidatePath candidate
            | candidate <- validated
            , sourceCandidateKind candidate /= RegularCandidate
            ]
        regularCandidates =
            [ candidate
            | candidate <- validated
            , sourceCandidateKind candidate == RegularCandidate
            , not (underAnyPrefix inertPrefixes (sourceCandidatePath candidate))
            , not (containsGeneratedComponent (sourceCandidatePath candidate))
            ]
    (includedReversed, _, _, _) <-
        foldM selectBounded ([], 0 :: Int, 0 :: Integer, 0 :: Integer) regularCandidates
    Right (sortSelected (reverse includedReversed))
  where
    candidates = sourceCollectionCandidates request
    includeRecursive = sourceCollectionMode request == ExtensionMode
    selectCandidate globWork candidate = do
        registrySelected <-
            selectLanguageSourceInputForMode
                includeRecursive
                (sourceCollectionLanguage request)
                (sourceCollectionPackageRoot request)
                (sourceCandidatePath candidate)
        (declaredSelected, nextGlobWork) <-
            if sourceCollectionMode request == DeclaredSourcesMode && not registrySelected
                then matchDeclaredSources globWork (sourceCandidatePath candidate) (sourceCollectionDeclaredSources request)
                else Right (False, globWork)
        pure
            ( if registrySelected || declaredSelected
                then
                    Just
                        SelectedSourceFile
                            { selectedSourcePath = sourceCandidatePath candidate
                            , selectedSourceDigest =
                                sha256FinalizeHex
                                    (sha256Update sha256Init (sourceCandidateContent candidate))
                            }
                else Nothing
            , nextGlobWork
            )
    selectBounded (included, includedCount, includedBytes, globWork) candidate = do
        (selected, nextGlobWork) <- selectCandidate globWork candidate
        case selected of
            Nothing -> Right (included, includedCount, includedBytes, nextGlobWork)
            Just file
                | includedCount >= maximumSelectedCount -> Left "SOURCE_HASH_LIMIT_EXCEEDED"
                | candidateBytes > maximumFileBytes -> Left "SOURCE_HASH_LIMIT_EXCEEDED"
                | includedBytes + candidateBytes > maximumPackageBytes -> Left "SOURCE_HASH_LIMIT_EXCEEDED"
                | otherwise ->
                    Right
                        ( file : included
                        , includedCount + 1
                        , includedBytes + candidateBytes
                        , nextGlobWork
                        )
              where
                candidateBytes = fromIntegral (BS.length (sourceCandidateContent candidate))

maximumCandidateCount :: Int
maximumCandidateCount = 100000

maximumSelectedCount :: Int
maximumSelectedCount = 50000

maximumFileBytes :: Integer
maximumFileBytes = 64 * 1024 * 1024

maximumPackageBytes :: Integer
maximumPackageBytes = 1024 * 1024 * 1024

maximumDeclaredPatterns :: Int
maximumDeclaredPatterns = 256

maximumDeclaredPatternBytes :: Int
maximumDeclaredPatternBytes = 64 * 1024

maximumDeclaredMatchWork :: Integer
maximumDeclaredMatchWork = 50000000

validateDeclaredSources :: [String] -> Either String ()
validateDeclaredSources patterns
    | length patterns > maximumDeclaredPatterns = Left "SOURCE_HASH_LIMIT_EXCEEDED"
    | sum (map (BS.length . TextEncoding.encodeUtf8 . Text.pack) patterns) > maximumDeclaredPatternBytes =
        Left "SOURCE_HASH_LIMIT_EXCEEDED"
    | otherwise = mapM_ validatePortableGlob patterns

matchDeclaredSources :: Integer -> FilePath -> [String] -> Either String (Bool, Integer)
matchDeclaredSources initialWork path = go initialWork
  where
    go work [] = Right (False, work)
    go work (patternValue : remaining) =
        let cost = fromIntegral ((length patternValue + 1) * (length path + 1))
            nextWork = work + cost
         in if nextWork > maximumDeclaredMatchWork
                then Left "SOURCE_HASH_LIMIT_EXCEEDED"
                else
                    if portableGlobMatches patternValue path
                        then Right (True, nextWork)
                        else go nextWork remaining

validateCandidates :: [SourceCandidate] -> Either String [SourceCandidate]
validateCandidates candidates = reverse . snd <$> foldM step (Map.empty, []) candidates
  where
    step (identities, accepted) candidate = do
        path <- validatePortablePath (sourceCandidatePath candidate)
        let identity = TrackedUnicode.casefold (TrackedUnicode.nfc path)
        case Map.lookup identity identities of
            Just _ -> Left "SOURCE_HASH_PATH_COLLISION"
            _ ->
                Right
                    ( Map.insert identity path identities
                    , candidate{sourceCandidatePath = path} : accepted
                    )

validateCandidateTopology :: [SourceCandidate] -> Either String ()
validateCandidateTopology candidates = go ordered
  where
    ordered =
        map snd
            . Map.toAscList
            . Map.fromList
            $ map (\candidate -> (TextEncoding.encodeUtf8 (Text.pack (sourceCandidatePath candidate)), candidate)) candidates
    go (candidate : next : remaining)
        | sourceCandidateKind candidate == RegularCandidate
            && (sourceCandidatePath candidate ++ "/") `isPrefixOf` sourceCandidatePath next =
            Left "SOURCE_HASH_PATH_COLLISION"
        | otherwise = go (next : remaining)
    go _ = Right ()

validatePortablePath :: FilePath -> Either String FilePath
validatePortablePath path
    | null path = Left "SOURCE_HASH_PATH_INVALID"
    | length path > 512 = Left "SOURCE_HASH_PATH_INVALID"
    | head path == '/' = Left "SOURCE_HASH_PATH_INVALID"
    | '\\' `elem` path = Left "SOURCE_HASH_PATH_INVALID"
    | '\0' `elem` path = Left "SOURCE_HASH_PATH_INVALID"
    | hasDrivePrefix path = Left "SOURCE_HASH_PATH_INVALID"
    | any invalidUnicodeScalar path = Left "SOURCE_HASH_PATH_INVALID"
    | any unsafePathCharacter path = Left "SOURCE_HASH_PATH_INVALID"
    | TrackedUnicode.nfc path /= path = Left "SOURCE_HASH_PATH_INVALID"
    | any invalidComponent components = Left "SOURCE_HASH_PATH_INVALID"
    | otherwise = Right path
  where
    components = splitPortablePath path
    invalidComponent component =
        null component
            || component == "."
            || component == ".."
            || last component == ' '
            || last component == '.'
            || reservedBasename component
    invalidUnicodeScalar character =
        let value = ord character
         in value >= 0xD800 && value <= 0xDFFF

validatePackageRoot :: String -> FilePath -> Either String ()
validatePackageRoot language packageRoot =
    case validatePortablePath packageRoot of
        Left _ -> Left "SOURCE_HASH_PACKAGE_ROOT_INVALID"
        Right _ ->
            case splitPortablePath packageRoot of
                "code" : bucket : rootLanguage : rest
                    | bucket `elem` ["packages", "programs"]
                        && rootLanguage == language
                        && not (null rest) -> Right ()
                _ -> Left "SOURCE_HASH_PACKAGE_ROOT_INVALID"

hasDrivePrefix :: String -> Bool
hasDrivePrefix (first : ':' : _) = isAlpha first
hasDrivePrefix _ = False

unsafePathCharacter :: Char -> Bool
unsafePathCharacter character =
    ord character < 0x20
        || character `elem` ['\DEL', '<', '>', ':', '"', '|', '?', '*']
        || generalCategory character `elem` [Control, Format, LineSeparator, ParagraphSeparator]

unsafeGlobCharacter :: Char -> Bool
unsafeGlobCharacter character =
    ord character < 0x20
        || character `elem` ['\DEL', '<', '>', ':', '"', '|']
        || generalCategory character `elem` [Control, Format, LineSeparator, ParagraphSeparator]

reservedBasename :: String -> Bool
reservedBasename component =
    Set.member
        (TrackedUnicode.fullUppercase (takeWhile (/= '.') component))
        windowsReservedBasenames

windowsReservedBasenames :: Set.Set String
windowsReservedBasenames =
    Set.fromList
        ( ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"]
            ++ [prefix ++ suffix | prefix <- ["COM", "LPT"], suffix <- map show [1 :: Int .. 9] ++ ["\x00B9", "\x00B2", "\x00B3"]]
        )

containsGeneratedComponent :: FilePath -> Bool
containsGeneratedComponent = any (`elem` generatedDirectoryComponents) . splitPortablePath

underAnyPrefix :: [FilePath] -> FilePath -> Bool
underAnyPrefix prefixes path =
    any (\prefix -> path == prefix || (prefix ++ "/") `isPrefixOf` path) prefixes

splitPortablePath :: FilePath -> [String]
splitPortablePath value =
    case break (== '/') value of
        (component, []) -> [component]
        (component, _ : rest) -> component : splitPortablePath rest

sortSelected :: [SelectedSourceFile] -> [SelectedSourceFile]
sortSelected =
    map snd
        . Map.toAscList
        . Map.fromList
        . map (\file -> (TextEncoding.encodeUtf8 (Text.pack (selectedSourcePath file)), file))

portableGlobMatches :: String -> FilePath -> Bool
portableGlobMatches patternValue path =
    matchSegments (splitPortablePath patternValue) (splitPortablePath path)
  where
    matchSegments [] [] = True
    matchSegments ("**" : patterns) segments =
        matchSegments patterns segments
            || case segments of
                [] -> False
                _ : remaining -> matchSegments ("**" : patterns) remaining
    matchSegments (patternSegment : patterns) (pathSegment : segments) =
        matchSegment patternSegment pathSegment && matchSegments patterns segments
    matchSegments _ _ = False

    matchSegment [] [] = True
    matchSegment ('*' : patterns) characters =
        matchSegment patterns characters
            || case characters of
                [] -> False
                _ : rest -> matchSegment ('*' : patterns) rest
    matchSegment ('?' : patterns) (_ : characters) = matchSegment patterns characters
    matchSegment patternCharacters@('[' : _) (character : characters) =
        case parseCharacterClass patternCharacters of
            Just (negated, members, remainingPattern) ->
                (if negated then not (classMatches members character) else classMatches members character)
                    && matchSegment remainingPattern characters
            Nothing -> character == '[' && matchSegment (tail patternCharacters) characters
    matchSegment (patternCharacter : patterns) (character : characters) =
        patternCharacter == character && matchSegment patterns characters
    matchSegment _ _ = False

validatePortableGlob :: String -> Either String ()
validatePortableGlob patternValue
    | null patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | length patternValue > 512 = Left "SOURCE_HASH_GLOB_INVALID"
    | head patternValue == '/' = Left "SOURCE_HASH_GLOB_INVALID"
    | '\\' `elem` patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | "//" `isPrefixWithin` patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | hasDrivePrefix patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | any invalidUnicodeScalar patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | any unsafeGlobCharacter patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | TrackedUnicode.nfc patternValue /= patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | any invalidGlobComponent (splitPortablePath patternValue) = Left "SOURCE_HASH_GLOB_INVALID"
    | hasInvalidCharacterClass patternValue = Left "SOURCE_HASH_GLOB_INVALID"
    | otherwise = Right ()
  where
    invalidGlobComponent component =
        null component
            || component == "."
            || component == ".."
            || last component == ' '
            || last component == '.'
            || (not (any (`elem` ("*[]{}" :: String)) component) && reservedBasename component)
    invalidUnicodeScalar character =
        let value = ord character
         in value >= 0xD800 && value <= 0xDFFF

isPrefixWithin :: String -> String -> Bool
isPrefixWithin needle haystack =
    any (needle `isPrefixOf`) (tails haystack)
  where
    tails [] = [[]]
    tails value@(_ : rest) = value : tails rest

data ClassMember
    = ClassLiteral Char
    | ClassRange Char Char

parseCharacterClass :: String -> Maybe (Bool, [ClassMember], String)
parseCharacterClass ('[' : rest) = do
    let (negated, afterNegation) =
            case rest of
                '!' : remaining -> (True, remaining)
                _ -> (False, rest)
        (prefixMembers, afterLeadingClose) =
            case afterNegation of
                ']' : remaining -> ("]", remaining)
                _ -> ([], afterNegation)
        (bodyRest, closingAndRest) = break (== ']') afterLeadingClose
    case closingAndRest of
        [] -> Nothing
        _ : remainingPattern ->
            let body = prefixMembers ++ bodyRest
             in if null body
                    then Nothing
                    else Just (negated, parseClassMembers body, remainingPattern)
parseCharacterClass _ = Nothing

parseClassMembers :: String -> [ClassMember]
parseClassMembers (lower : '-' : upper : remaining) =
    ClassRange lower upper : parseClassMembers remaining
parseClassMembers (character : remaining) =
    ClassLiteral character : parseClassMembers remaining
parseClassMembers [] = []

classMatches :: [ClassMember] -> Char -> Bool
classMatches members character = any matches members
  where
    matches (ClassLiteral literal) = character == literal
    matches (ClassRange lower upper) = lower <= character && character <= upper

hasInvalidCharacterClass :: String -> Bool
hasInvalidCharacterClass [] = False
hasInvalidCharacterClass patternValue@('[' : rest) =
    case parseCharacterClass patternValue of
        Nothing -> hasInvalidCharacterClass rest
        Just (_, members, remaining) ->
            any invalidRange members
                || any (`isPrefixWithin` classBody patternValue) ["--", "&&", "~~", "||"]
                || hasInvalidCharacterClass remaining
  where
    invalidRange (ClassRange lower upper) = lower > upper
    invalidRange _ = False
hasInvalidCharacterClass (_ : rest) = hasInvalidCharacterClass rest

classBody :: String -> String
classBody ('[' : rest) =
    let afterNegation = case rest of '!' : remaining -> remaining; _ -> rest
        afterLeadingClose = case afterNegation of ']' : remaining -> remaining; _ -> afterNegation
     in takeWhile (/= ']') afterLeadingClose
classBody _ = []

matchesPackageExact :: String -> FilePath -> PackageExactInput -> Bool
matchesPackageExact packageRoot relative rule =
    packageRoot == packageExactRoot rule && relative `elem` packageExactPaths rule

matchesScoped :: FilePath -> FilePath -> Bool -> ScopedInput -> Bool
matchesScoped relative basename rootOnly rule =
    scopedDecision rule == "include"
        && inScope
        && (basename `elem` scopedExactBasenames rule || any (`isSuffixOf` basename) (scopedSuffixes rule))
  where
    inScope =
        case scopedScope rule of
            "root" -> rootOnly
            "subtree" ->
                case scopedPathPrefix rule of
                    Just prefix -> (prefix ++ "/") `isPrefixOf` relative
                    Nothing -> False
            _ -> False
