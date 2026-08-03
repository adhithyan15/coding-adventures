{-# LANGUAGE OverloadedStrings #-}

module ResolutionUtf8Spec (resolutionUtf8Spec) where

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
resolveFixture root = do
    packages <- discoverPackages (root </> "code")
    resolveDependencies (filter ((== "lua") . packageLanguage) packages)

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
