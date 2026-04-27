module ContentAddressableStorageSpec (spec) where

import qualified Data.ByteString.Char8 as BSC
import System.Directory
    ( createDirectory
    , getTemporaryDirectory
    , removeFile
    , removePathForcibly
    )
import System.IO (hClose, openTempFile)
import Test.Hspec

import ContentAddressableStorage

spec :: Spec
spec = do
    describe "hex utilities" $ do
        it "round-trips a SHA-1 key through hex" $ do
            let key = sha1Key (BSC.pack "hello")
            keyToHex key `shouldBe` "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
            hexToKey (keyToHex key) `shouldBe` Right key

    describe "MemoryStore" $ do
        it "stores and retrieves blobs by content hash" $ do
            store <- newMemoryStore
            let cas = newContentAddressableStore store
            putResult <- putContent cas (BSC.pack "hello, world")
            case putResult of
                Left err -> expectationFailure (show err)
                Right key -> do
                    getContent cas key `shouldReturn` Right (BSC.pack "hello, world")
                    existsContent cas key `shouldReturn` Right True

        it "detects corrupted blobs on read" $ do
            store <- newMemoryStore
            let cas = newContentAddressableStore store
            putResult <- putContent cas (BSC.pack "hello")
            case putResult of
                Left err -> expectationFailure (show err)
                Right key -> do
                    putBlob store key (BSC.pack "tampered") `shouldReturn` Right ()
                    getContent cas key `shouldReturn` Left (CasCorrupted key)

        it "finds keys by unique prefix and rejects ambiguous prefixes" $ do
            store <- newMemoryStore
            let cas = newContentAddressableStore store
            Right keyA <- putContent cas (BSC.pack "alpha")
            Right keyB <- putContent cas (BSC.pack "beta")
            let prefixA = take 8 (keyToHex keyA)
                ambiguousPrefix = sharedPrefix (keyToHex keyA) (keyToHex keyB)
            findByPrefix cas prefixA `shouldReturn` Right keyA
            if null ambiguousPrefix
                then pure ()
                else findByPrefix cas ambiguousPrefix `shouldReturn` Left (CasAmbiguousPrefix ambiguousPrefix)

    describe "LocalDiskStore" $
        it "round-trips blobs on disk" $ do
            tempDir <- withTempDirectory "cas"
            store <- newLocalDiskStore tempDir
            let cas = newContentAddressableStore store
            Right key <- putContent cas (BSC.pack "disk blob")
            getContent cas key `shouldReturn` Right (BSC.pack "disk blob")
            removePathForcibly tempDir

withTempDirectory :: String -> IO FilePath
withTempDirectory label = do
    tempRoot <- getTemporaryDirectory
    (path, handle) <- openTempFile tempRoot (label ++ "-XXXXXX")
    hClose handle
    removeFile path
    createDirectory path
    pure path

sharedPrefix :: String -> String -> String
sharedPrefix left right =
    takeWhileSame left right
  where
    takeWhileSame (l : ls) (r : rs)
        | l == r = l : takeWhileSame ls rs
        | otherwise = []
    takeWhileSame _ _ = []
