{-# LANGUAGE OverloadedStrings #-}

module ResolutionUtf8Spec
    ( resolutionCabalSpec
    , resolutionDartSpec
    , resolutionDotnetSpec
    , resolutionElixirSpec
    , resolutionGoSpec
    , resolutionGradleSpec
    , resolutionPerlSpec
    , resolutionPythonSpec
    , resolutionRubySpec
    , resolutionRustSpec
    , resolutionSwiftSpec
    , resolutionTypescriptSpec
    , resolutionUtf8Spec
    )
where

import Control.Exception (bracket, try)
import Control.Monad (forM_)
import Data.Aeson
    ( FromJSON(..)
    , Object
    , eitherDecodeStrict'
    , withObject
    , (.:)
    , (.:?)
    , (.!=)
    )
import Data.Aeson.Types (Parser)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Base64 as Base64
import qualified Data.ByteString.Char8 as BS8
import Data.List (isInfixOf, sort)
import qualified Data.Text as Text
import qualified Data.Text.Encoding as Text
import qualified DirectedGraph as DG
import System.Directory
    ( createDirectory
    , createDirectoryIfMissing
    , getTemporaryDirectory
    , removeFile
    , removePathForcibly
    )
import System.FilePath ((</>), pathSeparator, takeDirectory)
import System.IO (hClose, openTempFile)
import Test.Hspec

import BuildTool

data FixtureFile = FixtureFile
    { fixturePath :: FilePath
    , fixtureContentUtf8 :: Maybe Text.Text
    , fixtureContentBase64 :: Maybe Text.Text
    }

data ResolutionFixture = ResolutionFixture
    { fixtureFiles :: [FixtureFile]
    , fixtureExpectedEdges :: [[String]]
    }

instance FromJSON FixtureFile where
    parseJSON = withObject "fixture file" $ \object ->
        FixtureFile
            <$> object .: "path"
            <*> object .:? "content_utf8"
            <*> object .:? "content_base64"

instance FromJSON ResolutionFixture where
    parseJSON = withObject "resolution fixture" $ \object -> do
        workspace <- object .: "workspace" :: Parser Object
        expected <- object .: "expected" :: Parser Object
        result <- expected .: "result" :: Parser Object
        ResolutionFixture
            <$> workspace .: "files"
            <*> (result .:? "edges" .!= [])

resolutionUtf8Spec :: Spec
resolutionUtf8Spec = describe "Lua resolution conformance" $ do
    it "consumes the shared valid fixture and resolves only the exact edge" $
        withFixture "resolution-lua-utf8.json" $ \root fixture -> do
            graph <- resolveFixture root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph `shouldBe` ["lua/other", "lua/pkg"]

    it "ignores aliases outside the authoritative dependencies table" $
        withFixture "resolution-lua-field-aware.json" $ \root fixture -> do
            graph <- resolveFixture root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.independentGroups graph
                `shouldBe` Right [["lua/beta", "lua/gamma"], ["lua/alpha"]]

    it "preserves genuine dependency cycles as graph errors" $
        withFixture "resolution-lua-cycle.json" $ \root fixture -> do
            graph <- resolveFixture root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.independentGroups graph `shouldBe` Left DG.CycleError

    it "merges selected BUILD dependency comments without collapsing program identity" $
        withFixture "resolution-build-deps-comment.json" $ \root fixture -> do
            graph <- resolveFixture root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe` ["lua/conduit", "lua/programs/conduit-hello"]

    it "prefers a package alias over a same-basename program alias" $
        withFixture "resolution-lua-program-package.json" $ \root fixture -> do
            graph <- resolveFixture root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "lua/consumer"
                    , "lua/grammar_tools"
                    , "lua/programs/grammar-tools"
                    ]

    it "consumes the shared invalid fixture as a typed stable error" $
        withFixture "resolution-lua-invalid-utf8.json" $ \root _ -> do
            result <- try (resolveFixture root) :: IO (Either MetadataEncodingError DG.DirectedGraph)
            case result of
                Right _ -> expectationFailure "invalid UTF-8 unexpectedly resolved"
                Left metadataError -> assertMetadataError root metadataError

    it "accepts a literal replacement character encoded as valid UTF-8" $
        withTemporaryDirectory "haskell-build-tool-replacement" $ \root -> do
            writeLuaPackage
                root
                ( Text.encodeUtf8
                    "package = \"coding-adventures-pkg\"\nversion = \"0.1.0-1\"\ndescription = { summary = \"Literal U+FFFD: \xFFFD\" }\ndependencies = { \"lua >= 5.4\" }\n"
                )
            graph <- resolveFixture root
            DG.nodes graph `shouldBe` ["lua/pkg"]

    it "rejects representative malformed UTF-8 sequence classes" $ do
        let malformedSequences =
                [ ("illegal leading byte", BS.pack [0xFF])
                , ("unexpected continuation byte", BS.pack [0x80])
                , ("truncated multibyte sequence", BS.pack [0xE2, 0x82])
                , ("overlong encoding", BS.pack [0xC0, 0xAF])
                , ("UTF-16 surrogate encoding", BS.pack [0xED, 0xA0, 0x80])
                ]
        forM_ malformedSequences $ \(label, malformedBytes) ->
            withTemporaryDirectory "haskell-build-tool-malformed" $ \root -> do
                writeLuaPackage
                    root
                    ( BS8.pack
                        "package = \"coding-adventures-pkg\"\nversion = \"0.1.0-1\"\ndependencies = { \"lua >= 5.4\" }\n-- malformed: "
                        <> malformedBytes
                        <> BS8.pack "\n"
                    )
                result <- try (resolveFixture root) :: IO (Either MetadataEncodingError DG.DirectedGraph)
                case result of
                    Right _ -> expectationFailure (label ++ " unexpectedly resolved")
                    Left metadataError -> assertMetadataError root metadataError

    it "maps invalid metadata through the real front-door path to exit code 2" $
        withFixture "resolution-lua-invalid-utf8.json" $ \root _ -> do
            runWithArgs
                [ "--root"
                , root
                , "--language"
                , "lua"
                , "--force"
                , "--dry-run"
                ]
                `shouldReturn` 2

resolutionCabalSpec :: Spec
resolutionCabalSpec = describe "Cabal resolution conformance" $ do
    it "reads only every authoritative build-depends field" $
        withFixture "resolution-haskell-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "haskell" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "haskell/alpha"
                    , "haskell/ambiguous"
                    , "haskell/beta"
                    , "haskell/delta"
                    , "haskell/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["haskell/ambiguous", "haskell/beta", "haskell/delta", "haskell/gamma"]
                        , ["haskell/alpha"]
                        ]

resolutionGoSpec :: Spec
resolutionGoSpec = describe "Go resolution conformance" $ do
    it "reads only require directives" $
        withFixture "resolution-go-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "go" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "go/alpha"
                    , "go/beta-helper"
                    , "go/delta_name"
                    , "go/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["go/beta-helper", "go/delta_name", "go/gamma"]
                        , ["go/alpha"]
                        ]

resolutionElixirSpec :: Spec
resolutionElixirSpec = describe "Elixir resolution conformance" $ do
    it "reads local path tuples only from authoritative deps lists" $
        withFixture "resolution-elixir-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "elixir" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "elixir/alpha"
                    , "elixir/beta-helper"
                    , "elixir/delta_name"
                    , "elixir/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["elixir/beta-helper", "elixir/gamma"]
                        , ["elixir/delta_name"]
                        , ["elixir/alpha"]
                        ]

resolutionDartSpec :: Spec
resolutionDartSpec = describe "Dart resolution conformance" $ do
    it "discovers Dart and reads only direct root dependency keys" $
        withFixture "resolution-dart-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "dart" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "dart/alpha"
                    , "dart/attacker"
                    , "dart/beta-helper"
                    , "dart/delta_name"
                    , "dart/gamma"
                    , "dart/shadowed_name"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ [ "dart/attacker"
                          , "dart/beta-helper"
                          , "dart/delta_name"
                          , "dart/gamma"
                          , "dart/shadowed_name"
                          ]
                        , ["dart/alpha"]
                        ]

resolutionGradleSpec :: Spec
resolutionGradleSpec = describe "Gradle resolution conformance" $ do
    forM_ ["java", "kotlin"] $ \language ->
        it ("reads only same-lane relative includeBuild calls for " ++ language) $
            withFixture ("resolution-gradle-" ++ language ++ "-field-aware.json") $ \root fixture -> do
                graph <- resolveFixtureFor language root
                graphEdges graph `shouldBe` fixtureExpectedEdges fixture
                DG.nodes graph
                    `shouldBe`
                        [ language ++ "/alpha"
                        , language ++ "/beta-helper"
                        , language ++ "/gamma"
                        , language ++ "/programs/delta-app"
                        ]
                DG.independentGroups graph
                    `shouldBe`
                        Right
                            [ [language ++ "/beta-helper", language ++ "/gamma"]
                            , [language ++ "/alpha"]
                            , [language ++ "/programs/delta-app"]
                            ]

resolutionDotnetSpec :: Spec
resolutionDotnetSpec = describe ".NET resolution conformance" $ do
    forM_ ["csharp", "fsharp"] $ \language ->
        it ("reads only root ProjectReference Include paths for " ++ language) $
            withFixture ("resolution-dotnet-" ++ language ++ "-field-aware.json") $ \root fixture -> do
                graph <- resolveFixtureFor language root
                graphEdges graph `shouldBe` fixtureExpectedEdges fixture
                DG.nodes graph
                    `shouldBe`
                        [ language ++ "/alpha"
                        , language ++ "/beta-helper"
                        , language ++ "/gamma"
                        , language ++ "/programs/delta-app"
                        ]
                DG.independentGroups graph
                    `shouldBe`
                        Right
                            [ [language ++ "/beta-helper", language ++ "/gamma"]
                            , [language ++ "/alpha"]
                            , [language ++ "/programs/delta-app"]
                            ]
    it "resolves exact root project paths across the shared .NET scope" $
        withFixture "resolution-dotnet-cross-language-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "all" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "csharp/graph"
                    , "dotnet/programs/bridge-app"
                    , "fsharp/helpers"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["fsharp/helpers"]
                        , ["csharp/graph"]
                        , ["dotnet/programs/bridge-app"]
                        ]

resolutionPythonSpec :: Spec
resolutionPythonSpec = describe "Python resolution conformance" $ do
    it "preserves the shared canonical dependency diamond" $
        withFixture "resolution-python-diamond.json" $ \root fixture -> do
            graph <- resolveFixtureFor "python" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture

    it "reads only PEP 621 dependencies and normalizes distribution names" $
        withFixture "resolution-python-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "python" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "python/alpha"
                    , "python/beta-helper"
                    , "python/delta"
                    , "python/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["python/beta-helper", "python/delta", "python/gamma"]
                        , ["python/alpha"]
                        ]

resolutionRustSpec :: Spec
resolutionRustSpec = describe "Rust resolution conformance" $ do
    it "reads only inline path dependencies in the top-level dependencies table" $
        withFixture "resolution-rust-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "rust" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "rust/alpha"
                    , "rust/beta-helper"
                    , "rust/delta"
                    , "rust/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["rust/beta-helper", "rust/delta", "rust/gamma"]
                        , ["rust/alpha"]
                        ]

resolutionRubySpec :: Spec
resolutionRubySpec = describe "Ruby resolution conformance" $ do
    it "reads only runtime dependency declarations on the gem specification receiver" $
        withFixture "resolution-ruby-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "ruby" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "ruby/alpha"
                    , "ruby/beta"
                    , "ruby/delta"
                    , "ruby/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["ruby/beta", "ruby/delta", "ruby/gamma"]
                        , ["ruby/alpha"]
                        ]

resolutionPerlSpec :: Spec
resolutionPerlSpec = describe "Perl resolution conformance" $ do
    it "reads only top-level cpanfile requirements through authoritative aliases" $
        withFixture "resolution-perl-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "perl" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "perl/alpha"
                    , "perl/beta-helper"
                    , "perl/delta_name"
                    , "perl/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["perl/beta-helper", "perl/delta_name", "perl/gamma"]
                        , ["perl/alpha"]
                        ]

resolutionSwiftSpec :: Spec
resolutionSwiftSpec = describe "Swift resolution conformance" $ do
    it "reads only local package path declarations" $
        withFixture "resolution-swift-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "swift" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "swift/alpha"
                    , "swift/beta-helper"
                    , "swift/delta_name"
                    , "swift/gamma"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ ["swift/beta-helper", "swift/delta_name", "swift/gamma"]
                        , ["swift/alpha"]
                        ]

resolutionTypescriptSpec :: Spec
resolutionTypescriptSpec = describe "TypeScript resolution conformance" $ do
    it "reads only direct root dependency tables through exact manifest aliases" $
        withFixture "resolution-typescript-field-aware.json" $ \root fixture -> do
            graph <- resolveFixtureFor "typescript" root
            graphEdges graph `shouldBe` fixtureExpectedEdges fixture
            DG.nodes graph
                `shouldBe`
                    [ "typescript/alpha"
                    , "typescript/beta-helper"
                    , "typescript/delta_name"
                    , "typescript/gamma"
                    , "typescript/malformed"
                    , "typescript/wrong-shape"
                    ]
            DG.independentGroups graph
                `shouldBe`
                    Right
                        [ [ "typescript/beta-helper"
                          , "typescript/delta_name"
                          , "typescript/gamma"
                          , "typescript/malformed"
                          , "typescript/wrong-shape"
                          ]
                        , ["typescript/alpha"]
                        ]

assertMetadataError :: FilePath -> MetadataEncodingError -> Expectation
assertMetadataError root metadataError = do
    metadataErrorCode metadataError `shouldBe` "METADATA_INVALID_UTF8"
    metadataErrorPackage metadataError `shouldBe` "lua/pkg"
    metadataErrorManifest metadataError
        `shouldBe` "code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec"
    metadataErrorEncoding metadataError `shouldBe` "UTF-8"
    renderMetadataEncodingError metadataError
        `shouldBe` "METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8"
    renderMetadataEncodingError metadataError `shouldSatisfy` (not . isInfixOf root)

resolveFixture :: FilePath -> IO DG.DirectedGraph
resolveFixture = resolveFixtureFor "lua"

resolveFixtureFor :: String -> FilePath -> IO DG.DirectedGraph
resolveFixtureFor language root = do
    packages <- discoverPackages (root </> "code")
    resolveDependencies
        ( if language == "all"
            then packages
            else filter ((== language) . packageLanguage) packages
        )

graphEdges :: DG.DirectedGraph -> [[String]]
graphEdges graph =
    sort
        [ [fromNode, toNode]
        | fromNode <- DG.nodes graph
        , Right toNodes <- [DG.successors fromNode graph]
        , toNode <- toNodes
        ]

withFixture :: FilePath -> (FilePath -> ResolutionFixture -> IO a) -> IO a
withFixture fixtureName action = do
    fixture <- loadFixture fixtureName
    withTemporaryDirectory "haskell-build-tool-fixture" $ \root -> do
        materializeFixture root fixture
        action root fixture

loadFixture :: FilePath -> IO ResolutionFixture
loadFixture fixtureName = do
    maybeRoot <- findRepoRoot Nothing
    repoRoot <- maybe (fail "could not locate repository root for fixtures") pure maybeRoot
    let sharedFixturePath =
            repoRoot
                </> "code"
                </> "specs"
                </> "fixtures"
                </> "build-tool-v1"
                </> "cases"
                </> fixtureName
    bytes <- BS.readFile sharedFixturePath
    either (fail . ("invalid shared fixture: " ++)) pure (eitherDecodeStrict' bytes)

materializeFixture :: FilePath -> ResolutionFixture -> IO ()
materializeFixture root fixture =
    forM_ (fixtureFiles fixture) $ \fixtureFile -> do
        let destination = root </> portableToNative (fixturePath fixtureFile)
        createDirectoryIfMissing True (takeDirectory destination)
        bytes <- fixtureBytes fixtureFile
        BS.writeFile destination bytes

fixtureBytes :: FixtureFile -> IO BS.ByteString
fixtureBytes fixtureFile =
    case (fixtureContentUtf8 fixtureFile, fixtureContentBase64 fixtureFile) of
        (Just content, Nothing) -> pure (Text.encodeUtf8 content)
        (Nothing, Just content) ->
            either (fail . ("invalid fixture base64: " ++)) pure (Base64.decode (Text.encodeUtf8 content))
        _ -> fail "fixture file must contain exactly one content encoding"

writeLuaPackage :: FilePath -> BS.ByteString -> IO ()
writeLuaPackage root rockspecBytes = do
    let packageRoot = root </> "code" </> "packages" </> "lua" </> "pkg"
    createDirectoryIfMissing True packageRoot
    BS8.writeFile (packageRoot </> "BUILD") "echo build\n"
    BS.writeFile
        (packageRoot </> "coding-adventures-pkg-0.1.0-1.rockspec")
        rockspecBytes

portableToNative :: FilePath -> FilePath
portableToNative = map (\character -> if character == '/' then pathSeparator else character)

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
