{-# LANGUAGE OverloadedStrings #-}

module HashingSpec (hashingSpec) where

import Control.Exception (bracket)
import Data.Aeson (Value, eitherDecodeStrict')
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BS8
import qualified Data.Text.Encoding as TextEncoding
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
import Test.Hspec

import BuildTool
import LanguageSourceInputRegistry
    ( languageSourceInputRegistryDigest
    , languageSourceInputRegistryValue
    )

hashingSpec :: Spec
hashingSpec = describe "package hashing" $ do
    it "preserves non-ASCII, NUL, and malformed source bytes" $
        withBinaryPackage $ \pkg -> do
            digest <- hashPackage pkg
            digest `shouldBe` "b3cb0c3c9fd4b67978ba69fdaa4cbc6b21c3af89928ef770042d293a44faf5d4"

    it "uses the same local SHA-256 digest when git is unavailable" $
        withBinaryPackage $ \pkg -> do
            digest <- hashPackage pkg
            digest `shouldBe` "b3cb0c3c9fd4b67978ba69fdaa4cbc6b21c3af89928ef770042d293a44faf5d4"

    it "uses the standard SHA-256 digest for an empty package" $
        withSourcePackage "haskell" [] $ \pkg ->
            hashPackage pkg
                `shouldReturn` "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    it "recognizes OCaml source and root metadata" $
        withSourcePackage
            "ocaml"
            [("BUILD", "dune runtest\n"), ("src/main.ml", "let value = 1\n"), ("dune-project", "(lang dune 3.0)\n")]
            $ \pkg -> do
                before <- hashPackage pkg
                BS8.writeFile (packagePath pkg </> "src" </> "main.ml") "let value = 2\n"
                after <- hashPackage pkg
                after `shouldNotBe` before

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
                after <- hashPackage pkg
                after `shouldNotBe` initial

    it "prunes exact generated components but preserves case-near authored directories" $
        withSourcePackage
            "lua"
            [ ("BUILD", "lua test.lua\n")
            , ("_build/generated.lua", "return 1\n")
            , ("_Build/authored.lua", "return 1\n")
            ]
            $ \pkg -> do
                before <- hashPackage pkg
                BS8.writeFile (packagePath pkg </> "_build" </> "generated.lua") "return 2\n"
                afterGenerated <- hashPackage pkg
                afterGenerated `shouldBe` before
                BS8.writeFile (packagePath pkg </> "_Build" </> "authored.lua") "return 2\n"
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
        languageSourceInputRegistryDigest
            `shouldBe` "f49bfe8c7c9c0fb9b534ecc9ca4a614f3684abe32bdb0edac82d99bdc806fb70"

    it "invalidates Java hashes when Gradle settings change" $
        withGradlePackage $ \pkg -> do
            before <- hashPackage pkg
            BS8.writeFile
                (packagePath pkg </> "settings.gradle.kts")
                "includeBuild(\"../other-dependency\")\n"
            after <- hashPackage pkg
            after `shouldNotBe` before

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
