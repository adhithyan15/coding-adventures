module ScaffoldGenerator
    ( ParsedArgs(..)
    , ScaffoldConfig(..)
    , defaultConfig
    , isKebabCase
    , parseArgs
    , runWithArgs
    , toModuleName
    ) where

import Control.Monad (filterM, foldM, forM)
import Data.Char (isAlphaNum, isLower, toLower, toUpper)
import Data.List (intercalate, nub, sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Maybe (listToMaybe, mapMaybe)
import qualified Data.Set as Set
import Data.Set (Set)
import System.Directory
    ( canonicalizePath
    , createDirectoryIfMissing
    , doesDirectoryExist
    , doesFileExist
    , getCurrentDirectory
    , listDirectory
    )
import System.FilePath
    ( (</>)
    , makeRelative
    , takeDirectory
    , takeExtension
    )

data ScaffoldConfig = ScaffoldConfig
    { configRoot :: Maybe FilePath
    , configPackageName :: Maybe String
    , configPackageType :: String
    , configDependsOn :: [String]
    , configLayer :: Int
    , configDescription :: String
    , configDryRun :: Bool
    }
    deriving (Eq, Show)

data ParsedArgs
    = ParsedHelp
    | ParsedVersion
    | ParsedRun ScaffoldConfig
    deriving (Eq, Show)

data HaskellPackage = HaskellPackage
    { registryDirName :: String
    , registryCabalName :: String
    , registryPath :: FilePath
    }
    deriving (Eq, Show)

versionString :: String
versionString = "0.1.0"

defaultConfig :: ScaffoldConfig
defaultConfig =
    ScaffoldConfig
        { configRoot = Nothing
        , configPackageName = Nothing
        , configPackageType = "library"
        , configDependsOn = []
        , configLayer = 0
        , configDescription = ""
        , configDryRun = False
        }

usageText :: String
usageText =
    unlines
        [ "scaffold-generator -- Create Haskell package or program scaffolding"
        , ""
        , "USAGE:"
        , "    scaffold-generator [OPTIONS] PACKAGE_NAME"
        , ""
        , "OPTIONS:"
        , "        --root <PATH>              Repo root (auto-detects .git if omitted)"
        , "    -t, --type <TYPE>              library or program (default: library)"
        , "    -d, --depends-on <DEPS>        Comma-separated sibling Haskell dependencies"
        , "        --layer <N>                Layer number for README context"
        , "        --description <TEXT>       One-line description"
        , "        --dry-run                  Print target path without writing files"
        , "    -h, --help                     Show this help"
        , "    -V, --version                  Show version"
        ]

parseArgs :: [String] -> Either String ParsedArgs
parseArgs = go defaultConfig
  where
    go cfg [] =
        case configPackageName cfg of
            Nothing -> Left "missing PACKAGE_NAME"
            Just _ -> Right (ParsedRun cfg)
    go _ ["--root"] = Left "--root requires a value"
    go _ ["-t"] = Left "--type requires a value"
    go _ ["--type"] = Left "--type requires a value"
    go _ ["-d"] = Left "--depends-on requires a value"
    go _ ["--depends-on"] = Left "--depends-on requires a value"
    go _ ["--layer"] = Left "--layer requires a value"
    go _ ["--description"] = Left "--description requires a value"
    go _ ("-h" : _) = Right ParsedHelp
    go _ ("--help" : _) = Right ParsedHelp
    go _ ("-V" : _) = Right ParsedVersion
    go _ ("--version" : _) = Right ParsedVersion
    go cfg ("--root" : value : rest) = go cfg{configRoot = Just value} rest
    go cfg ("-t" : value : rest) = go cfg{configPackageType = value} rest
    go cfg ("--type" : value : rest) = go cfg{configPackageType = value} rest
    go cfg ("-d" : value : rest) = go cfg{configDependsOn = parseCommaList value} rest
    go cfg ("--depends-on" : value : rest) = go cfg{configDependsOn = parseCommaList value} rest
    go cfg ("--layer" : value : rest) =
        case reads value of
            [(layerValue, "")] -> go cfg{configLayer = layerValue} rest
            _ -> Left ("invalid value for --layer: " ++ value)
    go cfg ("--description" : value : rest) = go cfg{configDescription = value} rest
    go cfg ("--dry-run" : rest) = go cfg{configDryRun = True} rest
    go cfg (value : rest)
        | "--" `isPrefixOfWord` value = Left ("unknown flag: " ++ value)
        | otherwise =
            case configPackageName cfg of
                Nothing -> go cfg{configPackageName = Just value} rest
                Just _ -> Left ("unexpected extra argument: " ++ value)

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
        Right (ParsedRun cfg) -> runScaffold cfg

runScaffold :: ScaffoldConfig -> IO Int
runScaffold cfg = do
    rootResult <- resolveRepoRoot cfg
    case rootResult of
        Left err -> do
            putStrLn err
            pure 1
        Right repoRoot ->
            case configPackageName cfg of
                Nothing -> do
                    putStrLn "missing PACKAGE_NAME"
                    pure 1
                Just pkgName ->
                    if not (isKebabCase pkgName)
                        then do
                            putStrLn ("invalid PACKAGE_NAME: " ++ pkgName)
                            pure 1
                        else if configPackageType cfg `notElem` ["library", "program"]
                            then do
                                putStrLn "package type must be 'library' or 'program'"
                                pure 1
                            else do
                                registry <- discoverRegistry repoRoot
                                dependencyClosure <- resolveDependencyClosure registry (configDependsOn cfg)
                                case dependencyClosure of
                                    Left err -> do
                                        putStrLn err
                                        pure 1
                                    Right deps -> do
                                        let targetDir = targetDirectory repoRoot (configPackageType cfg) pkgName
                                        targetExists <- doesDirectoryExist targetDir
                                        if targetExists
                                            then do
                                                putStrLn ("target already exists: " ++ targetDir)
                                                pure 1
                                            else do
                                                if configDryRun cfg
                                                    then do
                                                        putStrLn ("Would generate: " ++ targetDir)
                                                        pure 0
                                                    else do
                                                        generatePackage repoRoot cfg pkgName deps targetDir
                                                        putStrLn ("Generated " ++ targetDir)
                                                        pure 0

resolveRepoRoot :: ScaffoldConfig -> IO (Either String FilePath)
resolveRepoRoot cfg =
    case configRoot cfg of
        Just root -> Right <$> canonicalizePath root
        Nothing -> do
            found <- findRepoRoot
            pure $
                case found of
                    Nothing -> Left "could not locate repo root (.git directory)"
                    Just root -> Right root

findRepoRoot :: IO (Maybe FilePath)
findRepoRoot = do
    start <- canonicalizePath =<< getCurrentDirectory
    walk start
  where
    walk current = do
        let gitDir = current </> ".git"
        gitDirExists <- doesDirectoryExist gitDir
        gitFileExists <- doesFileExist gitDir
        if gitDirExists || gitFileExists
            then pure (Just current)
            else
                let parent = takeDirectory current
                 in if parent == current
                        then pure Nothing
                        else walk parent

discoverRegistry :: FilePath -> IO (Map String HaskellPackage)
discoverRegistry repoRoot = do
    packages <- discoverHaskellDirs (repoRoot </> "code" </> "packages" </> "haskell")
    programs <- discoverHaskellDirs (repoRoot </> "code" </> "programs" </> "haskell")
    let allPackages = packages ++ programs
    pure
        (Map.fromList
            ([ (registryDirName pkg, pkg) | pkg <- allPackages ]
                ++ [ (registryCabalName pkg, pkg) | pkg <- allPackages ]
            )
        )

discoverHaskellDirs :: FilePath -> IO [HaskellPackage]
discoverHaskellDirs root = do
    exists <- doesDirectoryExist root
    if not exists
        then pure []
        else do
            entries <- listDirectory root
            directories <- filterM (doesDirectoryExist . (root </>)) entries
            fmap (mapMaybe id) $
                forM directories $ \entry -> do
                    let dirPath = root </> entry
                    cabalName <- readCabalName dirPath
                    pure $
                        fmap
                            (\name ->
                                HaskellPackage
                                    { registryDirName = entry
                                    , registryCabalName = name
                                    , registryPath = dirPath
                                    }
                            )
                            cabalName

readCabalName :: FilePath -> IO (Maybe String)
readCabalName dirPath = do
    entries <- listDirectory dirPath
    let cabalFiles = [dirPath </> entry | entry <- entries, takeExtension entry == ".cabal"]
    names <- mapM readNameField cabalFiles
    pure (listToMaybe (mapMaybe id names))

readNameField :: FilePath -> IO (Maybe String)
readNameField path = do
    contents <- readFile path
    pure $
        listToMaybe
            [ trim (drop 1 rest)
            | line <- lines contents
            , let stripped = trim line
            , let (field, rest) = break (== ':') stripped
            , map toLower field == "name"
            , not (null rest)
            ]

resolveDependencyClosure :: Map String HaskellPackage -> [String] -> IO (Either String [HaskellPackage])
resolveDependencyClosure registry directDeps = do
    let uniqueDirect = nub (filter (not . null) directDeps)
    foldM step (Right ([], Set.empty)) uniqueDirect >>= pure . fmap finish
  where
    step (Left err) _ = pure (Left err)
    step (Right (ordered, seen)) depName =
        case Map.lookup depName registry of
            Nothing -> pure (Left ("unknown dependency: " ++ depName))
            Just depPkg -> expand registry depPkg (Right (ordered, seen))

    finish (ordered, _) = ordered

expand ::
       Map String HaskellPackage
    -> HaskellPackage
    -> Either String ([HaskellPackage], Set String)
    -> IO (Either String ([HaskellPackage], Set String))
expand _ _ (Left err) = pure (Left err)
expand registry pkg (Right (ordered, seen))
    | registryDirName pkg `Set.member` seen = pure (Right (ordered, seen))
    | otherwise = do
        deps <- readInternalDeps registry (registryPath pkg)
        expanded <- foldM (\acc dep -> expand registry dep acc) (Right (ordered, seen)) deps
        pure $
            case expanded of
                Left err -> Left err
                Right (ordered', seen') ->
                    Right (ordered' ++ [pkg], Set.insert (registryDirName pkg) seen')

readInternalDeps :: Map String HaskellPackage -> FilePath -> IO [HaskellPackage]
readInternalDeps registry dirPath = do
    entries <- listDirectory dirPath
    let cabalFiles = [dirPath </> entry | entry <- entries, takeExtension entry == ".cabal"]
    contents <- fmap concat (mapM readFile cabalFiles)
    let tokens = tokenize contents
    pure
        ( nub
            [ depPkg
            | token <- tokens
            , Just depPkg <- [Map.lookup token registry]
            , registryPath depPkg /= dirPath
            ]
        )

generatePackage ::
       FilePath
    -> ScaffoldConfig
    -> String
    -> [HaskellPackage]
    -> FilePath
    -> IO ()
generatePackage repoRoot cfg pkgName deps targetDir = do
    let moduleName = toModuleName pkgName
    let cabalName =
            if configPackageType cfg == "program"
                then "coding-adventures-" ++ pkgName
                else pkgName
    let dependencyNames = nub (map registryCabalName deps)
    let dependencyPaths = nub (map registryPath deps)
    let relativeDepPaths = sort [makeRelative targetDir depPath | depPath <- dependencyPaths]
    let descriptionText =
            if null (configDescription cfg)
                then defaultDescription cfg pkgName
                else configDescription cfg
    createDirectoryIfMissing True targetDir
    case configPackageType cfg of
        "program" -> generateProgram targetDir moduleName cabalName descriptionText dependencyNames relativeDepPaths
        _ -> generateLibrary targetDir moduleName cabalName descriptionText dependencyNames relativeDepPaths
    writeFile (targetDir </> "README.md") (readmeContents cfg pkgName descriptionText dependencyNames)
    writeFile (targetDir </> "CHANGELOG.md") "# Changelog\n\n## 0.1.0\n\n- Initial scaffold.\n"
    writeFile (targetDir </> "BUILD") buildFileContents
    writeFile (targetDir </> "BUILD_windows") "echo \"haskell support not enabled in windows CI yet -- skipping\"\n"
    writeFile (targetDir </> "cabal.project") (cabalProjectContents relativeDepPaths)
  where
    _ = repoRoot

generateLibrary :: FilePath -> String -> String -> String -> [String] -> [FilePath] -> IO ()
generateLibrary targetDir moduleName cabalName descriptionText dependencyNames _ = do
    createDirectoryIfMissing True (targetDir </> "src")
    createDirectoryIfMissing True (targetDir </> "test")
    writeFile (targetDir </> "src" </> (moduleName ++ ".hs")) (libraryModuleContents moduleName descriptionText)
    writeFile (targetDir </> "test" </> (moduleName ++ "Spec.hs")) (librarySpecContents moduleName "description")
    writeFile (targetDir </> "test" </> "Spec.hs") (rootSpecContents moduleName)
    writeFile (targetDir </> (cabalName ++ ".cabal")) (libraryCabalContents moduleName cabalName descriptionText dependencyNames)

generateProgram :: FilePath -> String -> String -> String -> [String] -> [FilePath] -> IO ()
generateProgram targetDir moduleName cabalName descriptionText dependencyNames _ = do
    createDirectoryIfMissing True (targetDir </> "src")
    createDirectoryIfMissing True (targetDir </> "app")
    createDirectoryIfMissing True (targetDir </> "test")
    writeFile (targetDir </> "src" </> (moduleName ++ ".hs")) (programModuleContents moduleName descriptionText)
    writeFile (targetDir </> "app" </> "Main.hs") (programMainContents moduleName)
    writeFile (targetDir </> "test" </> (moduleName ++ "Spec.hs")) (librarySpecContents moduleName "programDescription")
    writeFile (targetDir </> "test" </> "Spec.hs") (rootSpecContents moduleName)
    writeFile (targetDir </> (cabalName ++ ".cabal")) (programCabalContents moduleName cabalName descriptionText dependencyNames)

libraryCabalContents :: String -> String -> String -> [String] -> String
libraryCabalContents moduleName cabalName descriptionText dependencyNames =
    unlines $
        [ "cabal-version: 3.0"
        , "name:          " ++ cabalName
        , "version:       0.1.0"
        , "synopsis:      " ++ descriptionText
        , "license:       MIT"
        , "author:        Adhithya Rajasekaran"
        , "maintainer:    Adhithya Rajasekaran"
        , "build-type:    Simple"
        , ""
        , "library"
        , "    exposed-modules:  " ++ moduleName
        , "    build-depends:    base >=4.14"
        ]
            ++ map (\dep -> "                    , " ++ dep) dependencyNames
            ++ [ "    hs-source-dirs:   src"
               , "    default-language: Haskell2010"
               , ""
               , "test-suite spec"
               , "    type:             exitcode-stdio-1.0"
               , "    main-is:          Spec.hs"
               , "    other-modules:    " ++ moduleName ++ "Spec"
               , "    hs-source-dirs:   test"
               , "    build-depends:    base >=4.14"
               , "                    , " ++ cabalName
               , "                    , hspec == 2.*"
               , "    default-language: Haskell2010"
               ]

programCabalContents :: String -> String -> String -> [String] -> String
programCabalContents moduleName cabalName descriptionText dependencyNames =
    unlines $
        [ "cabal-version: 3.0"
        , "name:          " ++ cabalName
        , "version:       0.1.0"
        , "synopsis:      " ++ descriptionText
        , "license:       MIT"
        , "author:        Adhithya Rajasekaran"
        , "maintainer:    Adhithya Rajasekaran"
        , "build-type:    Simple"
        , ""
        , "library"
        , "    exposed-modules:  " ++ moduleName
        , "    build-depends:    base >=4.14"
        ]
            ++ map (\dep -> "                    , " ++ dep) dependencyNames
            ++ [ "    hs-source-dirs:   src"
               , "    default-language: Haskell2010"
               , ""
               , "executable " ++ registryExecutableName moduleName
               , "    main-is:          Main.hs"
               , "    hs-source-dirs:   app"
               , "    build-depends:    base >=4.14"
               , "                    , " ++ cabalName
               ]
            ++ map (\dep -> "                    , " ++ dep) dependencyNames
            ++ [ "    default-language: Haskell2010"
               , ""
               , "test-suite spec"
               , "    type:             exitcode-stdio-1.0"
               , "    main-is:          Spec.hs"
               , "    other-modules:    " ++ moduleName ++ "Spec"
               , "    hs-source-dirs:   test"
               , "    build-depends:    base >=4.14"
               , "                    , " ++ cabalName
               , "                    , hspec == 2.*"
               , "    default-language: Haskell2010"
               ]

registryExecutableName :: String -> String
registryExecutableName moduleName = map toLower moduleName

libraryModuleContents :: String -> String -> String
libraryModuleContents moduleName descriptionText =
    unlines
        [ "module " ++ moduleName ++ " where"
        , ""
        , "description :: String"
        , "description = " ++ show descriptionText
        ]

programModuleContents :: String -> String -> String
programModuleContents moduleName descriptionText =
    unlines
        [ "module " ++ moduleName ++ " where"
        , ""
        , "programDescription :: String"
        , "programDescription = " ++ show descriptionText
        , ""
        , "run :: IO ()"
        , "run = putStrLn programDescription"
        ]

programMainContents :: String -> String
programMainContents moduleName =
    unlines
        [ "module Main where"
        , ""
        , "import " ++ moduleName ++ " (run)"
        , ""
        , "main :: IO ()"
        , "main = run"
        ]

librarySpecContents :: String -> String -> String
librarySpecContents moduleName exportedValue =
    unlines
        [ "module " ++ moduleName ++ "Spec (spec) where"
        , ""
        , "import Test.Hspec"
        , "import " ++ moduleName
        , ""
        , "spec :: Spec"
        , "spec = describe " ++ show moduleName ++ " $ do"
        , "    it \"exposes a non-empty starter description\" $ do"
        , "        " ++ exportedValue ++ " `shouldSatisfy` (not . null)"
        ]

rootSpecContents :: String -> String
rootSpecContents moduleName =
    unlines
        [ "import Test.Hspec"
        , "import " ++ moduleName ++ "Spec"
        , ""
        , "main :: IO ()"
        , "main = hspec spec"
        ]

cabalProjectContents :: [FilePath] -> String
cabalProjectContents relativeDeps =
    unlines $
        [ "packages: ." ]
            ++ map ("          " ++) relativeDeps

buildFileContents :: String
buildFileContents =
    "if command -v cabal >/dev/null 2>&1; then cabal test all; else echo 'cabal not found -- skipping'; fi\n"

readmeContents :: ScaffoldConfig -> String -> String -> [String] -> String
readmeContents cfg pkgName descriptionText dependencyNames =
    unlines
        [ "# " ++ pkgName
        , ""
        , descriptionText
        , ""
        , "## Type"
        , ""
        , configPackageType cfg
        , ""
        , "## Dependencies"
        , ""
        , if null dependencyNames then "(none)" else intercalate ", " dependencyNames
        ]

defaultDescription :: ScaffoldConfig -> String -> String
defaultDescription cfg pkgName =
    case configPackageType cfg of
        "program" -> "Executable entry point for " ++ pkgName ++ layerSuffix
        _ -> "Educational Haskell package for " ++ pkgName ++ layerSuffix
  where
    layerSuffix =
        if configLayer cfg > 0
            then " (layer " ++ show (configLayer cfg) ++ ")"
            else ""

targetDirectory :: FilePath -> String -> String -> FilePath
targetDirectory repoRoot pkgType pkgName =
    case pkgType of
        "program" -> repoRoot </> "code" </> "programs" </> "haskell" </> pkgName
        _ -> repoRoot </> "code" </> "packages" </> "haskell" </> pkgName

parseCommaList :: String -> [String]
parseCommaList =
    filter (not . null)
        . map trim
        . splitOnComma

splitOnComma :: String -> [String]
splitOnComma [] = [""]
splitOnComma (',' : rest) = "" : splitOnComma rest
splitOnComma (char : rest) =
    case splitOnComma rest of
        [] -> [[char]]
        part : parts -> (char : part) : parts

isKebabCase :: String -> Bool
isKebabCase [] = False
isKebabCase (first : rest)
    | not (isLower first) = False
    | otherwise = all validChar rest && lastChar /= '-' && not (hasDoubleDash (first : rest))
  where
    validChar char = isLower char || isAlphaNum char || char == '-'
    lastChar = last (first : rest)

hasDoubleDash :: String -> Bool
hasDoubleDash [] = False
hasDoubleDash [_] = False
hasDoubleDash ('-' : '-' : _) = True
hasDoubleDash (_ : rest) = hasDoubleDash rest

toModuleName :: String -> String
toModuleName =
    concatMap capitalize . splitOnDash
  where
    capitalize [] = []
    capitalize (first : rest) = toUpper first : rest

splitOnDash :: String -> [String]
splitOnDash [] = [""]
splitOnDash ('-' : rest) = "" : splitOnDash rest
splitOnDash (char : rest) =
    case splitOnDash rest of
        [] -> [[char]]
        part : parts -> (char : part) : parts

tokenize :: String -> [String]
tokenize contents =
    mapMaybe normalize (wordsBy (not . isTokenChar) contents)
  where
    normalize [] = Nothing
    normalize token = Just (map toLower token)

isTokenChar :: Char -> Bool
isTokenChar char = isAlphaNum char || char `elem` "-_./"

wordsBy :: (Char -> Bool) -> String -> [String]
wordsBy predicate input =
    case dropWhile predicate input of
        [] -> []
        remaining ->
            let (word, rest) = break predicate remaining
             in word : wordsBy predicate rest

trim :: String -> String
trim = dropWhileEnd isWhitespace . dropWhile isWhitespace

dropWhileEnd :: (Char -> Bool) -> String -> String
dropWhileEnd predicate = reverse . dropWhile predicate . reverse

isWhitespace :: Char -> Bool
isWhitespace char = char `elem` [' ', '\n', '\r', '\t']

isPrefixOfWord :: String -> String -> Bool
isPrefixOfWord prefix value = take (length prefix) value == prefix
