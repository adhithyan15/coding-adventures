-- | TC-9: cross-implementation round-trip against the real @zstd@ CLI binary.
--
-- Every other test in this package (see "ZstdSpec") only ever round-trips
-- data through this package's OWN 'compress' \/ 'decompress' pair. That
-- proves internal consistency but nothing about RFC 8878 conformance: an
-- encoder and decoder that agree with each other about a made-up bit
-- layout will pass every such test while still being wire-incompatible
-- with the real format.
--
-- This module is what actually proves the wire format is real RFC 8878.
-- It shells out to the system @zstd@ binary (via "System.Process") in both
-- directions:
--
--   1. Compress with 'compress', decompress with @zstd -d@.
--   2. Compress with @zstd@, decompress with 'decompress'.
--
-- Lesson 95 (see @lessons.md@) documents the compounding FSE
-- sequences-codec bugs (and a separate Frame_Header_Descriptor
-- checksum-bit bug) that were found in the sibling Java\/Rust ports and
-- confirmed, via this test file, to have been independently present here
-- too. They were invisible to every self-round-trip test in the suite,
-- because both the encoder and the decoder agreed on the same wrong
-- convention.
--
-- Gracefully skipped (marked pending, not failed) when the @zstd@ binary
-- isn't on @PATH@, since CI\/dev environments vary and this package's
-- correctness does not depend on the binary being present.
module ZstdCliInteropSpec (spec) where

import Control.Exception (IOException, finally, try)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import System.Directory (getTemporaryDirectory, removeFile)
import System.Exit (ExitCode (ExitSuccess))
import System.IO (hClose, openBinaryTempFile)
import System.Process (readProcessWithExitCode)
import Test.Hspec
import Zstd (compress, decompress)

spec :: Spec
spec = describe "real zstd CLI interoperability (TC-9)" $ do
    it "round-trips ordinary prose with the real zstd CLI in both directions" $
        withZstdCli $ do
            let original = BSC.pack (concat (replicate 25 "the quick brown fox jumps over the lazy dog "))
            interopBothDirections original

    it "round-trips a high sequence count (2-byte sequence-count wire form) via the real zstd CLI" $
        withZstdCli $ do
            -- A repeating 6-byte cycle across 9 KB gives our LZSS-based
            -- matcher plenty of short, distinct matches -- comfortably more
            -- than 128 sequences in one block, past the RFC 8878
            -- SS3.1.1.3.1 boundary where the sequence-count wire encoding
            -- switches from its 1-byte form to its 2-byte form. A
            -- byte-order bug in that 2-byte form would round-trip fine
            -- against itself but be silently non-conformant, so only a
            -- real interop check like this one can catch it.
            let cycle6 = BSC.pack "ABCDEF"
                original = BS.take 9000 (BS.concat (replicate 1501 cycle6))
            oursZst <- expectRight (compress original)
            decodedByCli <-
                runInteropForward "our compressed output" oursZst
            decodedByCli `shouldBe` original

-- | Direction 1 (compress here, decompress with the CLI) AND direction 2
-- (compress with the CLI, decompress here) for one input.
interopBothDirections :: BS.ByteString -> Expectation
interopBothDirections original = do
    -- Direction 1: compress with ours, decompress with `zstd -d`.
    oursZst <- expectRight (compress original)
    decodedByCli <- runInteropForward "our compressed output" oursZst
    decodedByCli `shouldBe` original

    -- Direction 2: compress with `zstd`, decompress with ours.
    theirCompressed <- compressWithCli original
    case decompress theirCompressed of
        Left err ->
            expectationFailure
                ("our decompress() failed to decode real zstd's compressed output: " ++ err)
        Right decodedByUs -> decodedByUs `shouldBe` original

-- | Feed compressed bytes to @zstd -d@ and return the decoded bytes.
--
-- 'readProcessWithExitCode' treats stdout as a 'String', which mangles
-- binary data through the text codec -- so both this function and
-- 'compressWithCli' route the binary payload through temp files (@-o@ on
-- the way out) rather than piping bytes through the process's textual
-- stdout. Only 'stderr', which is always human-readable text, is captured
-- directly for error messages. Cleanup runs via 'finally' so a failed CLI
-- invocation or a decode/read exception doesn't leak temp files into the
-- shared system temp directory.
runInteropForward :: String -> BS.ByteString -> IO BS.ByteString
runInteropForward label compressed = do
    tempDir <- getTemporaryDirectory
    (inPath, inHandle) <- openBinaryTempFile tempDir "zstd-haskell-tc9-input.zst"
    BS.hPut inHandle compressed
    hClose inHandle
    (outPath, outHandle) <- openBinaryTempFile tempDir "zstd-haskell-tc9-decoded.bin"
    hClose outHandle
    let cleanup = removeFile inPath >> removeFile outPath
    ( do
            (exitCode, _stdout, stderr) <-
                readProcessWithExitCode "zstd" ["-d", "-q", "-f", "-o", outPath, inPath] ""
            case exitCode of
                ExitSuccess -> BS.readFile outPath
                _ -> fail ("real `zstd -d` failed to decode " ++ label ++ ": " ++ stderr)
        )
        `finally` cleanup

-- | Compress bytes with the real @zstd@ CLI and return the raw frame bytes.
-- See 'runInteropForward' for why cleanup runs via 'finally'.
compressWithCli :: BS.ByteString -> IO BS.ByteString
compressWithCli original = do
    tempDir <- getTemporaryDirectory
    (inPath, inHandle) <- openBinaryTempFile tempDir "zstd-haskell-tc9-input.txt"
    BS.hPut inHandle original
    hClose inHandle
    (outPath, outHandle) <- openBinaryTempFile tempDir "zstd-haskell-tc9-output.zst"
    hClose outHandle
    let cleanup = removeFile inPath >> removeFile outPath
    ( do
            (exitCode, _stdout, stderr) <-
                readProcessWithExitCode "zstd" ["-q", "-c", "-f", "-o", outPath, inPath] ""
            case exitCode of
                ExitSuccess -> BS.readFile outPath
                _ -> fail ("real `zstd -c` failed to compress input: " ++ stderr)
        )
        `finally` cleanup

-- | Run an interop check, but mark the test pending (not failed) when the
-- @zstd@ binary isn't reachable on @PATH@ -- CI/dev environments vary, and
-- this package's own correctness does not depend on the binary existing.
withZstdCli :: Expectation -> Expectation
withZstdCli action = do
    available <- isZstdCliAvailable
    if available
        then action
        else pendingWith "zstd CLI not found on PATH -- skipping interop test"

isZstdCliAvailable :: IO Bool
isZstdCliAvailable = do
    result <- try (readProcessWithExitCode "zstd" ["--version"] "") :: IO (Either IOException (ExitCode, String, String))
    pure (either (const False) (\(code, _, _) -> code == ExitSuccess) result)

expectRight :: (Show failure) => Either failure value -> IO value
expectRight (Left failure) = expectationFailure (show failure) >> fail "unreachable"
expectRight (Right value) = pure value
