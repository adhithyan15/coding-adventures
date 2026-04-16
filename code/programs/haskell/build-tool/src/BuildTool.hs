module BuildTool
    ( BuildResult(..)
    , Config(..)
    , Package(..)
    , ParsedArgs(..)
    , defaultConfig
    , discoverPackages
    , findRepoRoot
    , inferLanguage
    , parseArgs
    , runWithArgs
    ) where

import Control.Monad (filterM, foldM, forM, unless, when)
import Data.Char (isAlphaNum, ord, toLower)
import Data.List (intercalate, isPrefixOf, nub, sort, sortOn)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Maybe (fromMaybe, listToMaybe, mapMaybe)
import qualified Data.Set as Set
import Data.Set (Set)
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
    , makeRelative
    , normalise
    , splitDirectories
    , takeDirectory
    , takeExtension
    , takeFileName
    )
import qualified System.Info as SystemInfo
import System.Process (CreateProcess(..), proc, readCreateProcessWithExitCode)
import Text.Read (readMaybe)

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
        Right (ParsedRun cfg) -> runBuild cfg

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
inferPackageName path = inferLanguage path ++ "/" ++ takeFileName path

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
    contents <- readFile path
    pure
        [ trim line
        | line <- lines contents
        , let stripped = trim line
        , not (null stripped)
        , not ("#" `isPrefixOf` stripped)
        ]

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
    dependencyPairs <-
        forM packages $ \pkg -> do
            deps <- resolvePackageDeps aliasScopes pkg
            pure (pkg, deps)
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
buildAliasScopes packages =
    foldM registerAlias Map.empty packages
  where
    registerAlias scopes pkg = do
        aliases <- packageAliases pkg
        let scope = dependencyScope (packageLanguage pkg)
        let scopeMap = Map.findWithDefault Map.empty scope scopes
        let updatedScope =
                foldl
                    (\aliasMap alias -> Map.insert alias (packageName pkg) aliasMap)
                    scopeMap
                    aliases
        pure (Map.insert scope updatedScope scopes)

dependencyScope :: String -> String
dependencyScope language
    | language `elem` ["csharp", "fsharp", "dotnet"] = "dotnet"
    | otherwise = language

packageAliases :: Package -> IO [String]
packageAliases pkg = do
    let dirName = map toLower (takeFileName (packagePath pkg))
    let kebab = map (\char -> if char == '_' then '-' else char) dirName
    let snake = map (\char -> if char == '-' then '_' else char) dirName
    manifestNames <- exactManifestNames pkg
    pure
        (nub
            ( filter
                (not . null)
                ( map (map toLower)
                    ( [ dirName
                      , kebab
                      , snake
                      , "coding-adventures-" ++ kebab
                      , "coding_adventures_" ++ snake
                      ]
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
    let packageJsonNames = if packageLanguage pkg == "typescript" then readPackageJsonNames root else pure []
    let gemNames = if packageLanguage pkg == "ruby" then readGemspecNames root else pure []
    sequenceToList [cabalNames, cargoNames, pythonNames, goNames, packageJsonNames, gemNames]

sequenceToList :: [IO [String]] -> IO [String]
sequenceToList actions = fmap concat (sequence actions)

readCabalPackageNames :: FilePath -> IO [String]
readCabalPackageNames root = do
    entries <- listDirectory root
    let cabalFiles = [root </> entry | entry <- entries, takeExtension entry == ".cabal"]
    names <- mapM readSimpleFieldName cabalFiles
    pure (mapMaybe id names)

readCargoNames :: FilePath -> IO [String]
readCargoNames root = do
    let cargoPath = root </> "Cargo.toml"
    exists <- doesFileExist cargoPath
    if not exists
        then pure []
        else do
            contents <- readFile cargoPath
            pure (parseQuotedField "name" contents)

readPyprojectNames :: FilePath -> IO [String]
readPyprojectNames root = do
    let pyprojectPath = root </> "pyproject.toml"
    exists <- doesFileExist pyprojectPath
    if not exists
        then pure []
        else do
            contents <- readFile pyprojectPath
            pure (parseAssignmentField "name" contents)

readGoModuleNames :: FilePath -> IO [String]
readGoModuleNames root = do
    let goModPath = root </> "go.mod"
    exists <- doesFileExist goModPath
    if not exists
        then pure []
        else do
            contents <- readFile goModPath
            pure
                [ map toLower (trim (drop (length ("module" :: String)) stripped))
                | line <- lines contents
                , let stripped = trim line
                , "module " `isPrefixOf` stripped
                ]

readPackageJsonNames :: FilePath -> IO [String]
readPackageJsonNames root = do
    let packageJson = root </> "package.json"
    exists <- doesFileExist packageJson
    if not exists
        then pure []
        else do
            contents <- readFile packageJson
            pure (parseJsonLikeField "name" contents)

readGemspecNames :: FilePath -> IO [String]
readGemspecNames root = do
    entries <- listDirectory root
    let gemspecs = [root </> entry | entry <- entries, takeExtension entry == ".gemspec"]
    fmap concat $
        forM gemspecs $ \path -> do
            contents <- readFile path
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

readSimpleFieldName :: FilePath -> IO (Maybe String)
readSimpleFieldName path = do
    contents <- readFile path
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

parseJsonLikeField :: String -> String -> [String]
parseJsonLikeField fieldName contents =
    nub
        [ map toLower value
        | line <- lines contents
        , let stripped = trim line
        , let prefix = "\"" ++ fieldName ++ "\""
        , prefix `isPrefixOf` stripped
        , let afterColon = dropWhile (/= ':') stripped
        , not (null afterColon)
        , let rhs = trim (drop 1 afterColon)
        , not (null rhs)
        , take 1 rhs == "\""
        , let value = takeWhile (/= '"') (drop 1 rhs)
        , not (null value)
        ]

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

readManifestTokens :: Package -> IO [String]
readManifestTokens pkg = do
    let manifestCandidates =
            [ "pyproject.toml"
            , "package.json"
            , "Cargo.toml"
            , "go.mod"
            , "mix.exs"
            , "mix.lock"
            , "Package.swift"
            , "pom.xml"
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
            contents <- readFile path
            pure (tokenize contents)

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
            contents <- readFile path
            let relative = makeRelative (packagePath pkg) path
            pure (relative ++ "\n" ++ contents ++ "\n")
    hashString (concat filePayloads)

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
            , "package.swift"
            , "makefile.pl"
            , "cpanfile"
            , "gemfile"
            , "project.clj"
            , "deps.edn"
            ]
        sourceExtensions =
            case packageLanguage pkg of
                "python" -> [".py"]
                "ruby" -> [".rb"]
                "go" -> [".go"]
                "rust" -> [".rs"]
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

hashString :: String -> IO String
hashString payload = do
    (exitCode, stdoutText, _) <- readCreateProcessWithExitCode (proc "git" ["hash-object", "--stdin"]) payload
    pure $
        case exitCode of
            ExitSuccess -> trim stdoutText
            ExitFailure _ -> fallbackHash payload

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
            contents <- readFile path
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
