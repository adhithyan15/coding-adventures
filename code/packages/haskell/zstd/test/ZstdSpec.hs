-- | Test suite for the Zstd (CMP07) Haskell package.
--
-- Covers round-trip correctness across a variety of input types:
-- empty, single-byte, all 256 byte values, RLE blocks, prose, pseudo-random
-- bytes, multi-block (>128 KB), repeat-offset patterns, determinism, and
-- repeated pattern data.

module ZstdSpec (spec) where

import Test.Hspec
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC
import Data.ByteString (ByteString)
import Data.Bits ((.&.))
import Data.Word (Word8)

import Zstd (compress, decompress)

-- | Convert a 'Char' to a 'Word8' (ASCII range only).
c2w :: Char -> Word8
c2w = fromIntegral . fromEnum

-- | Round-trip helper: compress then decompress, failing on error.
rt :: ByteString -> ByteString
rt bs = case decompress (compress bs) of
    Right result -> result
    Left  err    -> error ("round-trip failed: " ++ err)

spec :: Spec
spec = describe "Zstd" $ do

    -- TC-1: empty input
    it "round-trips empty input" $ do
        -- An empty input must produce a valid ZStd frame and decompress back
        -- to empty bytes without panic or error.
        rt BS.empty `shouldBe` BS.empty

    -- TC-2: single byte
    it "round-trips a literal" $ do
        -- The smallest non-empty input: one byte.
        rt (BS.singleton 0x42) `shouldBe` BS.singleton 0x42

    -- TC-3: all 256 byte values
    it "round-trips all 256 bytes" $ do
        -- Every possible byte value 0x00..=0xFF in order. Exercises
        -- literal encoding of non-ASCII and zero bytes.
        let input = BS.pack [0..255]
        rt input `shouldBe` input

    -- TC-4: RLE block (identical bytes)
    it "round-trips RLE block (identical bytes)" $ do
        -- 1024 identical bytes should be detected as an RLE block.
        -- Compressed size should be tiny (< 30 bytes).
        let input      = BS.replicate 1024 65  -- 1024 x 'A'
            compressed = compress input
        decompress compressed `shouldBe` Right input
        BS.length compressed `shouldSatisfy` (< 30)

    -- TC-5: English prose (repetitive text)
    it "round-trips prose (repetitive text)" $ do
        -- Repeated English text has strong LZ77 matches. Must achieve >= 20%
        -- compression (output <= 80% of input size).
        let text       = BS.concat (replicate 25 (BC.pack "the quick brown fox jumps over the lazy dog "))
            compressed = compress text
        decompress compressed `shouldBe` Right text
        let threshold = BS.length text * 80 `div` 100
        BS.length compressed `shouldSatisfy` (< threshold)

    -- TC-6: pseudo-random bytes
    it "round-trips pseudo-random bytes" $ do
        -- LCG pseudo-random bytes. No significant compression expected, but
        -- round-trip must be exact regardless of block type chosen.
        let lcgBytes = take 512 (iterate lcgStep 42)
            input    = BS.pack (map lcgByte lcgBytes)
        rt input `shouldBe` input

    -- TC-7: 300 KB multi-block
    it "round-trips 300 KB (multiblock)" $ do
        -- 300 KB > MAX_BLOCK_SIZE (128 KB), requiring at least 3 blocks.
        -- Uses 'x' repeated, so all blocks should be RLE.
        let input = BS.replicate (300 * 1024) (c2w 'x')
        rt input `shouldBe` input

    -- TC-8: repeat-offset pattern
    it "round-trips repeat-offset pattern" $ do
        -- Alternating pattern with long runs of 'X' and repeated "ABCDEFGH".
        -- Strong LZ77 matches → should compress well (< 70% of input).
        let pattern = BC.pack "ABCDEFGH"
            xRun    = BS.replicate 128 (c2w 'X')
            input   = mconcat (pattern : concatMap (\_ -> [xRun, pattern]) [1..10 :: Int])
            compressed = compress input
        decompress compressed `shouldBe` Right input
        let threshold = BS.length input * 70 `div` 100
        BS.length compressed `shouldSatisfy` (< threshold)

    -- TC-9: determinism
    it "compress is deterministic" $ do
        -- Compressing the same data twice must produce identical bytes.
        let dat = BS.concat (replicate 50 (BC.pack "hello, ZStd world! "))
        compress dat `shouldBe` compress dat

    -- TC-10: repeated pattern
    it "round-trips repeated pattern" $ do
        -- Cycle through "ABCDEF" 3000 times. Strong repetition.
        let dat = BS.pack (take 3000 (cycle (map c2w "ABCDEF")))
        rt dat `shouldBe` dat

    -- TC-11: hello world
    it "round-trips hello world" $ do
        let input = BC.pack "hello world"
        rt input `shouldBe` input

    -- TC-12: all zeros
    it "round-trips all zeros" $ do
        let input = BS.replicate 1000 0
        rt input `shouldBe` input

    -- TC-13: all 0xFF bytes
    it "round-trips all 0xFF bytes" $ do
        let input = BS.replicate 1000 255
        rt input `shouldBe` input

    -- TC-14: manual wire format (raw block)
    it "decodes a manually constructed raw-block frame" $ do
        -- Manually constructed ZStd frame with a raw block containing "hello".
        --
        -- Frame layout:
        --   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
        --   [4]     FHD = 0x20: Single_Segment=1, FCS=1byte
        --   [5]     FCS = 0x05 (content_size = 5)
        --   [6..8]  Block header: Last=1, Type=Raw, Size=5
        --             = (5 << 3) | (0 << 1) | 1 = 41 = 0x29
        --             = [0x29, 0x00, 0x00]
        --   [9..13] b"hello"
        let frame = BS.pack
                [ 0x28, 0xB5, 0x2F, 0xFD  -- magic
                , 0x20                     -- FHD: Single_Segment=1, FCS=1byte
                , 0x05                     -- FCS = 5
                , 0x29, 0x00, 0x00         -- block header: last=1, raw, size=5
                , c2w 'h', c2w 'e', c2w 'l', c2w 'l', c2w 'o'
                ]
        decompress frame `shouldBe` Right (BC.pack "hello")

-- ─── LCG helpers for TC-6 ─────────────────────────────────────────────────────

-- | LCG next state.
lcgStep :: Int -> Int
lcgStep s = s * 1664525 + 1013904223

-- | Extract a byte from an LCG state.
lcgByte :: Int -> Word8
lcgByte s = fromIntegral (s .&. 0xFF)
