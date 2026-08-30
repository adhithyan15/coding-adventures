module ToolchainDetection
    ( ToolchainDiagnostic(..)
    , ToolchainPackage(..)
    , ToolchainResult(..)
    , canonicalToolchains
    , evaluateToolchainSnapshot
    , parseExtraToolchains
    ) where

import qualified Data.ByteString as BS
import Data.List (find, foldl', isPrefixOf)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import qualified Data.Text as Text
import qualified Data.Text.Encoding as TextEncoding

data ToolchainPackage = ToolchainPackage
    { toolchainPackageName :: String
    , toolchainPackageLanguage :: String
    , toolchainPackageBuildFiles :: Map FilePath String
    }
    deriving (Eq, Show)

data ToolchainDiagnostic = ToolchainDiagnostic
    { toolchainDiagnosticCode :: String
    , toolchainDiagnosticSeverity :: String
    , toolchainDiagnosticPackage :: Maybe String
    }
    deriving (Eq, Show)

data ToolchainResult = ToolchainResult
    { toolchainOutcome :: String
    , toolchainFlags :: Map String Bool
    , toolchainDiagnostics :: [ToolchainDiagnostic]
    }
    deriving (Eq, Show)

canonicalToolchains :: [String]
canonicalToolchains =
    [ "cpp", "dart", "dotnet", "elixir", "go", "haskell", "java"
    , "kotlin", "lua", "ocaml", "perl", "python", "ruby", "rust"
    , "swift", "typescript"
    ]

evaluateToolchainSnapshot
    :: String
    -> Bool
    -> [ToolchainPackage]
    -> Maybe [String]
    -> [String]
    -> ToolchainResult
evaluateToolchainSnapshot platform forceFull packages scheduled forced =
    validateSnapshotLimits packages `seq` case firstUnsupported selected of
        Just packageName -> failure (Just packageName)
        Nothing ->
            case find (`notElem` canonicalToolchains) forced of
                Just _ -> failure Nothing
                Nothing ->
                    ToolchainResult
                        "ok"
                        (foldl' enable initial (forced ++ inferred))
                        []
  where
    selected = maybe packages (selectPackages packages) scheduled
    initial = Map.fromList [(name, forceFull) | name <- canonicalToolchains]
    inferred
        | forceFull = []
        | otherwise = concatMap packageToolchains selected
    enable flags name = Map.insert name True flags
    failure packageName =
        ToolchainResult
            "error"
            Map.empty
            [ToolchainDiagnostic "TOOLCHAIN_UNSUPPORTED" "error" packageName]
    packageToolchains package =
        case languageToolchain (toolchainPackageLanguage package) of
            Nothing -> []
            Just primary -> primary : selectedDeclarations platform package

selectPackages :: [ToolchainPackage] -> [String] -> [ToolchainPackage]
selectPackages packages names =
    let wanted = Set.fromList names
     in filter ((`Set.member` wanted) . toolchainPackageName) packages

firstUnsupported :: [ToolchainPackage] -> Maybe String
firstUnsupported packages =
    toolchainPackageName <$> find (isUnsupported . toolchainPackageLanguage) packages
  where
    isUnsupported language = languageToolchain language == Nothing

languageToolchain :: String -> Maybe String
languageToolchain language
    | language == "wasm" = Just "rust"
    | language `elem` ["c", "cpp"] = Just "cpp"
    | language `elem` ["csharp", "fsharp", "dotnet"] = Just "dotnet"
    | language `elem` canonicalToolchains = Just language
    | otherwise = Nothing

selectedDeclarations :: String -> ToolchainPackage -> [String]
selectedDeclarations platform package =
    maybe [] parseExtraToolchains selectedContent
  where
    files = toolchainPackageBuildFiles package
    candidates = case platform of
        "darwin" -> ["BUILD_mac", "BUILD_mac_and_linux", "BUILD"]
        "linux" -> ["BUILD_linux", "BUILD_mac_and_linux", "BUILD"]
        "windows" -> ["BUILD_windows", "BUILD"]
        "win32" -> ["BUILD_windows", "BUILD"]
        _ -> []
    selectedContent = snd <$> find ((`Map.member` files) . fst) [(name, files Map.! name) | name <- candidates, Map.member name files]

parseExtraToolchains :: String -> [String]
parseExtraToolchains content
    | utf8ByteLength content > 65536 = []
    | logicalLineCount content > 4096 = []
    | otherwise = reverse (fst (foldl' collect ([], Set.empty) linesWithTerminators))
  where
    linesWithTerminators = splitLines content
    collect (found, seen) (rawLine, terminated) =
        let line = trimAscii (stripCR terminated rawLine)
            prefix = "# needs-toolchain:"
         in if prefix `isPrefixOf` line
                then
                    let suffix = drop (length prefix) line
                        name = trimAscii suffix
                     in if not (null suffix)
                            && head suffix `elem` [' ', '\t']
                            && name `elem` canonicalToolchains
                            && not (Set.member name seen)
                            then (name : found, Set.insert name seen)
                            else (found, seen)
                else (found, seen)

splitLines :: String -> [(String, Bool)]
splitLines [] = [("", False)]
splitLines value = case break (== '\n') value of
    (line, []) -> [(line, False)]
    (line, _ : rest) -> (line, True) : splitLines rest

stripCR :: Bool -> String -> String
stripCR True value = case reverse value of
    '\r' : rest -> reverse rest
    _ -> value
stripCR False value = value

trimAscii :: String -> String
trimAscii = dropWhile (`elem` [' ', '\t']) . reverse . dropWhile (`elem` [' ', '\t']) . reverse

utf8ByteLength :: String -> Int
utf8ByteLength = BS.length . TextEncoding.encodeUtf8 . Text.pack

logicalLineCount :: String -> Int
logicalLineCount = (+ 1) . length . filter (== '\n')

validateSnapshotLimits :: [ToolchainPackage] -> ()
validateSnapshotLimits packages
    | any exceedsPerFile contents =
        error "toolchain BUILD snapshot exceeds its per-file resource ceiling"
    | sum (map utf8ByteLength contents) > 1048576 =
        error "toolchain BUILD snapshot exceeds its aggregate resource ceiling"
    | otherwise = ()
  where
    contents = concatMap (Map.elems . toolchainPackageBuildFiles) packages
    exceedsPerFile content =
        utf8ByteLength content > 65536 || logicalLineCount content > 4096
