{-# LANGUAGE OverloadedStrings #-}

module HashingSpec (hashingSpec) where

import Control.Exception (bracket)
import Control.Monad (forM_)
import Data.Aeson (FromJSON(..), Value(..), eitherDecodeStrict', encode, withObject, (.:), (.:?))
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KeyMap
import qualified Data.ByteString as BS
import Data.ByteString.Builder (Builder, char8, lazyByteString, toLazyByteString, word64BE)
import qualified Data.ByteString.Char8 as BS8
import qualified Data.ByteString.Lazy as LBS
import Data.Char (digitToInt, isHexDigit)
import Data.Foldable (toList)
import Data.List (sortOn)
import qualified Data.Map.Strict as Map
import qualified Data.Text.Encoding as TextEncoding
import System.Environment (lookupEnv, setEnv, unsetEnv)
import System.Directory
    ( createDirectory
    , createDirectoryIfMissing
    , doesFileExist
    , getTemporaryDirectory
    , removeFile
    , removePathForcibly
    )
import System.FilePath ((</>), takeDirectory)
import System.IO (hClose, openTempFile)
import qualified Sha256
import Test.Hspec

import BuildTool
import LanguageSourceInputRegistry
    ( LanguageRegistry(..)
    , LanguageRule(..)
    , SelectedSourceFile(..)
    , SourceCandidate(..)
    , SourceCandidateKind(..)
    , SourceCollectionMode(..)
    , SourceCollectionRequest(..)
    , languageSourceInputRegistry
    , languageSourceInputRegistryDigest
    , languageSourceInputRegistryValue
    , selectSourceCandidates
    )

data SourceFixture = SourceFixture
    { fixtureId :: String
    , fixtureDomain :: String
    , fixtureInput :: FixtureInput
    , fixtureExpected :: FixtureExpected
    }

instance FromJSON SourceFixture where
    parseJSON = withObject "source fixture" $ \object ->
        SourceFixture
            <$> object .: "id"
            <*> object .: "domain"
            <*> object .: "input"
            <*> object .: "expected"

data FixtureInput = FixtureInput
    { fixtureOperation :: String
    , fixtureOptions :: FixtureOptions
    }

instance FromJSON FixtureInput where
    parseJSON = withObject "source fixture input" $ \object ->
        FixtureInput <$> object .: "operation" <*> object .: "options"

data FixtureOptions = FixtureOptions
    { optionLanguage :: String
    , optionPackageRoot :: FilePath
    , optionMode :: String
    , optionRegistryDigest :: String
    , optionDeclaredSources :: [String]
    , optionCandidates :: [FixtureCandidate]
    }

instance FromJSON FixtureOptions where
    parseJSON = withObject "source fixture options" $ \object ->
        FixtureOptions
            <$> object .: "language"
            <*> object .: "package_root"
            <*> object .: "mode"
            <*> object .: "registry_sha256"
            <*> object .: "declared_srcs"
            <*> object .: "candidates"

data FixtureCandidate = FixtureCandidate
    { fixtureCandidatePath :: FilePath
    , fixtureCandidateKind :: String
    , fixtureCandidateContentHex :: Maybe String
    }

instance FromJSON FixtureCandidate where
    parseJSON = withObject "source fixture candidate" $ \object ->
        FixtureCandidate
            <$> object .: "path"
            <*> object .: "kind"
            <*> object .:? "content_hex"

data FixtureExpected = FixtureExpected
    { expectedCaseId :: String
    , expectedDomain :: String
    , expectedOutcome :: String
    , expectedResult :: FixtureResult
    , expectedDiagnostics :: [Value]
    }

instance FromJSON FixtureExpected where
    parseJSON = withObject "source fixture expected" $ \object ->
        FixtureExpected
            <$> object .: "case_id"
            <*> object .: "domain"
            <*> object .: "outcome"
            <*> object .: "result"
            <*> object .: "diagnostics"

newtype FixtureResult = FixtureResult {expectedFiles :: [ExpectedFile]}

instance FromJSON FixtureResult where
    parseJSON = withObject "source fixture result" $ \object ->
        FixtureResult <$> object .: "files"

data ExpectedFile = ExpectedFile
    { expectedPath :: FilePath
    , expectedDigest :: String
    }

instance FromJSON ExpectedFile where
    parseJSON = withObject "expected source file" $ \object ->
        ExpectedFile <$> object .: "path" <*> object .: "digest"

hashingSpec :: Spec
hashingSpec = describe "package hashing" $ do
    it "preserves non-ASCII, NUL, and malformed source bytes" $
        withBinaryPackage $ \pkg -> do
            digest <- hashPackage pkg
            digest `shouldBe` "b3cb0c3c9fd4b67978ba69fdaa4cbc6b21c3af89928ef770042d293a44faf5d4"

    it "uses the same local SHA-256 digest when git is unavailable" $
        withBinaryPackage $ \pkg ->
            withPath (packagePath pkg) $ do
                digest <- hashPackage pkg
                digest `shouldBe` "b3cb0c3c9fd4b67978ba69fdaa4cbc6b21c3af89928ef770042d293a44faf5d4"

    it "uses the standard SHA-256 digest for an empty package" $
        withSourcePackage "haskell" [] $ \pkg ->
            hashPackage pkg
                `shouldReturn` "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    it "matches the shared Hashing v1 package digest oracle" $
        hashSourceSnapshot [("code/packages/python/demo/src/data.bin", "abc")]
            `shouldBe` "6a2dc6b1e5428211f6f19387deb1c84836e61384745e7e0e5986fe49b17ff24f"

    it "streams exact bytes across the 8 KiB read boundary" $
        forM_ [8191, 8192, 8193] $ \byteCount -> do
            let contents = BS.replicate byteCount 0xA5
            withSourcePackage "haskell" [("src/Main.hs", contents)] $ \pkg ->
                hashPackage pkg
                    `shouldReturn`
                        hashSourceSnapshot
                            [("code/packages/haskell/demo/src/Main.hs", contents)]

    it "makes same-content renames observable" $ do
        let contents = "module Main where\n"
        hashSourceSnapshot [("code/packages/haskell/demo/src/Main.hs", contents)]
            `shouldNotBe`
                hashSourceSnapshot [("code/packages/haskell/demo/src/Renamed.hs", contents)]

    it "sorts multi-file Hashing v1 frames by raw UTF-8 path bytes" $ do
        let inputs =
                [ ("code/packages/haskell/demo/src/\x1f600.hs", "bc")
                , ("code/packages/haskell/demo/src/\xe000.hs", "a")
                ]
            expected = "83a0d53e1a37c12e99a3b9a661a4465e2013456551004a5d53878375341213aa"
        hashSourceSnapshot inputs `shouldBe` expected
        hashSourceSnapshot (reverse inputs) `shouldBe` expected

    it "recognizes OCaml source and root metadata" $
        withSourcePackage
            "ocaml"
            [("BUILD", "dune runtest\n"), ("src/main.ml", "let value = 1\n"), ("dune-project", "(lang dune 3.0)\n")]
            $ \pkg -> do
                digestBefore <- hashPackage pkg
                BS8.writeFile (packagePath pkg </> "src" </> "main.ml") "let value = 2\n"
                digestAfter <- hashPackage pkg
                digestAfter `shouldNotBe` digestBefore

    it "includes every universal BUILD front" $
        withSourcePackage
            "haskell"
            [ ("BUILD", "cabal test\n")
            , ("BUILD_linux", "cabal test linux\n")
            , ("BUILD_mac", "cabal test mac\n")
            , ("BUILD_mac_and_linux", "cabal test unix\n")
            , ("BUILD_windows", "cabal test windows\n")
            ]
            $ \pkg -> do
                initial <- hashPackage pkg
                BS8.appendFile (packagePath pkg </> "BUILD_mac") "# changed\n"
                digestAfter <- hashPackage pkg
                digestAfter `shouldNotBe` initial

    it "prunes exact generated components but preserves case-near authored directories" $
        withSourcePackage
            "lua"
            [ ("BUILD", "lua test.lua\n")
            , ("generated/_build/generated.lua", "return 1\n")
            , ("authored/_Build/authored.lua", "return 1\n")
            ]
            $ \pkg -> do
                digestBefore <- hashPackage pkg
                BS8.writeFile (packagePath pkg </> "generated" </> "_build" </> "generated.lua") "return 2\n"
                afterGenerated <- hashPackage pkg
                afterGenerated `shouldBe` digestBefore
                BS8.writeFile (packagePath pkg </> "authored" </> "_Build" </> "authored.lua") "return 2\n"
                afterAuthored <- hashPackage pkg
                afterAuthored `shouldNotBe` afterGenerated

    it "fails closed for an unknown language" $
        withSourcePackage "unknown" [("BUILD", "echo no\n")] $ \pkg ->
            hashPackage pkg `shouldThrow` anyIOException

    it "embeds a production registry equal to the checked complete registry" $ do
        maybeRoot <- findRepoRoot Nothing
        repoRoot <- maybe (fail "could not locate repository root") pure maybeRoot
        bytes <-
            BS.readFile
                ( repoRoot
                    </> "code"
                    </> "specs"
                    </> "fixtures"
                    </> "build-tool-v1"
                    </> "language-source-input-registry.json"
                )
        checked <- either (fail . ("invalid registry fixture: " ++)) pure (eitherDecodeStrict' bytes :: Either String Value)
        languageSourceInputRegistryValue `shouldBe` checked
        canonicalRegistryDigest checked `shouldBe` languageSourceInputRegistryDigest
        languageSourceInputRegistryDigest
            `shouldBe` "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70"

    it "makes every embedded registry language reachable from discovery" $ do
        let rules = registryLanguages languageSourceInputRegistry
            languages = map ruleLanguage rules
        length languages `shouldBe` 23
        length (Map.fromList [(language, ()) | language <- languages]) `shouldBe` 23
        forM_ languages $ \language ->
            inferLanguage ("code" </> "packages" </> language </> "demo")
                `shouldBe` language

    it "consumes every package-local source collection fixture through production selection" $
        forM_ packageLocalSourceFixtureNames $ \fixtureName -> do
            fixture <- loadSourceFixture fixtureName
            fixtureDomain fixture `shouldBe` "source_collection"
            fixtureOperation (fixtureInput fixture) `shouldBe` "source_collection"
            expectedCaseId (fixtureExpected fixture) `shouldBe` fixtureId fixture
            expectedDomain (fixtureExpected fixture) `shouldBe` "source_collection"
            expectedOutcome (fixtureExpected fixture) `shouldBe` "ok"
            expectedDiagnostics (fixtureExpected fixture) `shouldBe` []
            request <- either fail pure (fixtureRequest (fixtureOptions (fixtureInput fixture)))
            actual <- either fail pure (selectSourceCandidates request)
            map (\file -> (selectedSourcePath file, selectedSourceDigest file)) actual
                `shouldBe`
                    map
                        (\file -> (expectedPath file, expectedDigest file))
                        (expectedFiles (expectedResult (fixtureExpected fixture)))

    it "implements portable declared-source character classes" $ do
        let regular path = SourceCandidate path RegularCandidate "source\n"
            request =
                SourceCollectionRequest
                    { sourceCollectionLanguage = "csharp"
                    , sourceCollectionPackageRoot = "code/packages/csharp/demo"
                    , sourceCollectionMode = DeclaredSourcesMode
                    , sourceCollectionRegistryDigest = languageSourceInputRegistryDigest
                    , sourceCollectionDeclaredSources =
                        [ "src/[!a].cs"
                        , "src/[-a].cs"
                        , "src/[a-].cs"
                        , "src/[a-c].cs"
                        , "src/[^].cs"
                        , "src/[]].cs"
                        ]
                    , sourceCollectionCandidates =
                        map regular ["src/-.cs", "src/^.cs", "src/].cs", "src/a.cs", "src/b.cs", "src/c.cs"]
                    }
        selected <- either fail pure (selectSourceCandidates request)
        map selectedSourcePath selected
            `shouldBe` ["src/-.cs", "src/].cs", "src/^.cs", "src/a.cs", "src/b.cs", "src/c.cs"]

    it "rejects invalid globs and bounds declared-source match work" $ do
        let regular path = SourceCandidate path RegularCandidate "source\n"
            request patterns candidates =
                SourceCollectionRequest
                    { sourceCollectionLanguage = "csharp"
                    , sourceCollectionPackageRoot = "code/packages/csharp/demo"
                    , sourceCollectionMode = DeclaredSourcesMode
                    , sourceCollectionRegistryDigest = languageSourceInputRegistryDigest
                    , sourceCollectionDeclaredSources = patterns
                    , sourceCollectionCandidates = candidates
                    }
            invalid patternValue =
                selectSourceCandidates (request [patternValue] [])
                    `shouldBe` Left "SOURCE_HASH_GLOB_INVALID"
            pad3 value = replicate (3 - length rendered) '0' ++ rendered
              where
                rendered = show value
            expensivePatterns =
                [ "unmatched/" ++ replicate 220 'a' ++ pad3 index ++ "*.cs"
                | index <- [0 :: Int .. 255]
                ]
            ordinaryCandidates =
                [ regular ("src/file" ++ pad3 index ++ ".cs")
                | index <- [0 :: Int .. 99]
                ]
        mapM_ invalid ["CON", "src/[a--!].cs", "src/[z-a].cs"]
        selectSourceCandidates (request ["src/[.cs"] [regular "src/[.cs"])
            `shouldSatisfy` either (const False) ((== ["src/[.cs"]) . map selectedSourcePath)
        selectSourceCandidates (request expensivePatterns ordinaryCandidates)
            `shouldBe` Left "SOURCE_HASH_LIMIT_EXCEEDED"

    it "rejects impossible candidate topology and hostile portable paths" $ do
        let regular path = SourceCandidate path RegularCandidate "source\n"
            request candidates =
                SourceCollectionRequest
                    { sourceCollectionLanguage = "ocaml"
                    , sourceCollectionPackageRoot = "code/packages/ocaml/demo"
                    , sourceCollectionMode = ExtensionMode
                    , sourceCollectionRegistryDigest = languageSourceInputRegistryDigest
                    , sourceCollectionDeclaredSources = []
                    , sourceCollectionCandidates = candidates
                    }
        selectSourceCandidates (request [regular "src/main.ml", regular "src/main.ml"])
            `shouldBe` Left "SOURCE_HASH_PATH_COLLISION"
        selectSourceCandidates (request [regular "src", regular "src/main.ml"])
            `shouldBe` Left "SOURCE_HASH_PATH_COLLISION"
        forM_
            [ "C:/src/main.ml"
            , "src/main.ml:stream"
            , "src/CON.ml"
            , "src/name."
            , "src/name "
            , "src/\x202e\&evil.ml"
            , "src/*.ml"
            , replicate 513 'a' ++ ".ml"
            ]
            $ \path ->
                selectSourceCandidates (request [regular path])
                    `shouldBe` Left "SOURCE_HASH_PATH_INVALID"
        selectSourceCandidates
            (request [])
                { sourceCollectionPackageRoot = "code/packages/rust/demo"
                }
            `shouldBe` Left "SOURCE_HASH_PACKAGE_ROOT_INVALID"

    it "retains fixed and package-exact inputs in declared-source mode" $ do
        let regular path = SourceCandidate path RegularCandidate "source\n"
            request =
                SourceCollectionRequest
                    { sourceCollectionLanguage = "rust"
                    , sourceCollectionPackageRoot = "code/packages/rust/engram-wasm"
                    , sourceCollectionMode = DeclaredSourcesMode
                    , sourceCollectionRegistryDigest = languageSourceInputRegistryDigest
                    , sourceCollectionDeclaredSources = []
                    , sourceCollectionCandidates =
                        map
                            regular
                            [ "BUILD"
                            , "build-wasm.sh"
                            , "js/engram-mosaic-host-wasm.mjs"
                            , "js/smoke.mjs"
                            , "pkg/engram_engine.wasm"
                            , "src/lib.rs"
                            ]
                    }
        selected <- either fail pure (selectSourceCandidates request)
        map selectedSourcePath selected
            `shouldBe`
                [ "BUILD"
                , "build-wasm.sh"
                , "js/engram-mosaic-host-wasm.mjs"
                , "js/smoke.mjs"
                , "pkg/engram_engine.wasm"
                ]

    it "fails closed on registry drift, unknown languages, and portable path aliases" $ do
        let request candidates =
                SourceCollectionRequest
                    { sourceCollectionLanguage = "ocaml"
                    , sourceCollectionPackageRoot = "code/packages/ocaml/demo"
                    , sourceCollectionMode = ExtensionMode
                    , sourceCollectionRegistryDigest = languageSourceInputRegistryDigest
                    , sourceCollectionDeclaredSources = []
                    , sourceCollectionCandidates = candidates
                    }
            regular path = SourceCandidate path RegularCandidate "source\n"
        selectSourceCandidates
            (request []){sourceCollectionRegistryDigest = replicate 64 '0'}
            `shouldBe` Left "SOURCE_HASH_REGISTRY_MISMATCH"
        selectSourceCandidates
            (request []){sourceCollectionLanguage = "unknown"}
            `shouldBe` Left "SOURCE_HASH_LANGUAGE_UNKNOWN"
        selectSourceCandidates
            (request [regular "Straße.ml", regular "STRASSE.ml"])
            `shouldBe` Left "SOURCE_HASH_PATH_COLLISION"
        selectSourceCandidates
            (request [regular "Cafe\x0301.ml"])
            `shouldBe` Left "SOURCE_HASH_PATH_INVALID"
        selectSourceCandidates
            (request [regular ['\xD800']])
            `shouldBe` Left "SOURCE_HASH_PATH_INVALID"

    it "invalidates Java hashes when Gradle settings change" $
        withGradlePackage $ \pkg -> do
            digestBefore <- hashPackage pkg
            BS8.writeFile
                (packagePath pkg </> "settings.gradle.kts")
                "includeBuild(\"../other-dependency\")\n"
            digestAfter <- hashPackage pkg
            digestAfter `shouldNotBe` digestBefore

withBinaryPackage :: (Package -> IO a) -> IO a
withBinaryPackage action =
    withTemporaryDirectory "haskell-build-tool-hashing" $ \packageRoot -> do
        let buildFile = packageRoot </> "BUILD"
        let sourceFile = packageRoot </> "src" </> "main.lua"
        let sourceBytes =
                TextEncoding.encodeUtf8 "-- Café — 雪\nreturn {}\n"
                    <> BS.pack [0x00, 0x80, 0xFF]
        createDirectoryIfMissing True (packageRoot </> "src")
        BS8.writeFile buildFile "echo build\n"
        BS.writeFile sourceFile sourceBytes
        action
            Package
                { packageName = "lua/demo"
                , packagePath = packageRoot
                , packageBuildFile = buildFile
                , packageBuildCommands = ["echo build"]
                , packageLanguage = "lua"
                }

withGradlePackage :: (Package -> IO a) -> IO a
withGradlePackage action =
    withTemporaryDirectory "haskell-build-tool-gradle-hashing" $ \packageRoot -> do
        let buildFile = packageRoot </> "BUILD"
        createDirectoryIfMissing True (packageRoot </> "src")
        BS8.writeFile buildFile "gradle test\n"
        BS8.writeFile (packageRoot </> "settings.gradle.kts") "includeBuild(\"../dependency\")\n"
        BS8.writeFile (packageRoot </> "build.gradle.kts") "plugins {}\n"
        BS8.writeFile (packageRoot </> "src" </> "Main.java") "class Main {}\n"
        action
            Package
                { packageName = "java/demo"
                , packagePath = packageRoot
                , packageBuildFile = buildFile
                , packageBuildCommands = ["gradle test"]
                , packageLanguage = "java"
                }

withSourcePackage :: String -> [(FilePath, BS.ByteString)] -> (Package -> IO a) -> IO a
withSourcePackage language files action =
    withTemporaryDirectory "haskell-build-tool-source-hashing" $ \packageRoot -> do
        mapM_ (writeInput packageRoot) files
        let buildFile = packageRoot </> "BUILD"
        buildExists <- doesFileExist buildFile
        action
            Package
                { packageName = language ++ "/demo"
                , packagePath = packageRoot
                , packageBuildFile = buildFile
                , packageBuildCommands = if buildExists then ["test"] else []
                , packageLanguage = language
                }
  where
    writeInput packageRoot (relative, contents) = do
        let path = packageRoot </> relative
        createDirectoryIfMissing True (takeDirectory path)
        BS.writeFile path contents

withPath :: String -> IO a -> IO a
withPath value action = bracket replace restore (const action)
  where
    replace = do
        original <- lookupEnv "PATH"
        setEnv "PATH" value
        pure original
    restore (Just original) = setEnv "PATH" original
    restore Nothing = unsetEnv "PATH"

packageLocalSourceFixtureNames :: [FilePath]
packageLocalSourceFixtureNames =
    [ "source-collection-extension.json"
    , "source-collection-declared.json"
    , "source-collection-registry-roles.json"
    , "source-collection-engram-wasm-exact-inputs.json"
    ]

loadSourceFixture :: FilePath -> IO SourceFixture
loadSourceFixture fixtureName = do
    maybeRoot <- findRepoRoot Nothing
    repoRoot <- maybe (fail "could not locate repository root") pure maybeRoot
    bytes <-
        BS.readFile
            ( repoRoot
                </> "code"
                </> "specs"
                </> "fixtures"
                </> "build-tool-v1"
                </> "cases"
                </> fixtureName
            )
    either (fail . ("invalid source fixture: " ++)) pure (eitherDecodeStrict' bytes)

fixtureRequest :: FixtureOptions -> Either String SourceCollectionRequest
fixtureRequest options = do
    mode <-
        case optionMode options of
            "extension" -> Right ExtensionMode
            "declared_sources" -> Right DeclaredSourcesMode
            _ -> Left "unsupported source collection mode"
    candidates <- traverse fixtureCandidate (optionCandidates options)
    pure
        SourceCollectionRequest
            { sourceCollectionLanguage = optionLanguage options
            , sourceCollectionPackageRoot = optionPackageRoot options
            , sourceCollectionMode = mode
            , sourceCollectionRegistryDigest = optionRegistryDigest options
            , sourceCollectionDeclaredSources = optionDeclaredSources options
            , sourceCollectionCandidates = candidates
            }

fixtureCandidate :: FixtureCandidate -> Either String SourceCandidate
fixtureCandidate candidate =
    case (fixtureCandidateKind candidate, fixtureCandidateContentHex candidate) of
        ("file", Just encoded) ->
            SourceCandidate
                (fixtureCandidatePath candidate)
                RegularCandidate
                <$> decodeHex encoded
        ("symlink", _) ->
            Right (SourceCandidate (fixtureCandidatePath candidate) SymlinkCandidate BS.empty)
        ("reparse_point", _) ->
            Right (SourceCandidate (fixtureCandidatePath candidate) ReparseCandidate BS.empty)
        ("file", Nothing) -> Left "regular source fixture candidate has no content_hex"
        _ -> Left "unsupported source fixture candidate kind"

decodeHex :: String -> Either String BS.ByteString
decodeHex value
    | odd (length value) || any (not . isHexDigit) value = Left "invalid source fixture content_hex"
    | otherwise = BS.pack <$> go value
  where
    go [] = Right []
    go (high : low : rest) =
        ((fromIntegral (digitToInt high * 16 + digitToInt low)) :) <$> go rest
    go _ = Left "invalid source fixture content_hex"

canonicalRegistryDigest :: Value -> String
canonicalRegistryDigest value =
    Sha256.sha256FinalizeHex $
        foldl
            Sha256.sha256Update
            Sha256.sha256Init
            [ BS8.pack "coding-adventures/build-tool-language-source-input-registry/v1"
            , BS.singleton 0
            , LBS.toStrict (toLazyByteString (word64BE (fromIntegral (BS.length canonical))))
            , canonical
            ]
  where
    canonical = LBS.toStrict (toLazyByteString (canonicalJsonBuilder value))

canonicalJsonBuilder :: Value -> Builder
canonicalJsonBuilder (Object object) =
    char8 '{'
        <> commaSeparated
            [ lazyByteString (encode (Key.toText key))
                <> char8 ':'
                <> canonicalJsonBuilder child
            | (key, child) <-
                sortOn
                    (TextEncoding.encodeUtf8 . Key.toText . fst)
                    (KeyMap.toList object)
            ]
        <> char8 '}'
canonicalJsonBuilder (Array values) =
    char8 '['
        <> commaSeparated (map canonicalJsonBuilder (toList values))
        <> char8 ']'
canonicalJsonBuilder primitive = lazyByteString (encode primitive)

commaSeparated :: [Builder] -> Builder
commaSeparated [] = mempty
commaSeparated (first : rest) = first <> foldMap (char8 ',' <>) rest

withTemporaryDirectory :: String -> (FilePath -> IO a) -> IO a
withTemporaryDirectory template = bracket create removePathForcibly
  where
    create = do
        temporaryRoot <- getTemporaryDirectory
        (reservedPath, handle) <- openTempFile temporaryRoot template
        hClose handle
        removeFile reservedPath
        createDirectory reservedPath
        pure reservedPath
