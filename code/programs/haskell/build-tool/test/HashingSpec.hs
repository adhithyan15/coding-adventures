{-# LANGUAGE OverloadedStrings #-}

module HashingSpec (hashingSpec) where

import Control.Exception (bracket)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BS8
import qualified Data.Text.Encoding as TextEncoding
import System.Environment (lookupEnv, setEnv, unsetEnv)
import System.Directory
    ( createDirectory
    , createDirectoryIfMissing
    , getTemporaryDirectory
    , removeFile
    , removePathForcibly
    )
import System.FilePath ((</>))
import System.IO (hClose, openTempFile)
import Test.Hspec

import BuildTool

hashingSpec :: Spec
hashingSpec = describe "package hashing" $ do
    it "preserves non-ASCII, NUL, and malformed source bytes" $
        withBinaryPackage $ \pkg -> do
            digest <- hashPackage pkg
            digest `shouldBe` "2c75841be2e79fd42f33be508798c1a75b4030e3"

    it "falls back deterministically when git is unavailable" $
        withBinaryPackage $ \pkg ->
            withPath "" $ do
                digest <- hashPackage pkg
                digest `shouldBe` "1986673100"

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

withPath :: String -> IO a -> IO a
withPath value action = bracket replace restore (const action)
  where
    replace = do
        original <- lookupEnv "PATH"
        setEnv "PATH" value
        pure original
    restore (Just original) = setEnv "PATH" original
    restore Nothing = unsetEnv "PATH"

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
