{-# LANGUAGE OverloadedStrings #-}

module DiscoveryRegistrySpec (discoveryRegistrySpec) where

import BuildTool (Package(..), discoverPackages, findRepoRoot, inferLanguage)
import Control.Exception (bracket)
import Control.Monad (forM_, unless)
import Data.Aeson (FromJSON(..), Object, eitherDecodeStrict', withObject, (.:))
import Data.Aeson.Types (Parser)
import qualified Data.ByteString as Bytes
import Data.Char (isLower, toUpper)
import Data.List (isPrefixOf, nub, sort)
import qualified Data.Text as Text
import qualified Data.Text.Encoding as Text
import System.Directory
    ( canonicalizePath, createDirectory, createDirectoryIfMissing
    , getTemporaryDirectory, removeFile, removePathForcibly
    )
import System.FilePath ((</>), isAbsolute, makeRelative, pathSeparator, takeDirectory)
import System.IO (hClose, openTempFile)
import Test.Hspec (Spec, describe, it, shouldBe)

data FixtureFile = FixtureFile FilePath Text.Text
data ExpectedPackage = ExpectedPackage String String FilePath FilePath Bool
data DiscoveryFixture = DiscoveryFixture String String [FixtureFile] [ExpectedPackage] Int
data Registry = Registry [String] [String]

instance FromJSON FixtureFile where
    parseJSON = withObject "fixture file" $ \object ->
        FixtureFile <$> object .: "path" <*> object .: "content_utf8"

instance FromJSON ExpectedPackage where
    parseJSON = withObject "expected package" $ \object ->
        ExpectedPackage <$> object .: "name" <*> object .: "language"
            <*> object .: "rel_path" <*> object .: "build_file" <*> object .: "is_starlark"

instance FromJSON DiscoveryFixture where
    parseJSON = withObject "discovery fixture" $ \object -> do
        workspace <- object .: "workspace" :: Parser Object
        expected <- object .: "expected" :: Parser Object
        result <- expected .: "result" :: Parser Object
        limits <- object .: "limits" :: Parser Object
        DiscoveryFixture <$> object .: "id" <*> object .: "domain"
            <*> workspace .: "files" <*> result .: "packages" <*> limits .: "workspace_bytes"

instance FromJSON Registry where
    parseJSON = withObject "source registry" $ \object -> do
        universal <- object .: "universal_inputs" :: Parser Object
        languages <- object .: "languages" :: Parser [Object]
        Registry <$> universal .: "generated_directory_components"
            <*> mapM (.: "language") languages

discoveryRegistrySpec :: Spec
discoveryRegistrySpec = describe "discovery/language-registry" $ do
    it "compares every native tuple against the full checked fixture" $ do
        DiscoveryFixture caseId domain files expected byteLimit <- loadFixture "cases/discovery-language-registry.json"
        caseId `shouldBe` "discovery/language-registry"
        domain `shouldBe` "discovery"
        length files `shouldBe` 29
        length expected `shouldBe` 25
        let expectedPaths = concat [[root, buildFile] | ExpectedPackage _ _ root buildFile _ <- expected]
        all safeFixturePath expectedPaths `shouldBe` True
        -- This fixture publishes shell BUILD records, including the starlark
        -- language bucket; it does not publish evaluated Starlark metadata.
        all (\(ExpectedPackage _ _ _ _ flag) -> not flag) expected `shouldBe` True
        withFixture files byteLimit $ \root -> do
            packages <- discoverPackages (root </> "code")
            projection root packages `shouldBe` sort
                [(name, language, relPath, buildFile)
                | ExpectedPackage name language relPath buildFile _ <- expected]

    it "prunes every checked exact generated component before BUILD membership" $ do
        Registry components _ <- loadFixture "language-source-input-registry.json"
        length components `shouldBe` 27
        forM_ components $ \component -> do
            let prefix = "code/packages/haskell/"
                files = [build (prefix ++ component ++ "/shallow")
                        ,build (prefix ++ "deep/" ++ component ++ "/nested")
                        ,build (prefix ++ "near/" ++ component ++ "-example/kept")]
            withFixture files 1048576 $ \root -> do
                packages <- discoverPackages (root </> "code")
                map packageName packages `shouldBe` ["haskell/kept"]

    it "keeps case-near generated names in separate branches on case-insensitive hosts" $ do
        Registry components _ <- loadFixture "language-source-input-registry.json"
        forM_ components $ \component -> do
            let near = changeFirstLetter component
            unless (near /= component) $ fail "generated component has no letter"
            withFixture [build ("code/packages/haskell/excluded/" ++ component ++ "/gone")
                        ,build ("code/packages/haskell/source/" ++ near ++ "/kept")] 1048576 $ \root -> do
                packages <- discoverPackages (root </> "code")
                map packageName packages `shouldBe` ["haskell/kept"]

    it "discovers package and program identities for all checked registry languages" $ do
        Registry _ languages <- loadFixture "language-source-input-registry.json"
        length languages `shouldBe` 23
        let files = concat [[build ("code/packages/" ++ language ++ "/demo")
                            ,build ("code/programs/" ++ language ++ "/demo")]
                           | language <- languages]
        withFixture files 1048576 $ \root -> do
            packages <- discoverPackages (root </> "code")
            map (\package -> (packageName package, packageLanguage package)) packages
                `shouldBe` sort (concat [[(language ++ "/demo", language)
                                        ,(language ++ "/programs/demo", language)]
                                       | language <- languages])

    it "uses only code-root buckets despite ancestor and later language names" $ do
        withFixture [build "rust/code/packages/kotlin/demo"
                    ,build "rust/code/packages/custom/go"
                    ,build "rust/code/programs/kotlin/demo"
                    ,build "rust/code/packages/kotlin/programs/rust/MixedCase"] 1048576 $ \root -> do
            packages <- discoverPackages (root </> "rust" </> "code")
            map (\package -> (packageName package, packageLanguage package)) packages
                `shouldBe` [("kotlin/MixedCase", "kotlin"), ("kotlin/demo", "kotlin")
                           ,("kotlin/programs/demo", "kotlin"), ("unknown/go", "unknown")]

    it "preserves the rooted public language API with exact structural anchors" $ do
        forM_ [("/repo/rust/code/packages/kotlin/demo", "kotlin")
              ,("code/programs/ocaml/demo", "ocaml")
              ,("packages/java/demo", "java")
              ,("code/packages/custom/go", "unknown")
              ,("code/packages/Kotlin/demo", "unknown")
              ,("/repo/rust/loose", "unknown")
              ,("code/specs/fixtures/ocaml/BUILD", "unknown")
              ,("code/packages/rust/outer/code/programs/java/demo", "java")] $ \(path, wanted) ->
            inferLanguage path `shouldBe` wanted

    it "preserves the actual configured-root basename when that root contains BUILD" $ do
        withFixture [build "code/packages/custom/ConfiguredRoot"] 1048576 $ \root -> do
            packages <- discoverPackages (root </> "code/packages/custom/ConfiguredRoot")
            map (\package -> (packageName package, packageLanguage package)) packages
                `shouldBe` [("unknown/ConfiguredRoot", "unknown")]

    it "validates the complete bounded materialization before any write" $ do
        forM_ ["/outside/BUILD", "code/packages/go/../escape/BUILD"
              ,"code/packages/go//BUILD", "code/packages/go/./BUILD"
              ,"code/packages/go/C:/BUILD", "code/packages/go/..\\BUILD"
              ,"outside/BUILD"] $ \path -> safeFixturePath path `shouldBe` False
        validFiles [build "code/packages/go/demo", build "code/packages/go/demo"] 1048576 `shouldBe` False
        validFiles [FixtureFile "code/packages/go/demo/BUILD" "\955"] 1 `shouldBe` False
        validFiles [FixtureFile "code/packages/go/demo/BUILD" "\955"] 2 `shouldBe` True
        let uniqueFiles count = [build ("code/packages/go/demo-" ++ show n) | n <- [1 .. count :: Int]]
        validFiles (uniqueFiles 64) 1048576 `shouldBe` True
        validFiles (uniqueFiles 65) 1048576 `shouldBe` False

build :: FilePath -> FixtureFile
build root = FixtureFile (root ++ "/BUILD") "echo fixture-data-only\n"

loadFixture :: FromJSON a => FilePath -> IO a
loadFixture relative = do
    repo <- findRepoRoot Nothing >>= maybe (fail "repository root not found") pure
    bytes <- Bytes.readFile (repo </> "code/specs/fixtures/build-tool-v1" </> relative)
    either (fail . ("invalid checked discovery data: " ++)) pure (eitherDecodeStrict' bytes)

-- Transport data never chooses an absolute destination. Split portable input
-- before normalization so empty and dot components cannot disappear silently.
safeFixturePath :: FilePath -> Bool
safeFixturePath path =
    let components = splitPortable path
     in not (isAbsolute path) && not (any (`elem` ("\\:\0" :: String)) path)
            && all (`notElem` ["", ".", ".."]) components
            && case components of
                "code" : subtree : _ : _ -> subtree `elem` ["packages", "programs", "specs"]
                -- The ancestor-language regression adds one inert rust prefix.
                "rust" : "code" : subtree : _ : _ -> subtree `elem` ["packages", "programs"]
                _ -> False

splitPortable :: String -> [String]
splitPortable input = case break (== '/') input of
    (part, []) -> [part]
    (part, _ : rest) -> part : splitPortable rest

validFiles :: [FixtureFile] -> Int -> Bool
validFiles files byteLimit =
    let paths = [path | FixtureFile path _ <- files]
        byteCount = sum [Bytes.length (Text.encodeUtf8 content) | FixtureFile _ content <- files]
     in length files <= 64 && all safeFixturePath paths
            && length paths == length (nub paths) && byteLimit >= 0
            && byteLimit <= 1048576 && byteCount <= byteLimit

withFixture :: [FixtureFile] -> Int -> (FilePath -> IO a) -> IO a
withFixture files byteLimit action = do
    unless (validFiles files byteLimit) $ fail "unsafe or oversized discovery fixture"
    bracket create removePathForcibly $ \reserved -> do
        root <- canonicalizePath reserved
        -- Validate every joined destination before creating any parent/file.
        let destinations = [root </> native path | FixtureFile path _ <- files]
            beneath destination = (root ++ [pathSeparator]) `isPrefixOf` destination
        unless (all beneath destinations) $ fail "fixture escaped reserved temporary root"
        forM_ (zip destinations files) $ \(destination, FixtureFile _ content) -> do
            createDirectoryIfMissing True (takeDirectory destination)
            Bytes.writeFile destination (Text.encodeUtf8 content)
        action root
  where
    create = do
        temporary <- getTemporaryDirectory
        (reserved, handle) <- openTempFile temporary "haskell-registry"
        hClose handle
        removeFile reserved
        createDirectory reserved
        pure reserved

native :: FilePath -> FilePath
native = map (\character -> if character == '/' then pathSeparator else character)

projection :: FilePath -> [Package] -> [(String, String, FilePath, FilePath)]
projection root packages =
    [(packageName package, packageLanguage package
     ,portable (makeRelative root (packagePath package))
     ,portable (makeRelative root (packageBuildFile package))) | package <- packages]
  where
    portable = map (\character -> if character == pathSeparator then '/' else character)

changeFirstLetter :: String -> String
changeFirstLetter [] = []
changeFirstLetter (character : rest)
    | isLower character = toUpper character : rest
    | character >= 'A' && character <= 'Z' = toEnum (fromEnum character + 32) : rest
    | otherwise = character : changeFirstLetter rest
