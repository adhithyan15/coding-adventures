module BuildTool
    ( BuildResult(..)
    , Config(..)
    , MetadataEncodingError(..)
    , Package(..)
    , ParsedArgs(..)
    , TrackedArtifactDiagnostic(..)
    , TrackedArtifactDiagnosticDetails(..)
    , TrackedArtifactEntry(..)
    , defaultConfig
    , discoverPackages
    , findRepoRoot
    , hashPackage
    , inferLanguage
    , parseArgs
    , renderMetadataEncodingError
    , resolveDependencies
    , runWithArgs
    , trackedArtifactUnicodeVersion
    , validateTrackedArtifactSnapshot
    ) where

import Control.Exception (Exception, IOException, catch, evaluate, throwIO, try)
import Control.Monad (filterM, foldM, forM, when)
import Data.Aeson (Value(..), eitherDecodeStrict')
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KeyMap
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BS8
import Data.Char (isAlpha, isAlphaNum, isAsciiLower, isAsciiUpper, isSpace, ord, toLower)
import Data.List (intercalate, isInfixOf, isPrefixOf, nub, sort, sortOn)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Maybe (fromMaybe, listToMaybe, mapMaybe, maybeToList)
import qualified Data.Set as Set
import Data.Set (Set)
import qualified Data.Text as Text
import qualified Data.Text.Encoding as TextEncoding
import Data.Time.Clock (diffUTCTime, getCurrentTime)
import Data.Time.Format (defaultTimeLocale, formatTime)
import qualified DirectedGraph as DG
import GHC.Conc (getNumCapabilities)
import System.Directory
    ( canonicalizePath
    , createDirectoryIfMissing
    , doesDirectoryExist
    , doesFileExist
    , getCurrentDirectory
    , listDirectory
    )
import System.Exit (ExitCode(..))
import System.FilePath
    ( (</>)
    , addTrailingPathSeparator
    , isAbsolute
    , isPathSeparator
    , joinPath
    , makeRelative
    , normalise
    , pathSeparator
    , splitDirectories
    , takeDirectory
    , takeExtension
    , takeFileName
    )
import System.IO (hClose, hPutStrLn, hSetBinaryMode, stderr)
import qualified System.Info as SystemInfo
import System.Process
    ( CreateProcess(..)
    , StdStream(CreatePipe)
    , createProcess
    , proc
    , readCreateProcessWithExitCode
    , waitForProcess
    )
import Text.Read (readMaybe)
import qualified TrackedArtifactUnicode17 as TrackedUnicode

versionString :: String
versionString = "0.2.0"

data Config = Config
    { configRoot :: Maybe FilePath
    , configDiffBase :: String
    , configForce :: Bool
    , configDryRun :: Bool
    , configJobs :: Maybe Int
    , configLanguage :: String
    , configCacheFile :: FilePath
    , configEmitPlan :: Bool
    , configPlanFile :: FilePath
    , configValidateBuildFiles :: Bool
    }
    deriving (Eq, Show)

data ParsedArgs
    = ParsedHelp
    | ParsedVersion
    | ParsedRun Config
    deriving (Eq, Show)

data MetadataEncodingError = MetadataEncodingError
    { metadataErrorCode :: String
    , metadataErrorPackage :: String
    , metadataErrorManifest :: FilePath
    , metadataErrorEncoding :: String
    }
    deriving (Eq)

instance Show MetadataEncodingError where
    show = renderMetadataEncodingError

instance Exception MetadataEncodingError

data TrackedArtifactEntry = TrackedArtifactEntry
    { trackedEntryOrdinal :: Int
    , trackedEntryPath :: String
    , trackedEntryKind :: String
    }
    deriving (Eq, Show)

data TrackedArtifactDiagnosticDetails = TrackedArtifactDiagnosticDetails
    { trackedDiagnosticOrdinal :: Int
    , trackedDiagnosticEntryKind :: String
    , trackedDiagnosticProblem :: Maybe String
    }
    deriving (Eq, Show)

data TrackedArtifactDiagnostic = TrackedArtifactDiagnostic
    { trackedDiagnosticCode :: String
    , trackedDiagnosticSeverity :: String
    , trackedDiagnosticPath :: String
    , trackedDiagnosticDetails :: TrackedArtifactDiagnosticDetails
    }
    deriving (Eq, Show)

trackedArtifactUnicodeVersion :: String
trackedArtifactUnicodeVersion = TrackedUnicode.unicodeVersion

trackedArtifactComponentIdentity :: String
trackedArtifactComponentIdentity = "node_modules"

trackedArtifactRedactedPath :: String
trackedArtifactRedactedPath = "repository"

windowsReservedBasenames :: Set String
windowsReservedBasenames =
    Set.fromList
        ( ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"]
            ++ [prefix ++ suffix | prefix <- ["COM", "LPT"], suffix <- map show [1 :: Int .. 9]]
            ++ [prefix ++ suffix | prefix <- ["COM", "LPT"], suffix <- ["¹", "²", "³"]]
        )

validateTrackedArtifactSnapshot
    :: String
    -> [TrackedArtifactEntry]
    -> Either String [TrackedArtifactDiagnostic]
validateTrackedArtifactSnapshot unicodeVersion entries
    | unicodeVersion /= trackedArtifactUnicodeVersion =
        Left
            ( "tracked artifact Unicode version must be "
                ++ trackedArtifactUnicodeVersion
            )
    | otherwise = Right (sortOn trackedArtifactDiagnosticSortKey (mapMaybe validate entries))
  where
    validate entry =
        case normalizeTrackedArtifactPath (trackedEntryPath entry) of
            Left problem ->
                Just
                    TrackedArtifactDiagnostic
                        { trackedDiagnosticCode = "TRACKED_ARTIFACT_PATH_INVALID"
                        , trackedDiagnosticSeverity = "error"
                        , trackedDiagnosticPath = trackedArtifactRedactedPath
                        , trackedDiagnosticDetails = details entry (Just problem)
                        }
            Right normalizedPath
                | any
                    ((== trackedArtifactComponentIdentity) . TrackedUnicode.nfkcCasefold)
                    (splitTrackedPath normalizedPath) ->
                    Just
                        TrackedArtifactDiagnostic
                            { trackedDiagnosticCode = "TRACKED_ARTIFACT_FORBIDDEN"
                            , trackedDiagnosticSeverity = "error"
                            , trackedDiagnosticPath = normalizedPath
                            , trackedDiagnosticDetails = details entry Nothing
                            }
                | otherwise -> Nothing
    details entry problem =
        TrackedArtifactDiagnosticDetails
            { trackedDiagnosticOrdinal = trackedEntryOrdinal entry
            , trackedDiagnosticEntryKind = trackedEntryKind entry
            , trackedDiagnosticProblem = problem
            }

normalizeTrackedArtifactPath :: String -> Either String String
normalizeTrackedArtifactPath path
    | null path = Left "EMPTY"
    | not (null (drop 512 path)) = Left "TOO_LONG"
    | TrackedUnicode.nfc normalized /= normalized = Left "NON_NFC"
    | head normalized == '/' = Left "ABSOLUTE"
    | driveQualified normalized = Left "DRIVE_QUALIFIED"
    | any null segments = Left "EMPTY_SEGMENT"
    | any unsafeTrackedArtifactCharacter normalized = Left "UNSAFE_CHARACTER"
    | otherwise =
        case listToMaybe (mapMaybe trackedArtifactSegmentProblem segments) of
            Just problem -> Left problem
            Nothing -> Right normalized
  where
    normalized = map (\character -> if character == '\\' then '/' else character) path
    segments = splitTrackedPath normalized

driveQualified :: String -> Bool
driveQualified (drive : ':' : _) = asciiAlpha drive
driveQualified _ = False

asciiAlpha :: Char -> Bool
asciiAlpha character =
    isAsciiUpper character || isAsciiLower character

unsafeTrackedArtifactCharacter :: Char -> Bool
unsafeTrackedArtifactCharacter character =
    ord character < 0x20 || character `elem` ("<>:\"|?*" :: String)

trackedArtifactSegmentProblem :: String -> Maybe String
trackedArtifactSegmentProblem segment
    | segment == "." || segment == ".." = Just "DOT_SEGMENT"
    | last segment == '.' || last segment == ' ' = Just "TRAILING_DOT_OR_SPACE"
    | Set.member (TrackedUnicode.fullUppercase (takeWhile (/= '.') segment)) windowsReservedBasenames =
        Just "RESERVED_BASENAME"
    | otherwise = Nothing

splitTrackedPath :: String -> [String]
splitTrackedPath value =
    case break (== '/') value of
        (segment, []) -> [segment]
        (segment, _ : rest) -> segment : splitTrackedPath rest

trackedArtifactDiagnosticSortKey
    :: TrackedArtifactDiagnostic
    -> (String, String, String)
trackedArtifactDiagnosticSortKey diagnostic =
    ( trackedDiagnosticCode diagnostic
    , trackedDiagnosticPath diagnostic
    , intercalate
        "\0"
        [ trackedDiagnosticEntryKind details
        , show (trackedDiagnosticOrdinal details)
        , fromMaybe "" (trackedDiagnosticProblem details)
        ]
    )
  where
    details = trackedDiagnosticDetails diagnostic

renderMetadataEncodingError :: MetadataEncodingError -> String
renderMetadataEncodingError metadataError =
    intercalate
        " "
        [ metadataErrorCode metadataError ++ ":"
        , "package=" ++ metadataErrorPackage metadataError
        , "manifest=" ++ metadataErrorManifest metadataError
        , "encoding=" ++ metadataErrorEncoding metadataError
        ]

data Package = Package
    { packageName :: String
    , packagePath :: FilePath
    , packageBuildFile :: FilePath
    , packageBuildCommands :: [String]
    , packageLanguage :: String
    }
    deriving (Eq, Show)

data BuildResult = BuildResult
    { resultPackageName :: String
    , resultStatus :: String
    , resultDurationSeconds :: Double
    , resultStdout :: String
    , resultStderr :: String
    , resultReturnCode :: Int
    }
    deriving (Eq, Read, Show)

data CacheEntry = CacheEntry
    { cachePackageHash :: String
    , cacheDependencyHash :: String
    }
    deriving (Eq, Read, Show)

newtype BuildCache = BuildCache
    { cacheEntries :: Map String CacheEntry
    }
    deriving (Eq, Read, Show)

data PlanItem = PlanItem
    { planPackageName :: String
    , planLanguage :: String
    , planPath :: FilePath
    , planBuildFile :: FilePath
    , planCommands :: [String]
    , planReason :: String
    }
    deriving (Eq, Read, Show)

data PlanDocument = PlanDocument
    { planGeneratedAt :: String
    , planRepoRoot :: FilePath
    , planPackages :: [PlanItem]
    }
    deriving (Eq, Read, Show)

defaultConfig :: Config
defaultConfig =
    Config
        { configRoot = Nothing
        , configDiffBase = "origin/main"
        , configForce = False
        , configDryRun = False
        , configJobs = Nothing
        , configLanguage = "all"
        , configCacheFile = ".build-cache.json"
        , configEmitPlan = False
        , configPlanFile = "build-plan.json"
        , configValidateBuildFiles = False
        }

usageText :: String
usageText =
    unlines
        [ "build-tool -- Incremental, dependency-aware monorepo build runner"
        , ""
        , "USAGE:"
        , "    build-tool [OPTIONS]"
        , ""
        , "OPTIONS:"
        , "        --root <PATH>              Repo root (auto-detects .git if omitted)"
        , "        --diff-base <REF>          Git ref for change detection (default: origin/main)"
        , "        --force                    Rebuild everything regardless of cache"
        , "        --dry-run                  Show what would build without executing"
        , "        --jobs <N>                 Max parallel jobs per dependency level"
        , "        --language <LANG>          Filter to a single language or all"
        , "        --cache-file <PATH>        Cache file path (default: .build-cache.json)"
        , "        --emit-plan                Emit a JSON build plan"
        , "        --plan-file <PATH>         Plan file path (default: build-plan.json)"
        , "        --validate-build-files     Fail fast on empty BUILD files or unknown languages"
        , "    -h, --help                     Show this help"
        , "    -V, --version                  Show version"
        ]

parseArgs :: [String] -> Either String ParsedArgs
parseArgs = go defaultConfig
  where
    go cfg [] = Right (ParsedRun cfg)
    go _ ["--root"] = Left "--root requires a value"
    go _ ["--diff-base"] = Left "--diff-base requires a value"
    go _ ["--jobs"] = Left "--jobs requires a value"
    go _ ["--language"] = Left "--language requires a value"
    go _ ["--cache-file"] = Left "--cache-file requires a value"
    go _ ["--plan-file"] = Left "--plan-file requires a value"
    go _ ("-h" : _) = Right ParsedHelp
    go _ ("--help" : _) = Right ParsedHelp
    go _ ("-V" : _) = Right ParsedVersion
    go _ ("--version" : _) = Right ParsedVersion
    go cfg ("--root" : value : rest) = go cfg{configRoot = Just value} rest
    go cfg ("--diff-base" : value : rest) = go cfg{configDiffBase = value} rest
    go cfg ("--force" : rest) = go cfg{configForce = True} rest
    go cfg ("--dry-run" : rest) = go cfg{configDryRun = True} rest
    go cfg ("--jobs" : value : rest) =
        case reads value of
            [(jobs, "")] | jobs > 0 -> go cfg{configJobs = Just jobs} rest
            _ -> Left ("invalid value for --jobs: " ++ value)
    go cfg ("--language" : value : rest) =
        go cfg{configLanguage = map toLower value} rest
    go cfg ("--cache-file" : value : rest) = go cfg{configCacheFile = value} rest
    go cfg ("--emit-plan" : rest) = go cfg{configEmitPlan = True} rest
    go cfg ("--plan-file" : value : rest) = go cfg{configPlanFile = value} rest
    go cfg ("--validate-build-files" : rest) =
        go cfg{configValidateBuildFiles = True} rest
    go _ [flag]
        | "--" `isPrefixOf` flag = Left ("unknown flag: " ++ flag)
    go _ (flag : _)
        | "--" `isPrefixOf` flag = Left ("unknown flag: " ++ flag)
    go _ (value : _)
        = Left ("unexpected positional argument: " ++ value)

runWithArgs :: [String] -> IO Int
runWithArgs rawArgs =
    case parseArgs rawArgs of
        Left err -> do
            putStrLn err
            putStrLn ""
            putStrLn usageText
            pure 1
        Right ParsedHelp -> do
            putStrLn usageText
            pure 0
        Right ParsedVersion -> do
            putStrLn versionString
            pure 0
        Right (ParsedRun cfg) -> runBuild cfg `catch` handleMetadataEncodingError

handleMetadataEncodingError :: MetadataEncodingError -> IO Int
handleMetadataEncodingError metadataError = do
    hPutStrLn stderr (renderMetadataEncodingError metadataError)
    pure 2

runBuild :: Config -> IO Int
runBuild cfg = do
    repoRootResult <- resolveRepoRoot cfg
    case repoRootResult of
        Left err -> do
            putStrLn err
            pure 1
        Right repoRoot -> do
            let codeRoot = repoRoot </> "code"
            codeRootExists <- doesDirectoryExist codeRoot
            if not codeRootExists
                then do
                    putStrLn ("code directory not found at " ++ codeRoot)
                    pure 1
                else do
                    packages <- discoverPackages codeRoot
                    let filteredPackages = filterByLanguage (configLanguage cfg) packages
                    validationErrors <-
                        if configValidateBuildFiles cfg
                            then validatePackages filteredPackages
                            else pure []
                    if not (null validationErrors)
                        then do
                            mapM_ putStrLn validationErrors
                            pure 1
                        else do
                            graph <- resolveDependencies filteredPackages
                            packageHashes <- hashPackages filteredPackages
                            let dependencyHashes =
                                    Map.fromList
                                        [ (packageName pkg, hashDependencyClosure graph packageHashes (packageName pkg))
                                        | pkg <- filteredPackages
                                        ]
                            let cachePath = absolutizeWithin repoRoot (configCacheFile cfg)
                            cache <- loadCache cachePath
                            affectedSet <- detectAffectedPackages repoRoot (configDiffBase cfg) filteredPackages graph
                            timestamp <- currentTimestamp
                            when (configEmitPlan cfg) $
                                writePlan
                                    (absolutizeWithin repoRoot (configPlanFile cfg))
                                    timestamp
                                    repoRoot
                                    filteredPackages
                                    cache
                                    packageHashes
                                    dependencyHashes
                                    graph
                                    cfg
                                    affectedSet
                            results <-
                                executeBuilds
                                    filteredPackages
                                    graph
                                    cache
                                    packageHashes
                                    dependencyHashes
                                    cfg
                                    affectedSet
                            let updatedCache =
                                    foldl
                                        (\acc result ->
                                            if resultStatus result == "built"
                                                then
                                                    putCacheEntry
                                                        acc
                                                        (resultPackageName result)
                                                        (Map.findWithDefault "" (resultPackageName result) packageHashes)
                                                        (Map.findWithDefault "" (resultPackageName result) dependencyHashes)
                                                else acc
                                        )
                                        cache
                                        results
                            saveCache cachePath updatedCache
                            renderResults results
                            pure $
                                if any (\result -> resultStatus result == "failed") results
                                    then 1
                                    else 0

resolveRepoRoot :: Config -> IO (Either String FilePath)
resolveRepoRoot cfg =
    case configRoot cfg of
        Just root -> Right <$> canonicalizePath root
        Nothing -> do
            found <- findRepoRoot Nothing
            pure $
                case found of
                    Nothing -> Left "could not locate repo root (.git directory)"
                    Just root -> Right root

findRepoRoot :: Maybe FilePath -> IO (Maybe FilePath)
findRepoRoot maybeStart = do
    start <- maybe getCurrentDirectory pure maybeStart
    absolute <- canonicalizePath start
    walk absolute
  where
    walk current = do
        let gitPath = current </> ".git"
        hasGitDir <- doesDirectoryExist gitPath
        hasGitFile <- doesFileExist gitPath
        let hasGit = hasGitDir || hasGitFile
        if hasGit
            then pure (Just current)
            else
                let parent = takeDirectory current
                 in if parent == current
                        then pure Nothing
                        else walk parent

discoverPackages :: FilePath -> IO [Package]
discoverPackages codeRoot = do
    canonicalRoot <- canonicalizePath codeRoot
    found <- walk canonicalRoot
    pure (sortOnPackageName found)
  where
    walk current = do
        skipCurrent <- shouldSkipDirectory current
        if skipCurrent
            then pure []
            else do
                maybeBuildFile <- getBuildFile current
                case maybeBuildFile of
                    Just buildFile -> do
                        commands <- readBuildCommands buildFile
                        pure
                            [ Package
                                { packageName = inferPackageName current
                                , packagePath = current
                                , packageBuildFile = buildFile
                                , packageBuildCommands = commands
                                , packageLanguage = inferLanguage current
                                }
                            ]
                    Nothing -> do
                        entries <- listDirectory current
                        directories <-
                            filterM
                                (doesDirectoryExist . (current </>))
                                entries
                        nested <- mapM (walk . (current </>)) directories
                        pure (concat nested)

sortOnPackageName :: [Package] -> [Package]
sortOnPackageName = sortOn packageName

shouldSkipDirectory :: FilePath -> IO Bool
shouldSkipDirectory path =
    pure (takeFileName path `elem` skipDirectories)

skipDirectories :: [String]
skipDirectories =
    [ ".git"
    , ".hg"
    , ".svn"
    , ".venv"
    , ".tox"
    , ".mypy_cache"
    , ".pytest_cache"
    , ".ruff_cache"
    , ".stack-work"
    , "__pycache__"
    , "build"
    , "deps"
    , "dist"
    , "node_modules"
    , "target"
    , "vendor"
    , ".claude"
    , ".build"
    , "cover"
    , "Pods"
    ]

inferLanguage :: FilePath -> String
inferLanguage path =
    case filter (`elem` supportedLanguages) (map (map toLower) (splitDirectories (normalise path))) of
        lang : _ -> lang
        [] -> "unknown"

supportedLanguages :: [String]
supportedLanguages =
    [ "python"
    , "ruby"
    , "go"
    , "rust"
    , "dart"
    , "typescript"
    , "elixir"
    , "lua"
    , "perl"
    , "swift"
    , "haskell"
    , "wasm"
    , "starlark"
    , "java"
    , "kotlin"
    , "csharp"
    , "fsharp"
    , "dotnet"
    ]

inferPackageName :: FilePath -> String
inferPackageName path =
    let language = inferLanguage path
        components = map (map toLower) (splitDirectories (normalise path))
        isProgram =
            any
                (\(parent, child) -> parent == "programs" && child == language)
                (zip components (drop 1 components))
        identityPrefix = if isProgram then language ++ "/programs/" else language ++ "/"
     in identityPrefix ++ takeFileName path

getBuildFile :: FilePath -> IO (Maybe FilePath)
getBuildFile directory = do
    let platformCandidates =
            case hostOS of
                "darwin" -> ["BUILD_mac", "BUILD_mac_and_linux", "BUILD"]
                "linux" -> ["BUILD_linux", "BUILD_mac_and_linux", "BUILD"]
                "mingw32" -> ["BUILD_windows", "BUILD"]
                _ -> ["BUILD"]
    existing <- filterM (doesFileExist . (directory </>)) platformCandidates
    pure (listToMaybe (map (directory </>) existing))

hostOS :: String
hostOS = map toLower osName
  where
    osName = SystemInfo.os

readBuildCommands :: FilePath -> IO [String]
readBuildCommands path = do
    contents <- readFileStrict path
    pure
        [ trim line
        | line <- lines contents
        , let stripped = trim line
        , not (null stripped)
        , not ("#" `isPrefixOf` stripped)
        ]

-- Prelude.readFile is lazy and can retain a Windows file handle after package
-- discovery has returned. Force the complete text before handing it to a
-- parser so the handle reaches EOF and closes deterministically.
readFileStrict :: FilePath -> IO String
readFileStrict path = do
    contents <- readFile path
    _ <- evaluate (length contents)
    pure contents

validatePackages :: [Package] -> IO [String]
validatePackages packages =
    pure $
        concatMap
            (\pkg ->
                [ "unknown language for " ++ packagePath pkg
                | packageLanguage pkg == "unknown"
                ]
                    ++ [ "BUILD file has no executable lines for " ++ packagePath pkg
                       | null (packageBuildCommands pkg)
                       ]
            )
            packages

filterByLanguage :: String -> [Package] -> [Package]
filterByLanguage requested
    | requested == "all" = id
    | otherwise = filter (\pkg -> packageLanguage pkg == requested)

resolveDependencies :: [Package] -> IO DG.DirectedGraph
resolveDependencies packages = do
    aliasScopes <- buildAliasScopes packages
    let knownPackageNames = Set.fromList (map packageName packages)
    dependencyPairs <-
        forM packages $ \pkg -> do
            manifestDeps <- resolvePackageDeps aliasScopes pkg
            buildDeps <- readBuildToolDeps knownPackageNames pkg
            pure (pkg, sort (nub (manifestDeps ++ buildDeps)))
    pure $
        foldl
            (\graph (pkg, deps) ->
                foldl
                    (\inner dep -> DG.addEdge dep (packageName pkg) inner)
                    (DG.addNode (packageName pkg) graph)
                    deps
            )
            DG.empty
            dependencyPairs

buildAliasScopes :: [Package] -> IO (Map String (Map String String))
buildAliasScopes packages = do
    registrations <- foldM registerAlias Map.empty packages
    pure (Map.map (Map.mapMaybe selectIdentity) registrations)
  where
    registerAlias scopes pkg = do
        aliases <- packageAliases pkg
        let scope = dependencyScope (packageLanguage pkg)
        let scopeMap = Map.findWithDefault Map.empty scope scopes
        let updatedScope =
                foldl
                    (\aliasMap alias ->
                        Map.insertWith (++) alias [packageName pkg] aliasMap
                    )
                    scopeMap
                    aliases
        pure (Map.insert scope updatedScope scopes)

    selectIdentity identities =
        case (nub (filter (not . isProgramIdentity) identities), nub (filter isProgramIdentity identities)) of
            ([packageIdentity], _) -> Just packageIdentity
            ([], [programIdentity]) -> Just programIdentity
            _ -> Nothing

    isProgramIdentity identity =
        case wordsBy (== '/') identity of
            _language : "programs" : _name -> True
            _ -> False

dependencyScope :: String -> String
dependencyScope language
    | language `elem` ["csharp", "fsharp", "dotnet"] = "dotnet"
    | otherwise = language

packageAliases :: Package -> IO [String]
packageAliases pkg = do
    let dirName = map toLower (takeFileName (packagePath pkg))
    let kebab = map (\char -> if char == '_' then '-' else char) dirName
    let snake = map (\char -> if char == '-' then '_' else char) dirName
    let perlDistributionAliases = if packageLanguage pkg == "perl" then ["coding-adventures-" ++ dirName] else []
    let conventionalAliases =
            if packageLanguage pkg == "typescript"
                then [dirName, "@coding-adventures/" ++ dirName]
                else
                    [ dirName
                    , kebab
                    , snake
                    , "coding-adventures-" ++ kebab
                    , "coding_adventures_" ++ snake
                    ]
    let gradlePathAliases =
            if packageLanguage pkg `elem` ["java", "kotlin"]
                then [normalizeGradlePackagePath (packagePath pkg)]
                else []
    dotnetProjectAliases <-
        if packageLanguage pkg `elem` ["csharp", "fsharp", "dotnet"]
            then readDotnetProjectAliases pkg
            else pure []
    manifestNames <- exactManifestNames pkg
    pure
        (nub
            ( filter
                (not . null)
                ( map (map toLower)
                    ( conventionalAliases
                        ++ perlDistributionAliases
                        ++ gradlePathAliases
                        ++ dotnetProjectAliases
                        ++ manifestNames
                    )
                )
            )
        )

exactManifestNames :: Package -> IO [String]
exactManifestNames pkg = do
    let root = packagePath pkg
    let cabalNames = if packageLanguage pkg == "haskell" then readCabalPackageNames root else pure []
    let cargoNames = if packageLanguage pkg == "rust" then readCargoNames root else pure []
    let pythonNames = if packageLanguage pkg == "python" then readPyprojectNames root else pure []
    let goNames = if packageLanguage pkg == "go" then readGoModuleNames root else pure []
    let dartNames = if packageLanguage pkg == "dart" then readPubspecNames root else pure []
    let packageJsonNames = if packageLanguage pkg == "typescript" then readPackageJsonNames root else pure []
    let gemNames = if packageLanguage pkg == "ruby" then readGemspecNames root else pure []
    let perlNames = if packageLanguage pkg == "perl" then readPerlPackageNames root else pure []
    sequenceToList [cabalNames, cargoNames, pythonNames, goNames, dartNames, packageJsonNames, gemNames, perlNames]

sequenceToList :: [IO [String]] -> IO [String]
sequenceToList actions = fmap concat (sequence actions)

readDotnetProjectAliases :: Package -> IO [String]
readDotnetProjectAliases pkg =
    map normalizeDotnetProjectPath <$> rootDotnetProjectFiles pkg

readCabalPackageNames :: FilePath -> IO [String]
readCabalPackageNames root = do
    entries <- listDirectory root
    let cabalFiles = [root </> entry | entry <- entries, takeExtension entry == ".cabal"]
    case cabalFiles of
        [path] -> maybeToList <$> readSimpleFieldName path
        _ -> pure []

readCargoNames :: FilePath -> IO [String]
readCargoNames root = do
    let cargoPath = root </> "Cargo.toml"
    exists <- doesFileExist cargoPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict cargoPath
            pure (parseQuotedField "name" contents)

readPyprojectNames :: FilePath -> IO [String]
readPyprojectNames root = do
    let pyprojectPath = root </> "pyproject.toml"
    exists <- doesFileExist pyprojectPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict pyprojectPath
            pure (parseAssignmentField "name" contents)

readGoModuleNames :: FilePath -> IO [String]
readGoModuleNames root = do
    let goModPath = root </> "go.mod"
    exists <- doesFileExist goModPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict goModPath
            pure
                [ map toLower (trim (drop (length ("module" :: String)) stripped))
                | line <- lines contents
                , let stripped = trim line
                , "module " `isPrefixOf` stripped
                ]

readPubspecNames :: FilePath -> IO [String]
readPubspecNames root = do
    let pubspecPath = root </> "pubspec.yaml"
    exists <- doesFileExist pubspecPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict pubspecPath
            pure (mapMaybe dartRootName (lines contents))

dartRootName :: String -> Maybe String
dartRootName rawLine
    | length rawLine /= length (dropWhile (== ' ') rawLine) = Nothing
    | otherwise =
        case break (== ':') rawLine of
            (field, ':' : value)
                | map toLower (trim field) == "name"
                , let candidate = map toLower (trim (takeWhile (/= '#') value))
                , isDartPackageIdentifier candidate -> Just candidate
            _ -> Nothing

readPackageJsonNames :: FilePath -> IO [String]
readPackageJsonNames root = do
    let packageJson = root </> "package.json"
    manifest <- readPackageJsonObject packageJson
    pure
        ( case manifest >>= KeyMap.lookup (Key.fromString "name") of
            Just (String name) -> [map toLower (Text.unpack (Text.strip name))]
            _ -> []
        )

readGemspecNames :: FilePath -> IO [String]
readGemspecNames root = do
    entries <- listDirectory root
    let gemspecs = [root </> entry | entry <- entries, takeExtension entry == ".gemspec"]
    fmap concat $
        forM gemspecs $ \path -> do
            contents <- readFileStrict path
            pure
                [ map toLower (takeWhile (\char -> char /= '"' && char /= '\'') after)
                | line <- lines contents
                , let stripped = trim line
                , "spec.name" `isPrefixOf` stripped
                , quote <- ['"', '\'']
                , let suffix = dropWhile (/= quote) stripped
                , not (null suffix)
                , let after = drop 1 suffix
                ]

readPerlPackageNames :: FilePath -> IO [String]
readPerlPackageNames root = do
    let makefilePath = root </> "Makefile.PL"
    exists <- doesFileExist makefilePath
    if not exists
        then pure []
        else do
            contents <- readFileStrict makefilePath
            pure (perlPackageNames contents)

perlPackageNames :: String -> [String]
perlPackageNames = nub . mapMaybe (perlNamedFieldValue "NAME" . stripPerlComment) . lines

perlNamedFieldValue :: String -> String -> Maybe String
perlNamedFieldValue fieldName = search Nothing
  where
    search _ [] = Nothing
    search previous remaining@(character : rest)
        | fieldName `isPrefixOf` remaining
        , maybe True (not . isPerlIdentifierCharacter) previous
        , let afterField = drop (length fieldName) remaining
        , null afterField || not (isPerlIdentifierCharacter (head afterField)) =
            case trim afterField of
                '=' : '>' : rhs -> map toLower <$> listToMaybe (extractQuotedValues rhs)
                _ -> search (Just character) rest
        | otherwise = search (Just character) rest

isPerlIdentifierCharacter :: Char -> Bool
isPerlIdentifierCharacter character = isAlphaNum character || character == '_'

readSimpleFieldName :: FilePath -> IO (Maybe String)
readSimpleFieldName path = do
    contents <- readFileStrict path
    pure $
        listToMaybe
            [ map toLower (trim (drop 1 rest))
            | line <- lines contents
            , let stripped = trim line
            , "name" `isPrefixOf` map toLower stripped
            , let (field, rest) = break (== ':') stripped
            , map toLower field == "name"
            , not (null rest)
            ]

parseQuotedField :: String -> String -> [String]
parseQuotedField fieldName contents =
    nub
        [ map toLower value
        | line <- lines contents
        , let stripped = trim line
        , let (field, rest) = break (== '=') stripped
        , trim field == fieldName
        , not (null rest)
        , quote <- ['"', '\'']
        , let rhs = trim (drop 1 rest)
        , not (null rhs)
        , take 1 rhs == [quote]
        , let value = takeWhile (/= quote) (drop 1 rhs)
        , not (null value)
        ]

parseAssignmentField :: String -> String -> [String]
parseAssignmentField = parseQuotedField

resolvePackageDeps :: Map String (Map String String) -> Package -> IO [String]
resolvePackageDeps aliasScopes pkg = do
    tokens <- readManifestTokens pkg
    let scope = dependencyScope (packageLanguage pkg)
    let aliases = Map.findWithDefault Map.empty scope aliasScopes
    pure
        ( nub
            [ resolved
            | token <- tokens
            , Just resolved <- [Map.lookup token aliases]
            , resolved /= packageName pkg
            ]
        )

readBuildToolDeps :: Set String -> Package -> IO [String]
readBuildToolDeps knownPackageNames pkg = do
    contents <- readFileStrict (packageBuildFile pkg)
    pure
        ( sort
            ( nub
                [ dependency
                | line <- lines contents
                , dependency <- parseBuildToolDependencyLine line
                , dependency /= packageName pkg
                , dependency `Set.member` knownPackageNames
                ]
            )
        )

parseBuildToolDependencyLine :: String -> [String]
parseBuildToolDependencyLine line =
    case trim line of
        '#' : comment ->
            let directive = trim comment
                marker = "build-tool:"
                lowered = map toLower directive
             in if marker `isPrefixOf` lowered
                    then
                        let assignment = drop (length marker) directive
                            (field, rest) = break (== '=') assignment
                         in if map toLower (trim field) == "deps" && not (null rest)
                                then wordsBy (`elem` ", \t") (drop 1 rest)
                                else []
                    else []
        _ -> []

readManifestTokens :: Package -> IO [String]
readManifestTokens pkg
    | packageLanguage pkg == "lua" = readLuaDependencyTokens pkg
    | packageLanguage pkg == "elixir" = readElixirDependencyTokens pkg
    | packageLanguage pkg == "dart" = readDartDependencyTokens pkg
    | packageLanguage pkg == "go" = readGoDependencyTokens pkg
    | packageLanguage pkg == "haskell" = readCabalDependencyTokens pkg
    | packageLanguage pkg == "perl" = readPerlDependencyTokens pkg
    | packageLanguage pkg == "python" = readPythonDependencyTokens pkg
    | packageLanguage pkg == "ruby" = readRubyDependencyTokens pkg
    | packageLanguage pkg == "rust" = readRustDependencyTokens pkg
    | packageLanguage pkg == "swift" = readSwiftDependencyTokens pkg
    | packageLanguage pkg == "typescript" = readTypescriptDependencyTokens pkg
    | packageLanguage pkg `elem` ["java", "kotlin"] = readGradleDependencyTokens pkg
    | packageLanguage pkg `elem` ["csharp", "fsharp", "dotnet"] = readDotnetDependencyTokens pkg
    | otherwise = readGenericManifestTokens pkg

readPackageJsonObject :: FilePath -> IO (Maybe (KeyMap.KeyMap Value))
readPackageJsonObject path = do
    exists <- doesFileExist path
    if not exists
        then pure Nothing
        else do
            bytes <- BS.readFile path
            pure
                ( case eitherDecodeStrict' bytes of
                    Right (Object manifest) -> Just manifest
                    _ -> Nothing
                )

readTypescriptDependencyTokens :: Package -> IO [String]
readTypescriptDependencyTokens pkg = do
    manifest <- readPackageJsonObject (packagePath pkg </> "package.json")
    pure
        ( nub
            ( concatMap
                (typescriptDependencyNames manifest)
                ["dependencies", "devDependencies"]
            )
        )

typescriptDependencyNames :: Maybe (KeyMap.KeyMap Value) -> String -> [String]
typescriptDependencyNames manifest fieldName =
    case manifest >>= KeyMap.lookup (Key.fromString fieldName) of
        Just (Object dependencies) ->
            sort
                ( map
                    (map toLower . Text.unpack . Text.strip . Key.toText)
                    (KeyMap.keys dependencies)
                )
        _ -> []

readDartDependencyTokens :: Package -> IO [String]
readDartDependencyTokens pkg = do
    let manifestPath = packagePath pkg </> "pubspec.yaml"
    exists <- doesFileExist manifestPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict manifestPath
            pure (dartDependencyTokens contents)

dartDependencyTokens :: String -> [String]
dartDependencyTokens = nub . collect False Nothing . lines
  where
    collect _ _ [] = []
    collect inBlock directIndent (rawLine : rest)
        | null stripped || "#" `isPrefixOf` stripped = collect inBlock directIndent rest
        | indentation == 0 =
            collect
                (stripped `elem` ["dependencies:", "dev_dependencies:"])
                Nothing
                rest
        | not inBlock = collect False Nothing rest
        | otherwise =
            let expectedIndent = fromMaybe indentation directIndent
                candidate = if indentation == expectedIndent then dartDependencyKey stripped else Nothing
             in maybeToList candidate ++ collect True (Just expectedIndent) rest
      where
        stripped = trim rawLine
        indentation = length (takeWhile (== ' ') rawLine)

dartDependencyKey :: String -> Maybe String
dartDependencyKey line =
    case break (== ':') line of
        (field, ':' : _)
            | let candidate = map toLower (trim field)
            , isDartPackageIdentifier candidate -> Just candidate
        _ -> Nothing

isDartPackageIdentifier :: String -> Bool
isDartPackageIdentifier [] = False
isDartPackageIdentifier (first : rest) =
    first >= 'a'
        && first <= 'z'
        && all
            (\character ->
                (character >= 'a' && character <= 'z')
                    || (character >= '0' && character <= '9')
                    || character == '_'
            )
            rest

readElixirDependencyTokens :: Package -> IO [String]
readElixirDependencyTokens pkg = do
    let manifestPath = packagePath pkg </> "mix.exs"
    exists <- doesFileExist manifestPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict manifestPath
            pure (elixirDependencyTokens contents)

elixirDependencyTokens :: String -> [String]
elixirDependencyTokens contents =
    let uncommented = unlines (map stripElixirComment (lines contents))
     in nub
            ( mapMaybe elixirPathDependencyAtom
                (concatMap elixirTupleBodies (elixirDependencyBodies uncommented))
            )

elixirDependencyBodies :: String -> [String]
elixirDependencyBodies contents =
    elixirDirectDepsBodies contents
        ++ [functionBody | let functionBody = elixirDepsBody contents, not (null (trim functionBody))]

elixirDirectDepsBodies :: String -> [String]
elixirDirectDepsBodies = collect . lines
  where
    collect [] = []
    collect (line : rest) =
        case elixirFieldRemainder "deps:" line of
            Just remainder
                | "[" `isPrefixOf` trim remainder ->
                    let value = trim remainder
                        (bodyLines, remainingLines) = collectBracketed (elixirBracketDelta value) [value] rest
                     in unlines (reverse bodyLines) : collect remainingLines
            _ -> collect rest

    collectBracketed depth body remaining
        | depth <= 0 = (body, remaining)
    collectBracketed _ body [] = (body, [])
    collectBracketed depth body (line : rest) =
        collectBracketed (depth + elixirBracketDelta line) (line : body) rest

elixirFieldRemainder :: String -> String -> Maybe String
elixirFieldRemainder target = go False False
  where
    go _ _ [] = Nothing
    go insideString escaped remaining@(character : rest)
        | escaped = go insideString False rest
        | insideString && character == '\\' = go True True rest
        | character == '"' = go (not insideString) False rest
        | not insideString && target `isPrefixOf` remaining = Just (drop (length target) remaining)
        | otherwise = go insideString False rest

elixirBracketDelta :: String -> Int
elixirBracketDelta = go False False 0
  where
    go _ _ total [] = total
    go insideString escaped total (character : rest)
        | escaped = go insideString False total rest
        | insideString && character == '\\' = go True True total rest
        | character == '"' = go (not insideString) False total rest
        | not insideString && character == '[' = go False False (total + 1) rest
        | not insideString && character == ']' = go False False (total - 1) rest
        | otherwise = go insideString False total rest

elixirDepsBody :: String -> String
elixirDepsBody = unlines . collect False . lines
  where
    collect _ [] = []
    collect False (line : rest) =
        case elixirDepsShorthand line of
            Just body -> [body]
            Nothing ->
                if trim line `elem` ["defp deps do", "def deps do"]
                    then collect True rest
                    else collect False rest
    collect True (line : rest)
        | trim line == "end" = []
        | otherwise = line : collect True rest

elixirDepsShorthand :: String -> Maybe String
elixirDepsShorthand line =
    let stripped = trim line
        (_function, afterComma) = break (== ',') stripped
        shorthand = trim (drop 1 afterComma)
     in if ("defp deps," `isPrefixOf` stripped || "def deps," `isPrefixOf` stripped)
            && "do:" `isPrefixOf` shorthand
            then Just (drop 3 shorthand)
            else Nothing

stripElixirComment :: String -> String
stripElixirComment = go False False
  where
    go _ _ [] = []
    go insideString escaped (character : rest)
        | escaped = character : go insideString False rest
        | insideString && character == '\\' = character : go True True rest
        | character == '"' = character : go (not insideString) False rest
        | character == '#' && not insideString = []
        | otherwise = character : go insideString False rest

elixirTupleBodies :: String -> [String]
elixirTupleBodies text =
    case dropWhile (/= '{') text of
        [] -> []
        _openingBrace : afterOpening ->
            let (tupleBody, remainder) = break (== '}') afterOpening
             in tupleBody : elixirTupleBodies (drop 1 remainder)

elixirPathDependencyAtom :: String -> Maybe String
elixirPathDependencyAtom tupleBody =
    let (atomField, options) = break (== ',') tupleBody
        strippedAtom = trim atomField
        atom = takeWhile (\character -> isAlphaNum character || character == '_') (dropWhile (== ':') strippedAtom)
     in if null options
            || null atom
            || take 1 strippedAtom /= ":"
            || not (any isElixirPathField (splitCargoInlineFields (drop 1 options)))
            then Nothing
            else Just (map toLower atom)

isElixirPathField :: String -> Bool
isElixirPathField field =
    let (key, assignment) = break (== ':') field
        value = dropWhile (`elem` [' ', '\t']) (drop 1 assignment)
     in map toLower (trim key) == "path"
            && not (null assignment)
            && take 1 value == "\""
            && not (null (takeWhile (/= '"') (drop 1 value)))

readGoDependencyTokens :: Package -> IO [String]
readGoDependencyTokens pkg = do
    let manifestPath = packagePath pkg </> "go.mod"
    exists <- doesFileExist manifestPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict manifestPath
            pure (goDependencyTokens contents)

goDependencyTokens :: String -> [String]
goDependencyTokens = nub . collect False . lines
  where
    collect _ [] = []
    collect inside (rawLine : rest) =
        let fields = words (trim (stripGoComment rawLine))
         in if inside
                then
                    case fields of
                        [")"] -> collect False rest
                        modulePath : _ -> map toLower modulePath : collect True rest
                        [] -> collect True rest
                else
                    case fields of
                        "require" : "(" : _ -> collect True rest
                        "require" : modulePath : _ -> map toLower modulePath : collect False rest
                        _ -> collect False rest

stripGoComment :: String -> String
stripGoComment [] = []
stripGoComment ('/' : '/' : _) = []
stripGoComment (character : rest) = character : stripGoComment rest

readCabalDependencyTokens :: Package -> IO [String]
readCabalDependencyTokens pkg = do
    entries <- listDirectory (packagePath pkg)
    let cabalPaths =
            sort
                [ packagePath pkg </> entry
                | entry <- entries
                , map toLower (takeExtension entry) == ".cabal"
                ]
    case cabalPaths of
        [path] -> do
            contents <- readFileStrict path
            pure (cabalDependencyTokens contents)
        _ -> pure []

cabalDependencyTokens :: String -> [String]
cabalDependencyTokens = collect False . lines
  where
    collect _ [] = []
    collect inside (rawLine : rest) =
        let stripped = trim (stripCabalComment rawLine)
            (field, assignment) = break (== ':') stripped
            fieldName = map toLower (trim field)
            startsField = not (null assignment) && isCabalFieldName fieldName
            indented =
                case rawLine of
                    first : _ -> first `elem` [' ', '\t']
                    [] -> False
         in if startsField && fieldName == "build-depends"
                then tokenize (drop 1 assignment) ++ collect True rest
                else
                    if inside
                        then
                            if null stripped || startsField || not indented
                                then collect False rest
                                else tokenize stripped ++ collect True rest
                        else collect False rest

isCabalFieldName :: String -> Bool
isCabalFieldName fieldName =
    not (null fieldName)
        && isAlpha (head fieldName)
        && all (\character -> isAlphaNum character || character == '-') fieldName

stripCabalComment :: String -> String
stripCabalComment [] = []
stripCabalComment ('-' : '-' : _) = []
stripCabalComment (character : rest) = character : stripCabalComment rest

readPythonDependencyTokens :: Package -> IO [String]
readPythonDependencyTokens pkg = do
    let pyprojectPath = packagePath pkg </> "pyproject.toml"
    exists <- doesFileExist pyprojectPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict pyprojectPath
            pure (pythonDependencyTokens contents)

pythonDependencyTokens :: String -> [String]
pythonDependencyTokens = nub . mapMaybe pythonDistributionToken . pythonDependencyValues

pythonDependencyValues :: String -> [String]
pythonDependencyValues = collect False False . lines
  where
    collect _ _ [] = []
    collect inProject inDependencies (rawLine : rest) =
        let uncommented = stripTomlComment rawLine
            stripped = trim uncommented
         in case tomlSectionName stripped of
                Just sectionName -> collect (sectionName == "project") False rest
                Nothing
                    | inDependencies ->
                        extractQuotedValues uncommented
                            ++ collect inProject (not (hasUnquotedCharacter ']' uncommented)) rest
                    | inProject ->
                        case pythonDependencyArrayRemainder stripped of
                            Nothing -> collect True False rest
                            Just remainder ->
                                extractQuotedValues remainder
                                    ++ collect True (not (hasUnquotedCharacter ']' remainder)) rest
                    | otherwise -> collect False False rest

tomlSectionName :: String -> Maybe String
tomlSectionName stripped
    | length stripped >= 3
        && head stripped == '['
        && last stripped == ']'
        && take 2 stripped /= "[[" =
        Just (map toLower (trim (init (tail stripped))))
    | otherwise = Nothing

pythonDependencyArrayRemainder :: String -> Maybe String
pythonDependencyArrayRemainder stripped =
    let (field, assignment) = break (== '=') stripped
        afterAssignment = drop 1 assignment
        arrayRemainder = dropWhile (/= '[') afterAssignment
     in if map toLower (trim field) == "dependencies"
            && not (null assignment)
            && not (null arrayRemainder)
            then Just arrayRemainder
            else Nothing

pythonDistributionToken :: String -> Maybe String
pythonDistributionToken value =
    let name = takeWhile isPythonDistributionCharacter (dropWhile (`elem` [' ', '\t']) value)
     in if null name then Nothing else Just (normalizePythonDistributionName name)

isPythonDistributionCharacter :: Char -> Bool
isPythonDistributionCharacter character =
    isAlphaNum character || character `elem` ['-', '_', '.']

normalizePythonDistributionName :: String -> String
normalizePythonDistributionName = collapseSeparators False
  where
    collapseSeparators _ [] = []
    collapseSeparators previousWasSeparator (character : rest)
        | character `elem` ['-', '_', '.'] =
            if previousWasSeparator
                then collapseSeparators True rest
                else '-' : collapseSeparators True rest
        | otherwise = toLower character : collapseSeparators False rest

stripTomlComment :: String -> String
stripTomlComment = go Nothing
  where
    go _ [] = []
    go Nothing ('#' : _) = []
    go Nothing (character : rest)
        | character `elem` ['"', '\''] = character : go (Just character) rest
        | otherwise = character : go Nothing rest
    go (Just quote) ('\\' : escaped : rest)
        | quote == '"' = '\\' : escaped : go (Just quote) rest
    go (Just quote) (character : rest)
        | character == quote = character : go Nothing rest
        | otherwise = character : go (Just quote) rest

hasUnquotedCharacter :: Char -> String -> Bool
hasUnquotedCharacter target = go Nothing
  where
    go _ [] = False
    go Nothing (character : rest)
        | character == target = True
        | character `elem` ['"', '\''] = go (Just character) rest
        | otherwise = go Nothing rest
    go (Just quote) ('\\' : _escaped : rest)
        | quote == '"' = go (Just quote) rest
    go (Just quote) (character : rest)
        | character == quote = go Nothing rest
        | otherwise = go (Just quote) rest

readRustDependencyTokens :: Package -> IO [String]
readRustDependencyTokens pkg = do
    let cargoPath = packagePath pkg </> "Cargo.toml"
    exists <- doesFileExist cargoPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict cargoPath
            pure (rustDependencyTokens contents)

rustDependencyTokens :: String -> [String]
rustDependencyTokens = nub . collect False . lines
  where
    collect _ [] = []
    collect inDependencies (rawLine : rest) =
        let stripped = trim (stripTomlComment rawLine)
         in case tomlSectionName stripped of
                Just sectionName -> collect (sectionName == "dependencies") rest
                Nothing ->
                    if inDependencies
                        then maybeToList (rustDependencyToken stripped) ++ collect True rest
                        else collect False rest

rustDependencyToken :: String -> Maybe String
rustDependencyToken stripped =
    let (dependencyKey, assignment) = break (== '=') stripped
        dependencyValue = drop 1 assignment
        normalizedKey = map toLower (trim dependencyKey)
        resolvedKey =
            map toLower
                (fromMaybe normalizedKey (cargoInlineStringValue "package" dependencyValue))
     in if null assignment
            || null normalizedKey
            || not (hasCargoPathAssignment dependencyValue)
            then Nothing
            else Just resolvedKey

cargoInlineStringValue :: String -> String -> Maybe String
cargoInlineStringValue fieldName value =
    listToMaybe
        [ quotedValue
        | field <- splitCargoInlineFields value
        , let withoutOpeningBrace = dropWhile (`elem` [' ', '\t', '{']) field
        , let (key, assignment) = break (== '=') withoutOpeningBrace
        , map toLower (trim key) == fieldName
        , not (null assignment)
        , quotedValue <- take 1 (extractQuotedValues (drop 1 assignment))
        ]

splitCargoInlineFields :: String -> [String]
splitCargoInlineFields = go Nothing []
  where
    go _ current [] = [reverse current]
    go Nothing current (',' : rest) = reverse current : go Nothing [] rest
    go Nothing current (character : rest)
        | character `elem` ['"', '\''] = go (Just character) (character : current) rest
        | otherwise = go Nothing (character : current) rest
    go (Just quote) current ('\\' : escaped : rest)
        | quote == '"' = go (Just quote) (escaped : '\\' : current) rest
    go (Just quote) current (character : rest)
        | character == quote = go Nothing (character : current) rest
        | otherwise = go (Just quote) (character : current) rest

hasCargoPathAssignment :: String -> Bool
hasCargoPathAssignment = scan Nothing . map toLower . hideTomlStringContents
  where
    scan _ [] = False
    scan previous remaining@(character : rest)
        | isCargoPathFieldStart previous remaining = True
        | otherwise = scan (Just character) rest

    isCargoPathFieldStart previous remaining =
        "path" `isPrefixOf` remaining
            && maybe True (not . isCargoKeyCharacter) previous
            && case dropWhile (`elem` [' ', '\t']) (drop 4 remaining) of
                '=' : _ -> True
                _ -> False

isCargoKeyCharacter :: Char -> Bool
isCargoKeyCharacter character = isAlphaNum character || character `elem` ['_', '-']

hideTomlStringContents :: String -> String
hideTomlStringContents = go Nothing
  where
    go _ [] = []
    go Nothing (character : rest)
        | character `elem` ['"', '\''] = ' ' : go (Just character) rest
        | otherwise = character : go Nothing rest
    go (Just quote) ('\\' : _escaped : rest)
        | quote == '"' = ' ' : ' ' : go (Just quote) rest
    go (Just quote) (character : rest)
        | character == quote = ' ' : go Nothing rest
        | otherwise = ' ' : go (Just quote) rest

readRubyDependencyTokens :: Package -> IO [String]
readRubyDependencyTokens pkg = do
    entries <- listDirectory (packagePath pkg)
    let gemspecPaths =
            sort
                [ packagePath pkg </> entry
                | entry <- entries
                , map toLower (takeExtension entry) == ".gemspec"
                ]
    case listToMaybe gemspecPaths of
        Nothing -> pure []
        Just gemspecPath -> do
            contents <- readFileStrict gemspecPath
            pure (rubyDependencyTokens contents)

rubyDependencyTokens :: String -> [String]
rubyDependencyTokens contents =
    case rubySpecificationReceiver contents of
        Nothing -> []
        Just receiver ->
            nub (mapMaybe (rubyDependencyToken receiver) (lines contents))

rubySpecificationReceiver :: String -> Maybe String
rubySpecificationReceiver = listToMaybe . mapMaybe receiverFromLine . lines
  where
    receiverFromLine rawLine =
        let stripped = trim rawLine
            declaration = "Gem::Specification.new"
            afterDeclaration = drop (length declaration) stripped
            afterOpeningBar = dropWhile (/= '|') afterDeclaration
            receiver = takeWhile (/= '|') (drop 1 afterOpeningBar)
         in if declaration `isPrefixOf` stripped
                && "do" `isPrefixOf` trim afterDeclaration
                && not (null afterOpeningBar)
                && isRubyIdentifier receiver
                then Just receiver
                else Nothing

isRubyIdentifier :: String -> Bool
isRubyIdentifier [] = False
isRubyIdentifier (first : rest) =
    (isAlpha first || first == '_')
        && all (\character -> isAlphaNum character || character == '_') rest

rubyDependencyToken :: String -> String -> Maybe String
rubyDependencyToken receiver rawLine =
    listToMaybe
        [ map toLower gemName
        | methodName <- ["add_dependency", "add_runtime_dependency"]
        , let prefix = receiver ++ "." ++ methodName
        , let stripped = trim rawLine
        , prefix `isPrefixOf` stripped
        , let remainder = drop (length prefix) stripped
        , rubyCallBoundary remainder
        , gemName <- take 1 (extractQuotedValues (dropRubyCallOpening remainder))
        ]

rubyCallBoundary :: String -> Bool
rubyCallBoundary [] = False
rubyCallBoundary (character : _) = character `elem` [' ', '\t', '(']

dropRubyCallOpening :: String -> String
dropRubyCallOpening value =
    case trim value of
        '(' : rest -> trim rest
        rest -> rest

readPerlDependencyTokens :: Package -> IO [String]
readPerlDependencyTokens pkg = do
    let cpanfilePath = packagePath pkg </> "cpanfile"
    exists <- doesFileExist cpanfilePath
    if not exists
        then pure []
        else do
            contents <- readFileStrict cpanfilePath
            pure (perlDependencyTokens contents)

perlDependencyTokens :: String -> [String]
perlDependencyTokens = nub . collect 0 . lines
  where
    collect _ [] = []
    collect blockDepth (rawLine : rest) =
        let uncommented = stripPerlComment rawLine
            token = if blockDepth == 0 then maybeToList (perlRequiresToken uncommented) else []
            structure = hidePerlStringContents uncommented
            nextDepth = max 0 (blockDepth + countCharacter '{' structure - countCharacter '}' structure)
         in token ++ collect nextDepth rest

perlRequiresToken :: String -> Maybe String
perlRequiresToken rawLine =
    let stripped = trim rawLine
        keyword = "requires"
        remainder = drop (length keyword) stripped
        argument = trim remainder
     in if keyword `isPrefixOf` stripped
            && not (null remainder)
            && head remainder `elem` [' ', '\t']
            && not (null argument)
            && head argument `elem` ['"', '\'']
            then map toLower <$> listToMaybe (extractQuotedValues argument)
            else Nothing

stripPerlComment :: String -> String
stripPerlComment = go Nothing
  where
    go _ [] = []
    go Nothing ('#' : _) = []
    go Nothing (character : rest)
        | character `elem` ['"', '\''] = character : go (Just character) rest
        | otherwise = character : go Nothing rest
    go (Just quote) ('\\' : escaped : rest) = '\\' : escaped : go (Just quote) rest
    go (Just quote) (character : rest)
        | character == quote = character : go Nothing rest
        | otherwise = character : go (Just quote) rest

hidePerlStringContents :: String -> String
hidePerlStringContents = go Nothing
  where
    go _ [] = []
    go Nothing (character : rest)
        | character `elem` ['"', '\''] = ' ' : go (Just character) rest
        | otherwise = character : go Nothing rest
    go (Just quote) ('\\' : _escaped : rest) = ' ' : ' ' : go (Just quote) rest
    go (Just quote) (character : rest)
        | character == quote = ' ' : go Nothing rest
        | otherwise = ' ' : go (Just quote) rest

countCharacter :: Char -> String -> Int
countCharacter target = length . filter (== target)

readSwiftDependencyTokens :: Package -> IO [String]
readSwiftDependencyTokens pkg = do
    let manifestPath = packagePath pkg </> "Package.swift"
    exists <- doesFileExist manifestPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict manifestPath
            pure (swiftDependencyTokens contents)

swiftDependencyTokens :: String -> [String]
swiftDependencyTokens = nub . mapMaybe swiftPathToken . swiftPackagePathValues . stripSwiftComments

swiftPackagePathValues :: String -> [String]
swiftPackagePathValues = collect
  where
    collect [] = []
    collect ('"' : rest) = collect (dropSwiftString rest)
    collect source@(_character : rest)
        | ".package" `isPrefixOf` source =
            case parseSwiftPackagePath (drop (length (".package" :: String)) source) of
                Just (path, remaining) -> path : collect remaining
                Nothing -> collect rest
        | otherwise = collect rest

parseSwiftPackagePath :: String -> Maybe (String, String)
parseSwiftPackagePath source = do
    afterOpening <- consumeSwiftCharacter '(' (dropWhile isSpace source)
    let afterPathWhitespace = dropWhile isSpace afterOpening
    afterPath <- stripSwiftFieldName "path" afterPathWhitespace
    afterColon <- consumeSwiftCharacter ':' (dropWhile isSpace afterPath)
    parseSwiftQuotedString (dropWhile isSpace afterColon)

stripSwiftFieldName :: String -> String -> Maybe String
stripSwiftFieldName fieldName source
    | fieldName `isPrefixOf` source =
        let remainder = drop (length fieldName) source
         in if null remainder || not (isAlphaNum (head remainder) || head remainder `elem` ['_', '-'])
                then Just remainder
                else Nothing
    | otherwise = Nothing

consumeSwiftCharacter :: Char -> String -> Maybe String
consumeSwiftCharacter expected (character : rest)
    | character == expected = Just rest
consumeSwiftCharacter _ _ = Nothing

parseSwiftQuotedString :: String -> Maybe (String, String)
parseSwiftQuotedString ('"' : rest) = collect [] rest
  where
    collect _ [] = Nothing
    collect value ('\\' : escaped : remaining) = collect (escaped : '\\' : value) remaining
    collect value ('"' : remaining) = Just (reverse value, remaining)
    collect value (character : remaining) = collect (character : value) remaining
parseSwiftQuotedString _ = Nothing

dropSwiftString :: String -> String
dropSwiftString [] = []
dropSwiftString ('\\' : _escaped : rest) = dropSwiftString rest
dropSwiftString ('"' : rest) = rest
dropSwiftString (_character : rest) = dropSwiftString rest

stripSwiftComments :: String -> String
stripSwiftComments = collect 0 False
  where
    collect :: Int -> Bool -> String -> String
    collect _ _ [] = []
    collect blockDepth inString ('/' : '*' : rest)
        | not inString = ' ' : ' ' : collect (blockDepth + 1) False rest
    collect blockDepth inString ('*' : '/' : rest)
        | not inString && blockDepth > 0 = ' ' : ' ' : collect (blockDepth - 1) False rest
    collect blockDepth inString ('/' : '/' : rest)
        | not inString && blockDepth == 0 = ' ' : ' ' : stripSwiftLineComment rest
    collect blockDepth True ('\\' : escaped : rest) = '\\' : escaped : collect blockDepth True rest
    collect blockDepth inString ('"' : rest)
        | blockDepth == 0 = '"' : collect blockDepth (not inString) rest
    collect blockDepth inString (character : rest)
        | blockDepth > 0 = preserveSwiftLineBreak character : collect blockDepth inString rest
        | otherwise = character : collect blockDepth inString rest

    stripSwiftLineComment [] = []
    stripSwiftLineComment ('\n' : rest) = '\n' : collect 0 False rest
    stripSwiftLineComment ('\r' : rest) = '\r' : collect 0 False rest
    stripSwiftLineComment (_character : rest) = ' ' : stripSwiftLineComment rest

    preserveSwiftLineBreak character
        | character `elem` ['\n', '\r'] = character
        | otherwise = ' '

swiftPathToken :: String -> Maybe String
swiftPathToken path
    | isPortableAbsoluteSwiftPath path = Nothing
    | not (null path) && last path `elem` ['/', '\\'] = Nothing
    | null components = Nothing
    | lastComponent `elem` ["", ".", ".."] = Nothing
    | otherwise = Just (map toLower lastComponent)
  where
    components = wordsBy (`elem` ['/', '\\']) path
    lastComponent = last components

isPortableAbsoluteSwiftPath :: String -> Bool
isPortableAbsoluteSwiftPath [] = False
isPortableAbsoluteSwiftPath path@(first : rest) =
    isAbsolute path
        || first `elem` ['/', '\\']
        || case rest of
            ':' : _ -> isAlpha first
            _ -> False

readGradleDependencyTokens :: Package -> IO [String]
readGradleDependencyTokens pkg = do
    let settingsPath = packagePath pkg </> "settings.gradle.kts"
    exists <- doesFileExist settingsPath
    if not exists
        then pure []
        else do
            contents <- readFileStrict settingsPath
            pure
                ( nub
                    ( mapMaybe
                        (gradlePathToken pkg)
                        (gradleIncludeBuildValues (stripSwiftComments contents))
                    )
                )

gradleIncludeBuildValues :: String -> [String]
gradleIncludeBuildValues = collect Nothing
  where
    collect _ [] = []
    collect _ ('"' : rest) = collect (Just '"') (dropSwiftString rest)
    collect previous source@(character : rest)
        | gradleIdentifierBoundary previous source "includeBuild" =
            case parseGradleIncludeBuild (drop (length ("includeBuild" :: String)) source) of
                Just (path, remaining) -> path : collect Nothing remaining
                Nothing -> collect (Just character) rest
        | otherwise = collect (Just character) rest

gradleIdentifierBoundary :: Maybe Char -> String -> String -> Bool
gradleIdentifierBoundary previous source identifier =
    identifier `isPrefixOf` source
        && maybe True (not . isGradleIdentifierCharacter) previous
        && case drop (length identifier) source of
            next : _ -> not (isGradleIdentifierCharacter next)
            [] -> True

isGradleIdentifierCharacter :: Char -> Bool
isGradleIdentifierCharacter character = isAlphaNum character || character == '_'

parseGradleIncludeBuild :: String -> Maybe (String, String)
parseGradleIncludeBuild source = do
    afterOpening <- consumeSwiftCharacter '(' (dropWhile isSpace source)
    (path, afterString) <- parseSwiftQuotedString (dropWhile isSpace afterOpening)
    afterClosing <- consumeSwiftCharacter ')' (dropWhile isSpace afterString)
    pure (path, afterClosing)

gradlePathToken :: Package -> String -> Maybe String
gradlePathToken pkg path
    | null path = Nothing
    | '\\' `elem` path = Nothing
    | isPortableAbsoluteSwiftPath path = Nothing
    | otherwise =
        Just
            ( normalizeGradlePackagePath
                (packagePath pkg </> map portableSeparator path)
            )
  where
    portableSeparator character
        | character == '/' = pathSeparator
        | otherwise = character

normalizeGradlePackagePath :: FilePath -> String
normalizeGradlePackagePath =
    map toLower
        . joinPath
        . reverse
        . foldl collapse []
        . splitDirectories
        . normalise
  where
    collapse stack "." = stack
    collapse (top : rest) ".."
        | top /= ".." && not (isAbsolute top) = rest
    collapse stack component = component : stack

readDotnetDependencyTokens :: Package -> IO [String]
readDotnetDependencyTokens pkg = do
    projectFiles <- rootDotnetProjectFiles pkg
    fmap (sort . nub . concat) $
        forM projectFiles $ \projectFile -> do
            contents <- readFileStrict projectFile
            pure
                ( mapMaybe
                    (dotnetProjectPathToken projectFile)
                    (dotnetProjectReferenceIncludes contents)
                )

rootDotnetProjectFiles :: Package -> IO [FilePath]
rootDotnetProjectFiles pkg = do
    entries <- listDirectory (packagePath pkg)
    pure
        ( sort
            [ packagePath pkg </> entry
            | entry <- entries
            , map toLower (takeExtension entry) `elem` [".csproj", ".fsproj"]
            ]
        )

dotnetProjectReferenceIncludes :: String -> [String]
dotnetProjectReferenceIncludes = collect
  where
    collect source =
        case dropWhile (/= '<') source of
            [] -> []
            markup
                | "<!--" `isPrefixOf` markup -> collect (dropThroughXML "-->" (drop 4 markup))
                | "<![CDATA[" `isPrefixOf` markup -> collect (dropThroughXML "]]>" (drop 9 markup))
                | "<?" `isPrefixOf` markup -> collect (dropThroughXML "?>" (drop 2 markup))
                | "<!" `isPrefixOf` markup -> collect (dropThroughXML ">" (drop 2 markup))
                | otherwise ->
                    case parseXMLStartTag markup of
                        Just ("ProjectReference", attributes, remaining) ->
                            case xmlLiteralAttribute "Include" attributes of
                                Just include -> include : collect remaining
                                Nothing -> collect remaining
                        Just (_, _, remaining) -> collect remaining
                        Nothing -> collect (drop 1 markup)

dropThroughXML :: String -> String -> String
dropThroughXML marker = go
  where
    go [] = []
    go source@(_ : rest)
        | marker `isPrefixOf` source = drop (length marker) source
        | otherwise = go rest

parseXMLStartTag :: String -> Maybe (String, String, String)
parseXMLStartTag ('<' : first : rest)
    | first `notElem` ['/', '!', '?'] = do
        let (name, afterName) = span isXMLNameCharacter (first : rest)
        if null name
            then Nothing
            else do
                (attributes, remaining) <- takeXMLTagAttributes Nothing [] afterName
                pure (name, attributes, remaining)
parseXMLStartTag _ = Nothing

takeXMLTagAttributes :: Maybe Char -> String -> String -> Maybe (String, String)
takeXMLTagAttributes _ _ [] = Nothing
takeXMLTagAttributes quote collected (character : rest) =
    case quote of
        Just delimiter
            | character == delimiter ->
                takeXMLTagAttributes Nothing (character : collected) rest
            | otherwise ->
                takeXMLTagAttributes quote (character : collected) rest
        Nothing
            | character `elem` ['\'', '"'] ->
                takeXMLTagAttributes (Just character) (character : collected) rest
            | character == '>' -> Just (reverse collected, rest)
            | otherwise -> takeXMLTagAttributes Nothing (character : collected) rest

isXMLNameCharacter :: Char -> Bool
isXMLNameCharacter character =
    isAlphaNum character || character `elem` ['_', ':', '-', '.']

xmlLiteralAttribute :: String -> String -> Maybe String
xmlLiteralAttribute wanted = collect . dropWhile isSpace
  where
    collect [] = Nothing
    collect ('/' : _) = Nothing
    collect source =
        let (name, afterName) = span isXMLNameCharacter source
            assignment = dropWhile isSpace afterName
         in if null name
                then collect (dropWhile isSpace (drop 1 source))
                else case assignment of
                    '=' : rawValue ->
                        case parseXMLQuotedValue (dropWhile isSpace rawValue) of
                            Just (value, remaining)
                                | name == wanted -> Just value
                                | otherwise -> collect (dropWhile isSpace remaining)
                            Nothing -> Nothing
                    _ -> collect assignment

parseXMLQuotedValue :: String -> Maybe (String, String)
parseXMLQuotedValue (quote : rest)
    | quote `elem` ['\'', '"'] =
        case break (== quote) rest of
            (value, _ : remaining) -> Just (value, remaining)
            _ -> Nothing
parseXMLQuotedValue _ = Nothing

dotnetProjectPathToken :: FilePath -> String -> Maybe String
dotnetProjectPathToken projectFile path
    | null path = Nothing
    | any (`elem` ("*?#&" :: String)) path = Nothing
    | "$(" `isInfixOf` path = Nothing
    | isPortableAbsoluteSwiftPath path = Nothing
    | otherwise =
        Just
            ( normalizeDotnetProjectPath
                (takeDirectory projectFile </> map portableSeparator path)
            )
  where
    portableSeparator character
        | character `elem` ['/', '\\'] = pathSeparator
        | otherwise = character

normalizeDotnetProjectPath :: FilePath -> String
normalizeDotnetProjectPath = normalizeGradlePackagePath

readGenericManifestTokens :: Package -> IO [String]
readGenericManifestTokens pkg = do
    let manifestCandidates =
            [ "pyproject.toml"
            , "package.json"
            , "Cargo.toml"
            , "go.mod"
            , "mix.exs"
            , "mix.lock"
            , "Package.swift"
            , "pom.xml"
            , "settings.gradle.kts"
            , "build.gradle"
            , "build.gradle.kts"
            , "Makefile.PL"
            , "cpanfile"
            , "project.clj"
            , "deps.edn"
            ]
    entries <- listDirectory (packagePath pkg)
    let rootFiles =
            [ packagePath pkg </> entry
            | entry <- entries
            , entry `elem` manifestCandidates || takeExtension entry `elem` [".cabal", ".gemspec", ".rockspec", ".csproj", ".fsproj", ".sln"]
            ]
    existingFiles <- filterM doesFileExist rootFiles
    fmap (nub . concat) $
        forM existingFiles $ \path -> do
            contents <- readFileStrict path
            pure (tokenize contents)

readLuaDependencyTokens :: Package -> IO [String]
readLuaDependencyTokens pkg = do
    entries <- listDirectory (packagePath pkg)
    let rockspecPaths =
            sort
                [ packagePath pkg </> entry
                | entry <- entries
                , map toLower (takeExtension entry) == ".rockspec"
                ]
    fmap (nub . concat) $
        forM rockspecPaths $ \path -> do
            contents <- readRockspecUtf8 pkg path
            pure (concatMap tokenize (luaDependencyValues contents))

luaDependencyValues :: String -> [String]
luaDependencyValues = collect False . lines
  where
    collect _ [] = []
    collect inside (line : rest)
        | inside =
            quotedValues line
                ++ if closesTable line then [] else collect True rest
        | otherwise =
            case dependencyTableRemainder line of
                Nothing -> collect False rest
                Just remainder ->
                    quotedValues remainder
                        ++ if closesTable remainder then [] else collect True rest

dependencyTableRemainder :: String -> Maybe String
dependencyTableRemainder line =
    let uncommented = stripLuaComment line
        (field, assignment) = break (== '=') uncommented
        afterAssignment = drop 1 assignment
        afterOpeningBrace = drop 1 (dropWhile (/= '{') afterAssignment)
     in if map toLower (trim field) == "dependencies"
            && not (null assignment)
            && '{' `elem` afterAssignment
            then Just afterOpeningBrace
            else Nothing

quotedValues :: String -> [String]
quotedValues = extractQuotedValues . stripLuaComment

extractQuotedValues :: String -> [String]
extractQuotedValues = go
  where
    go [] = []
    go (character : rest)
        | character `elem` ['"', '\''] =
            let (value, remaining) = takeQuoted character rest
             in value : go remaining
        | otherwise = go rest

    takeQuoted _ [] = ([], [])
    takeQuoted quote ('\\' : escaped : rest) =
        let (value, remaining) = takeQuoted quote rest
         in ('\\' : escaped : value, remaining)
    takeQuoted quote (character : rest)
        | character == quote = ([], rest)
        | otherwise =
            let (value, remaining) = takeQuoted quote rest
             in (character : value, remaining)

closesTable :: String -> Bool
closesTable = elem '}' . stripLuaComment

stripLuaComment :: String -> String
stripLuaComment = go Nothing
  where
    go _ [] = []
    go Nothing ('-' : '-' : _) = []
    go quoteState ('\\' : escaped : rest) =
        '\\' : escaped : go quoteState rest
    go Nothing (character : rest)
        | character `elem` ['"', '\''] = character : go (Just character) rest
        | otherwise = character : go Nothing rest
    go (Just quote) (character : rest)
        | character == quote = character : go Nothing rest
        | otherwise = character : go (Just quote) rest

readRockspecUtf8 :: Package -> FilePath -> IO String
readRockspecUtf8 pkg path = do
    bytes <- BS.readFile path
    case TextEncoding.decodeUtf8' bytes of
        Left _ ->
            throwIO
                MetadataEncodingError
                    { metadataErrorCode = "METADATA_INVALID_UTF8"
                    , metadataErrorPackage = packageName pkg
                    , metadataErrorManifest = repositoryRelativeManifest pkg path
                    , metadataErrorEncoding = "UTF-8"
                    }
        Right contents -> pure (Text.unpack contents)

repositoryRelativeManifest :: Package -> FilePath -> FilePath
repositoryRelativeManifest pkg path =
    intercalate
        "/"
        [ "code"
        , "packages"
        , packageName pkg
        , takeFileName path
        ]

tokenize :: String -> [String]
tokenize contents =
    filter (not . null) $
        map
            (map toLower)
            (wordsBy (not . isTokenChar) contents)

isTokenChar :: Char -> Bool
isTokenChar char = isAlphaNum char || char `elem` "-_./"

wordsBy :: (Char -> Bool) -> String -> [String]
wordsBy predicate input =
    case dropWhile predicate input of
        [] -> []
        remaining ->
            let (word, rest) = break predicate remaining
             in word : wordsBy predicate rest

hashPackages :: [Package] -> IO (Map String String)
hashPackages packages =
    fmap Map.fromList $
        forM packages $ \pkg -> do
            digest <- hashPackage pkg
            pure (packageName pkg, digest)

hashPackage :: Package -> IO String
hashPackage pkg = do
    files <- packageRelevantFiles pkg
    filePayloads <-
        forM files $ \path -> do
            contents <- BS.readFile path
            let relative = normalizeRelativePath (makeRelative (packagePath pkg) path)
            let relativeBytes = TextEncoding.encodeUtf8 (Text.pack relative)
            pure (BS.concat [relativeBytes, BS8.pack "\n", contents, BS8.pack "\n"])
    hashBytes (BS.concat filePayloads)

normalizeRelativePath :: FilePath -> FilePath
normalizeRelativePath = map (\character -> if isPathSeparator character then '/' else character)

packageRelevantFiles :: Package -> IO [FilePath]
packageRelevantFiles pkg = do
    allFiles <- collectFilesRecursively (packagePath pkg)
    pure
        ( sort
            [ path
            | path <- allFiles
            , shouldHashFile pkg path
            ]
        )

collectFilesRecursively :: FilePath -> IO [FilePath]
collectFilesRecursively root = do
    entries <- listDirectory root
    fmap concat $
        forM entries $ \entry -> do
            let path = root </> entry
            isDirectory <- doesDirectoryExist path
            if isDirectory
                then do
                    skip <- shouldSkipDirectory path
                    if skip
                        then pure []
                        else collectFilesRecursively path
                else pure [path]

shouldHashFile :: Package -> FilePath -> Bool
shouldHashFile pkg path =
    let extension = map toLower (takeExtension path)
        name = map toLower (takeFileName path)
        buildName = map toLower (takeFileName (packageBuildFile pkg))
        manifestNames =
            [ "pyproject.toml"
            , "package.json"
            , "package-lock.json"
            , "tsconfig.json"
            , "cargo.toml"
            , "cargo.lock"
            , "go.mod"
            , "go.sum"
            , "mix.exs"
            , "mix.lock"
            , "pubspec.yaml"
            , "package.swift"
            , "makefile.pl"
            , "cpanfile"
            , "gemfile"
            , "project.clj"
            , "deps.edn"
            , "settings.gradle.kts"
            , "build.gradle"
            , "build.gradle.kts"
            ]
        sourceExtensions =
            case packageLanguage pkg of
                "python" -> [".py"]
                "ruby" -> [".rb"]
                "go" -> [".go"]
                "rust" -> [".rs"]
                "dart" -> [".dart"]
                "typescript" -> [".ts", ".tsx", ".js", ".jsx"]
                "elixir" -> [".ex", ".exs"]
                "lua" -> [".lua"]
                "perl" -> [".pl", ".pm", ".t"]
                "swift" -> [".swift"]
                "haskell" -> [".hs", ".cabal"]
                "java" -> [".java"]
                "kotlin" -> [".kt", ".kts"]
                "csharp" -> [".cs", ".csproj", ".sln"]
                "fsharp" -> [".fs", ".fsproj", ".sln"]
                "dotnet" -> [".cs", ".fs", ".csproj", ".fsproj", ".sln"]
                "starlark" -> [".star"]
                "wasm" -> [".rs", ".wat"]
                _ -> []
     in name == buildName || name `elem` manifestNames || extension `elem` sourceExtensions

hashDependencyClosure :: DG.DirectedGraph -> Map String String -> String -> String
hashDependencyClosure graph packageHashes pkgName =
    fallbackHash $
        concat
            [ dep ++ ":" ++ Map.findWithDefault "" dep packageHashes ++ "\n"
            | dep <- DG.transitivePredecessors pkgName graph
            ]

hashBytes :: BS.ByteString -> IO String
hashBytes payload = do
    result <- try (hashBytesWithGit payload) :: IO (Either IOException (ExitCode, String))
    pure $
        case result of
            Right (ExitSuccess, digest) -> digest
            _ -> fallbackHashBytes payload

hashBytesWithGit :: BS.ByteString -> IO (ExitCode, String)
hashBytesWithGit payload = do
    (maybeInput, maybeOutput, maybeError, processHandle) <-
        createProcess
            (proc "git" ["hash-object", "--stdin"])
                { std_in = CreatePipe
                , std_out = CreatePipe
                , std_err = CreatePipe
                }
    case (maybeInput, maybeOutput, maybeError) of
        (Just inputHandle, Just outputHandle, Just errorHandle) -> do
            hSetBinaryMode inputHandle True
            hSetBinaryMode outputHandle True
            hSetBinaryMode errorHandle True
            BS.hPut inputHandle payload
            hClose inputHandle
            output <- BS.hGetContents outputHandle
            errorOutput <- BS.hGetContents errorHandle
            exitCode <- waitForProcess processHandle
            _ <- evaluate (BS.length output + BS.length errorOutput)
            hClose outputHandle
            hClose errorHandle
            pure (exitCode, trim (BS8.unpack output))
        _ -> fail "git hash-object did not create binary pipes"

fallbackHashBytes :: BS.ByteString -> String
fallbackHashBytes =
    show
        . BS.foldl'
            (\acc byte -> (acc * 16777619 + fromIntegral byte) `mod` 2147483647)
            (2166136261 :: Int)

fallbackHash :: String -> String
fallbackHash =
    show
        . foldl
            (\acc char -> (acc * 16777619 + ord char) `mod` 2147483647)
            (2166136261 :: Int)

loadCache :: FilePath -> IO BuildCache
loadCache path = do
    exists <- doesFileExist path
    if not exists
        then pure emptyCache
        else do
            contents <- readFileStrict path
            pure (fromMaybe emptyCache (readMaybe contents))

saveCache :: FilePath -> BuildCache -> IO ()
saveCache path cache = do
    createDirectoryIfMissing True (takeDirectory path)
    writeFile path (show cache)

emptyCache :: BuildCache
emptyCache = BuildCache Map.empty

putCacheEntry :: BuildCache -> String -> String -> String -> BuildCache
putCacheEntry cache pkgName pkgHash depHash =
    cache
        { cacheEntries =
            Map.insert
                pkgName
                CacheEntry
                    { cachePackageHash = pkgHash
                    , cacheDependencyHash = depHash
                    }
                (cacheEntries cache)
        }

needsBuild :: BuildCache -> String -> String -> String -> Bool
needsBuild cache pkgName pkgHash depHash =
    case Map.lookup pkgName (cacheEntries cache) of
        Nothing -> True
        Just entry ->
            cachePackageHash entry /= pkgHash || cacheDependencyHash entry /= depHash

detectAffectedPackages :: FilePath -> String -> [Package] -> DG.DirectedGraph -> IO (Maybe (Set String))
detectAffectedPackages repoRoot diffBase packages graph = do
    diffBaseFiles <- gitLines repoRoot ["diff", "--name-only", diffBase ++ "...HEAD"]
    worktreeFiles <- gitLines repoRoot ["diff", "--name-only"]
    untrackedFiles <- gitLines repoRoot ["ls-files", "--others", "--exclude-standard"]
    case diffBaseFiles of
        Nothing -> pure Nothing
        Just committed -> do
            let changedRelative = nub (committed ++ fromMaybe [] worktreeFiles ++ fromMaybe [] untrackedFiles)
            changedAbsolute <- mapM (canonicalizeIfExists . (repoRoot </>)) changedRelative
            let directHits =
                    Set.fromList
                        [ packageName pkg
                        | pkg <- packages
                        , absPath <- changedAbsolute
                        , isWithinPath (packagePath pkg) absPath
                        ]
            pure
                (Just
                    (Set.unions
                        [ Set.insert pkgName (Set.fromList (DG.transitiveDependents pkgName graph))
                        | pkgName <- Set.toList directHits
                        ]
                    )
                )

gitLines :: FilePath -> [String] -> IO (Maybe [String])
gitLines cwdRoot arguments = do
    (exitCode, stdoutText, _) <- readCreateProcessWithExitCode ((proc "git" arguments){cwd = Just cwdRoot}) ""
    pure $
        case exitCode of
            ExitSuccess -> Just (filter (not . null) (map trim (lines stdoutText)))
            ExitFailure _ -> Nothing

canonicalizeIfExists :: FilePath -> IO FilePath
canonicalizeIfExists path = do
    exists <- doesFileExist path
    isDirectory <- doesDirectoryExist path
    if exists || isDirectory
        then canonicalizePath path
        else pure (normalise path)

isWithinPath :: FilePath -> FilePath -> Bool
isWithinPath parent child =
    let normalizedParent = addTrailingPathSeparator (normalise parent)
        normalizedChild = normalise child
        relative = makeRelative normalizedParent normalizedChild
     in not (".." `isPrefixOf` relative) && relative /= "." && not (isAbsolute relative)

writePlan ::
       FilePath
    -> String
    -> FilePath
    -> [Package]
    -> BuildCache
    -> Map String String
    -> Map String String
    -> DG.DirectedGraph
    -> Config
    -> Maybe (Set String)
    -> IO ()
writePlan path timestamp repoRoot packages cache packageHashes dependencyHashes graph cfg affectedSet = do
    let items =
            [ PlanItem
                { planPackageName = packageName pkg
                , planLanguage = packageLanguage pkg
                , planPath = packagePath pkg
                , planBuildFile = packageBuildFile pkg
                , planCommands = packageBuildCommands pkg
                , planReason = reason
                }
            | pkg <- packages
            , Just reason <- [buildReason cache packageHashes dependencyHashes cfg affectedSet graph pkg]
            ]
    createDirectoryIfMissing True (takeDirectory path)
    writeFile
        path
        (planDocumentJson
            PlanDocument
                { planGeneratedAt = timestamp
                , planRepoRoot = repoRoot
                , planPackages = items
                }
        )

buildReason ::
       BuildCache
    -> Map String String
    -> Map String String
    -> Config
    -> Maybe (Set String)
    -> DG.DirectedGraph
    -> Package
    -> Maybe String
buildReason cache packageHashes dependencyHashes cfg affectedSet _graph pkg
    | configForce cfg = Just "force"
    | otherwise =
        case affectedSet of
            Just affected ->
                if packageName pkg `Set.member` affected
                    then Just "git-diff"
                    else Nothing
            Nothing ->
                if needsBuild cache pkgName pkgHash depHash
                    then Just "cache-miss"
                    else Nothing
  where
    pkgName = packageName pkg
    pkgHash = Map.findWithDefault "" pkgName packageHashes
    depHash = Map.findWithDefault "" pkgName dependencyHashes

executeBuilds ::
       [Package]
    -> DG.DirectedGraph
    -> BuildCache
    -> Map String String
    -> Map String String
    -> Config
    -> Maybe (Set String)
    -> IO [BuildResult]
executeBuilds packages graph cache packageHashes dependencyHashes cfg affectedSet =
    case DG.independentGroups graph of
        Left DG.CycleError ->
            pure
                [ BuildResult
                    { resultPackageName = packageName pkg
                    , resultStatus = "failed"
                    , resultDurationSeconds = 0
                    , resultStdout = ""
                    , resultStderr = "cycle detected in dependency graph"
                    , resultReturnCode = 1
                    }
                | pkg <- packages
                ]
        Left (DG.NodeNotFound missingNode) ->
            pure
                [ BuildResult
                    { resultPackageName = packageName pkg
                    , resultStatus = "failed"
                    , resultDurationSeconds = 0
                    , resultStdout = ""
                    , resultStderr = "node not found in dependency graph: " ++ missingNode
                    , resultReturnCode = 1
                    }
                | pkg <- packages
                ]
        Right groups -> do
            let packageMap = Map.fromList [(packageName pkg, pkg) | pkg <- packages]
            defaultJobs <- getNumCapabilities
            let maxJobs = max 1 (fromMaybe defaultJobs (configJobs cfg))
            foldM
                (\results level -> do
                    levelResults <- runLevel packageMap results maxJobs level
                    pure (results ++ levelResults)
                )
                []
                groups
  where
    runLevel packageMap priorResults maxJobs level = do
        let failedPackages =
                Set.fromList
                    [ resultPackageName result
                    | result <- priorResults
                    , resultStatus result == "failed"
                    ]
        let decisions =
                mapMaybe
                    (\pkgName -> do
                        pkg <- Map.lookup pkgName packageMap
                        let deps = Set.fromList (DG.transitivePredecessors pkgName graph)
                        if not (Set.null (Set.intersection deps failedPackages))
                            then
                                Just
                                    ( Left
                                        BuildResult
                                            { resultPackageName = pkgName
                                            , resultStatus = "dep-skipped"
                                            , resultDurationSeconds = 0
                                            , resultStdout = ""
                                            , resultStderr = ""
                                            , resultReturnCode = 0
                                            }
                                    )
                            else
                                case buildReason cache packageHashes dependencyHashes cfg affectedSet graph pkg of
                                    Nothing ->
                                        Just
                                            ( Left
                                                BuildResult
                                                    { resultPackageName = pkgName
                                                    , resultStatus = "skipped"
                                                    , resultDurationSeconds = 0
                                                    , resultStdout = ""
                                                    , resultStderr = ""
                                                    , resultReturnCode = 0
                                                    }
                                            )
                                    Just _
                                        | configDryRun cfg ->
                                            Just
                                                ( Left
                                                    BuildResult
                                                        { resultPackageName = pkgName
                                                        , resultStatus = "would-build"
                                                        , resultDurationSeconds = 0
                                                        , resultStdout = ""
                                                        , resultStderr = ""
                                                        , resultReturnCode = 0
                                                        }
                                                )
                                        | otherwise -> Just (Right pkg)
                    )
                    level
        let immediate = [result | Left result <- decisions]
        let toRun = [pkg | Right pkg <- decisions]
        built <- runInChunks maxJobs toRun
        pure (immediate ++ built)

    runInChunks _ [] = pure []
    runInChunks maxJobs pkgs = do
        let (chunk, rest) = splitAt maxJobs pkgs
        builtChunk <- mapM runPackageBuild chunk
        remainder <- runInChunks maxJobs rest
        pure (builtChunk ++ remainder)

runPackageBuild :: Package -> IO BuildResult
runPackageBuild pkg = do
    start <- getCurrentTime
    outcomes <- mapM (runShellCommand (packagePath pkg)) (packageBuildCommands pkg)
    end <- getCurrentTime
    let stdoutText = concatMap (\(_, out, _) -> out) outcomes
    let stderrText = concatMap (\(_, _, err) -> err) outcomes
    case listToMaybe [(code, out, err) | (code@(ExitFailure _), out, err) <- outcomes] of
        Just (ExitFailure returnCode, _, _) ->
            pure
                BuildResult
                    { resultPackageName = packageName pkg
                    , resultStatus = "failed"
                    , resultDurationSeconds = realToFrac (diffUTCTime end start)
                    , resultStdout = stdoutText
                    , resultStderr = stderrText
                    , resultReturnCode = returnCode
                    }
        _ ->
            pure
                BuildResult
                    { resultPackageName = packageName pkg
                    , resultStatus = "built"
                    , resultDurationSeconds = realToFrac (diffUTCTime end start)
                    , resultStdout = stdoutText
                    , resultStderr = stderrText
                    , resultReturnCode = 0
                    }

runShellCommand :: FilePath -> String -> IO (ExitCode, String, String)
runShellCommand cwdRoot commandText =
    if hostOS == "mingw32"
        then readCreateProcessWithExitCode ((proc "cmd" ["/C", commandText]){cwd = Just cwdRoot}) ""
        else readCreateProcessWithExitCode ((proc "sh" ["-c", commandText]){cwd = Just cwdRoot}) ""

renderResults :: [BuildResult] -> IO ()
renderResults results = do
    let statusLine result =
            resultPackageName result
                ++ " ["
                ++ resultStatus result
                ++ "] ("
                ++ show (realToFrac (resultDurationSeconds result) :: Double)
                ++ "s)"
    mapM_ (putStrLn . statusLine) results
    let builtCount = length [() | result <- results, resultStatus result == "built"]
    let skippedCount = length [() | result <- results, resultStatus result `elem` ["skipped", "dep-skipped", "would-build"]]
    let failedCount = length [() | result <- results, resultStatus result == "failed"]
    putStrLn ""
    putStrLn
        ("Summary: built="
            ++ show builtCount
            ++ ", skipped="
            ++ show skippedCount
            ++ ", failed="
            ++ show failedCount
        )

currentTimestamp :: IO String
currentTimestamp = formatTime defaultTimeLocale "%Y-%m-%dT%H:%M:%SZ" <$> getCurrentTime

absolutizeWithin :: FilePath -> FilePath -> FilePath
absolutizeWithin root path
    | isAbsolute path = path
    | otherwise = root </> path

trim :: String -> String
trim = dropWhileEnd isWhitespace . dropWhile isWhitespace

isWhitespace :: Char -> Bool
isWhitespace char = char `elem` [' ', '\n', '\r', '\t']

dropWhileEnd :: (Char -> Bool) -> String -> String
dropWhileEnd predicate = reverse . dropWhile predicate . reverse

jsonString :: String -> String
jsonString value =
    "\"" ++ concatMap escape value ++ "\""
  where
    escape '"' = "\\\""
    escape '\\' = "\\\\"
    escape '\n' = "\\n"
    escape '\r' = "\\r"
    escape '\t' = "\\t"
    escape char = [char]

jsonArray :: [String] -> String
jsonArray values = "[" ++ intercalate ", " values ++ "]"

planItemJson :: PlanItem -> String
planItemJson item =
    "{"
        ++ intercalate
            ", "
            [ "\"package_name\": " ++ jsonString (planPackageName item)
            , "\"language\": " ++ jsonString (planLanguage item)
            , "\"path\": " ++ jsonString (planPath item)
            , "\"build_file\": " ++ jsonString (planBuildFile item)
            , "\"commands\": " ++ jsonArray (map jsonString (planCommands item))
            , "\"reason\": " ++ jsonString (planReason item)
            ]
        ++ "}"

planDocumentJson :: PlanDocument -> String
planDocumentJson document =
    "{\n"
        ++ "  \"generated_at\": "
        ++ jsonString (planGeneratedAt document)
        ++ ",\n  \"repo_root\": "
        ++ jsonString (planRepoRoot document)
        ++ ",\n  \"packages\": "
        ++ jsonArray (map planItemJson (planPackages document))
        ++ "\n}\n"
