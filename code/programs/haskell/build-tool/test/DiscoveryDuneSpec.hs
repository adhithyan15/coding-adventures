{-# LANGUAGE OverloadedStrings #-}

module DiscoveryDuneSpec (discoveryDuneSpec) where

import BuildTool (Package (..), discoverPackages, findRepoRoot)
import Control.Exception (bracket)
import Control.Monad (forM_, unless)
import Data.Aeson (FromJSON (..), Object, eitherDecodeStrict', withObject, (.:))
import Data.Aeson.Types (Parser)
import qualified Data.ByteString as Bytes
import Data.List (isPrefixOf, sort)
import qualified Data.Text as Text
import qualified Data.Text.Encoding as Text
import System.Directory
    (canonicalizePath, createDirectory, createDirectoryIfMissing, getTemporaryDirectory, removeFile, removePathForcibly)
import System.FilePath ((</>), isAbsolute, makeRelative, pathSeparator, splitDirectories, takeDirectory)
import System.IO (hClose, openTempFile)
import Test.Hspec (Spec, describe, it, shouldBe)

data FixtureFile = FixtureFile FilePath Text.Text
data ExpectedPackage = ExpectedPackage String FilePath FilePath
data DiscoveryFixture = DiscoveryFixture [FixtureFile] [ExpectedPackage]

instance FromJSON FixtureFile where
    parseJSON = withObject "fixture file" $ \object ->
        FixtureFile <$> object .: "path" <*> object .: "content_utf8"

instance FromJSON ExpectedPackage where
    parseJSON = withObject "expected package" $ \object ->
        ExpectedPackage <$> object .: "name" <*> object .: "build_file" <*> object .: "rel_path"

instance FromJSON DiscoveryFixture where
    parseJSON = withObject "discovery fixture" $ \object -> do
        workspace <- object .: "workspace" :: Parser Object
        expected <- object .: "expected" :: Parser Object
        result <- expected .: "result" :: Parser Object
        DiscoveryFixture <$> workspace .: "files" <*> result .: "packages"

discoveryDuneSpec :: Spec
discoveryDuneSpec = describe "shared Dune discovery fixture" $ do
    it "rejects fixture paths that could escape the temporary root" $ do
        safeFixturePath "code/packages/ocaml/../../../../outside/BUILD" `shouldBe` False
        safeFixturePath "code/packages/ocaml/..\\outside/BUILD" `shouldBe` False
        safeFixturePath "code/packages/ocaml/C:/outside/BUILD" `shouldBe` False

    it "excludes exact _build components but keeps near-case OCaml source paths" $ do
        repoRoot <- findRepoRoot Nothing >>= maybe (fail "repository root not found") pure
        let fixturePath = repoRoot </> "code" </> "specs" </> "fixtures"
                </> "build-tool-v1" </> "cases" </> "discovery-language-registry.json"
        bytes <- Bytes.readFile fixturePath
        fixture <- either (fail . ("invalid discovery fixture: " ++)) pure (eitherDecodeStrict' bytes)
        let DiscoveryFixture allFiles allExpected = fixture
            ocamlPath = isPrefixOf "code/packages/ocaml/"
            files = filter (\(FixtureFile path _) -> ocamlPath path) allFiles
            expected = filter (\(ExpectedPackage _ buildFile _) -> ocamlPath buildFile) allExpected
        length files `shouldBe` 4
        length expected `shouldBe` 3
        withTemporaryDirectory "haskell-dune-discovery" $ \root -> do
            canonicalRoot <- canonicalizePath root
            forM_ files $ \(FixtureFile path content) -> do
                unless (safeFixturePath path) $ fail ("unsafe discovery fixture path: " ++ show path)
                let destination = canonicalRoot </> portableToNative path
                createDirectoryIfMissing True (takeDirectory destination)
                Bytes.writeFile destination (Text.encodeUtf8 content)
            packages <- discoverPackages (canonicalRoot </> "code")
            let actual = sort
                    [ (packageName package, portable (makeRelative canonicalRoot (packageBuildFile package)),
                       portable (makeRelative canonicalRoot (packagePath package)))
                    | package <- packages
                    ]
                wanted = sort
                    [ (name, buildFile, relPath)
                    | ExpectedPackage name buildFile relPath <- expected
                    ]
            actual `shouldBe` wanted
            map (\(name, _, _) -> name) actual
                `shouldBe` ["ocaml/case-source", "ocaml/demo-ocaml", "ocaml/near-source"]

portableToNative :: FilePath -> FilePath
portableToNative = map (\character -> if character == '/' then pathSeparator else character)

safeFixturePath :: FilePath -> Bool
safeFixturePath path =
    let native = portableToNative path
     in isPrefixOf "code/packages/ocaml/" path
            && not (isAbsolute native)
            && not (any (`elem` ("\\:" :: String)) path)
            && all (`notElem` ["", ".", ".."]) (splitDirectories native)

portable :: FilePath -> FilePath
portable = map (\character -> if character == pathSeparator then '/' else character)

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
