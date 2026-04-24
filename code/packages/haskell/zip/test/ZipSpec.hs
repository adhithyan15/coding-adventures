-- | Hspec test suite for the Zip package (CMP09).
--
-- Covers 12 test cases:
--
--   TC-1:  round-trip single file, Stored (compress=False)
--   TC-2:  round-trip single file, DEFLATE (repetitive text)
--   TC-3:  multiple files in one archive
--   TC-4:  directory entry (name ends with \/)
--   TC-5:  CRC-32 mismatch detected (corrupt byte → Left error)
--   TC-6:  random-access read (10 files, read only f5.txt)
--   TC-7:  incompressible data stored as method=0
--   TC-8:  empty file
--   TC-9:  large file (100 KB repetitive)
--   TC-10: Unicode filename (UTF-8 bytes)
--   TC-11: nested paths
--   TC-12: empty archive

module ZipSpec (spec) where

import Test.Hspec
import Data.Bits (xor)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC
import Data.Word (Word8)

import Zip

-- ─── Helpers ──────────────────────────────────────────────────────────────────

-- | Round-trip using zip' / unzip'.
rt :: [(ByteString, ByteString)] -> Either String [(ByteString, ByteString)]
rt = unzip' . zip'

-- | Flip all bits of the byte at position @i@ in a ByteString.
flipByte :: ByteString -> Int -> ByteString
flipByte bs i =
    let (pre, rest) = BS.splitAt i bs
    in case BS.uncons rest of
        Nothing       -> bs
        Just (b, suf) -> BS.concat [pre, BS.singleton (b `xor` 0xFF), suf]

-- ─── Test suite ───────────────────────────────────────────────────────────────

spec :: Spec
spec = describe "Zip" $ do

    -- TC-1: round-trip single file, Stored (compress=False)
    it "TC-1: single file stored round-trip" $ do
        let name = BC.pack "hello.txt"
            dat  = BC.pack "hello, world!"
            arch = writeZip [(name, dat, False)]
        case readZip arch of
            Left err -> fail err
            Right entries -> do
                length entries `shouldBe` 1
                entryData (head entries) `shouldBe` dat
                entryName (head entries) `shouldBe` name

    -- TC-2: round-trip single file, DEFLATE (repetitive text)
    it "TC-2: single file DEFLATE round-trip" $ do
        let name = BC.pack "text.txt"
            dat  = BC.concat (replicate 20
                        (BC.pack "the quick brown fox jumps over the lazy dog "))
        let arch = writeZip [(name, dat, True)]
        case readZip arch of
            Left err -> fail err
            Right entries -> do
                entryData (head entries) `shouldBe` dat

    -- TC-3: multiple files in one archive
    it "TC-3: multiple files round-trip" $ do
        let files = [ (BC.pack "a.txt", BC.pack "file A")
                    , (BC.pack "b.txt", BC.pack "file B")
                    , (BC.pack "c.txt", BC.pack "file C")
                    ]
        case rt files of
            Left err  -> fail err
            Right got -> do
                length got `shouldBe` 3
                lookup (BC.pack "a.txt") got `shouldBe` Just (BC.pack "file A")
                lookup (BC.pack "b.txt") got `shouldBe` Just (BC.pack "file B")
                lookup (BC.pack "c.txt") got `shouldBe` Just (BC.pack "file C")

    -- TC-4: directory entry (name ends with /)
    it "TC-4: directory entry is preserved" $ do
        let arch = writeZip
                    [ (BC.pack "mydir/",         BS.empty, False)
                    , (BC.pack "mydir/file.txt", BC.pack "contents", True)
                    ]
        case readZip arch of
            Left err -> fail err
            Right entries -> do
                let names = map entryName entries
                names `shouldContain` [BC.pack "mydir/"]
                names `shouldContain` [BC.pack "mydir/file.txt"]
                let dirEntry = head (filter (\e -> entryName e == BC.pack "mydir/") entries)
                entryData dirEntry `shouldBe` BS.empty

    -- TC-5: CRC-32 mismatch detected (corrupt byte -> Left error)
    --
    -- We write a Stored (compress=False) entry and corrupt a byte inside the
    -- file data region. The Local File Header is 30 bytes; the filename "f.txt"
    -- is 5 bytes; so the data starts at offset 35.  Flipping a byte at 35
    -- changes the decompressed content so the CRC check must fail.
    it "TC-5: CRC-32 mismatch is detected" $ do
        let dat  = BC.pack "test data for crc check"
            arch = writeZip [(BC.pack "f.txt", dat, False)]
        let corrupted = flipByte arch 35
        case readZip corrupted of
            Left err -> err `shouldContain` "CRC"
            Right _  -> fail "Expected CRC error but got success"

    -- TC-6: random-access read (10 files, read only f5.txt)
    it "TC-6: random-access read of specific entry" $ do
        let files = [ (BC.pack ("f" ++ show i ++ ".txt"),
                       BC.pack ("content " ++ show i),
                       True)
                    | i <- [0..9 :: Int] ]
            arch  = writeZip files
        case readEntry arch (BC.pack "f5.txt") of
            Left err -> fail err
            Right d  -> d `shouldBe` BC.pack "content 5"

    -- TC-7: incompressible data stored as method=0
    --
    -- Pseudo-random bytes via a linear congruential generator are essentially
    -- incompressible with DEFLATE, so the library must fall back to method=0
    -- (Stored).  We verify round-trip correctness regardless.
    it "TC-7: incompressible data falls back to Stored" $ do
        let dat = BS.pack (map fromIntegral (take 1024 (iterate lcgStep (42 :: Int))))
        let arch = writeZip [(BC.pack "rand.bin", dat, True)]
        case readZip arch of
            Left err -> fail err
            Right entries ->
                entryData (head entries) `shouldBe` dat

    -- TC-8: empty file
    it "TC-8: empty file round-trips" $ do
        let arch = writeZip [(BC.pack "empty.txt", BS.empty, True)]
        case readZip arch of
            Left err -> fail err
            Right entries -> do
                length entries `shouldBe` 1
                entryData (head entries) `shouldBe` BS.empty

    -- TC-9: large file (100 KB repetitive)
    it "TC-9: large repetitive file compresses and round-trips" $ do
        let dat  = BS.concat (replicate 10000 (BC.pack "abcdefghij"))  -- 100 KB
            arch = writeZip [(BC.pack "big.bin", dat, True)]
        -- The archive should be smaller than the raw data.
        BS.length arch `shouldSatisfy` (< BS.length dat)
        case readZip arch of
            Left err -> fail err
            Right entries -> entryData (head entries) `shouldBe` dat

    -- TC-10: Unicode filename (UTF-8 bytes)
    --
    -- ZIP flags bit 11 signals UTF-8 filenames; we just verify that arbitrary
    -- byte sequences survive the round-trip unchanged.
    it "TC-10: Unicode filename survives round-trip" $ do
        -- UTF-8 encoding of "日本語/résumé.txt"
        let name = BS.pack
                    [ 0xe6, 0x97, 0xa5  -- 日
                    , 0xe6, 0x9c, 0xac  -- 本
                    , 0xe8, 0xaa, 0x9e  -- 語
                    , 0x2f              -- /
                    , 0x72              -- r
                    , 0xc3, 0xa9        -- é
                    , 0x73, 0x75, 0x6d  -- sum
                    , 0xc3, 0xa9        -- é
                    , 0x2e, 0x74, 0x78, 0x74  -- .txt
                    ]
            dat  = BC.pack "unicode content"
            arch = writeZip [(name, dat, False)]
        case readZip arch of
            Left err -> fail err
            Right entries -> do
                entryName (head entries) `shouldBe` name
                entryData (head entries) `shouldBe` dat

    -- TC-11: nested paths
    it "TC-11: nested path entries round-trip" $ do
        let files = [ (BC.pack "root.txt",          BC.pack "root")
                    , (BC.pack "dir/file.txt",       BC.pack "nested")
                    , (BC.pack "dir/sub/deep.txt",   BC.pack "deep")
                    ]
        case rt files of
            Left err  -> fail err
            Right got -> do
                lookup (BC.pack "root.txt")          got
                    `shouldBe` Just (BC.pack "root")
                lookup (BC.pack "dir/file.txt")      got
                    `shouldBe` Just (BC.pack "nested")
                lookup (BC.pack "dir/sub/deep.txt")  got
                    `shouldBe` Just (BC.pack "deep")

    -- TC-12: empty archive
    it "TC-12: empty archive round-trips" $ do
        let arch = writeZip []
        case unzip' arch of
            Left err  -> fail err
            Right got -> got `shouldBe` []

-- ─── LCG helper (for TC-7) ────────────────────────────────────────────────────

-- | One step of a 32-bit linear congruential generator (Knuth parameters).
lcgStep :: Int -> Int
lcgStep s = (s * 1664525 + 1013904223) `mod` (2 ^ (32 :: Int))
